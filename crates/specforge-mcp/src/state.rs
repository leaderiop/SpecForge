use specforge_common::{Diagnostic, ProjectConfig};
use specforge_graph::Graph;
use specforge_project::{ProjectSession, SharedRuntime, Update, UpdateKind};
use specforge_registry::RegistryBuild;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::subscriptions::{Changes, Subscriptions};
use crate::surface_table::ExtensionSurfaceTable;
use crate::types::McpEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPhase {
    Uninitialized,
    Initialized,
    ShuttingDown,
}

pub struct McpState {
    pub phase: ServerPhase,
    /// The protocol revision `initialize` negotiated (the latest one the
    /// server speaks until then).
    pub protocol_version: &'static str,
    /// The revision the request being handled names in its `_meta`: set
    /// for the length of a stateless (2026-07-28) request, which is served
    /// under its own revision whatever `initialize` negotiated.
    pub request_revision: Option<&'static str>,
    /// Whether a project (or the empty default surface) is being served:
    /// set by `initialize`, or by the first stateless request.
    pub served: bool,
    /// The served project: its root, environment (config, spec root,
    /// registries, rules, manifests, surfaces), graph, diagnostics and
    /// extension runtime: always opened from disk (ADR 0025). Detached
    /// while none is served.
    session: ProjectSession,
    /// How many times the served project changed: every update applied to
    /// it and every replacement bumps it ([`Self::session_generation`]).
    generation: u64,
    /// The last update [`Self::ensure_fresh`] applied.
    last_update: Option<Update>,
    /// What MCP serves from the served project's extensions, built from
    /// their declarations whenever its environment loads.
    surfaces: ExtensionSurfaceTable,
    /// Project compiled when the client's `initialize` names no `projectRoot`
    /// (the `specforge mcp <path>` argument).
    pub default_project_root: Option<PathBuf>,
    pub events: Vec<McpEvent>,
    /// Who hears about what (ADR 0024 D6). Read through
    /// [`Self::subscriptions`]; changed only by the subscription requests,
    /// an update of the served project, cancellation, disconnect and
    /// shutdown.
    pub(crate) subscriptions: Subscriptions,
    /// The Wasm runtime extensions run in, when the host supplies one; by
    /// default the served project's session builds the project's runtime
    /// (`specforge_component::ComponentRuntime::with_user_cache`) each time it loads.
    pub extension_runtime: Option<SharedRuntime>,
}

impl Default for McpState {
    fn default() -> Self {
        Self::new()
    }
}

impl McpState {
    pub fn new() -> Self {
        Self {
            phase: ServerPhase::Uninitialized,
            protocol_version: crate::lifecycle::LATEST_PROTOCOL_VERSION,
            request_revision: None,
            served: false,
            session: ProjectSession::detached(),
            generation: 0,
            last_update: None,
            surfaces: ExtensionSurfaceTable::empty(),
            default_project_root: None,
            events: Vec::new(),
            subscriptions: Subscriptions::new(),
            extension_runtime: None,
        }
    }

    /// Who hears about what.
    pub fn subscriptions(&self) -> &Subscriptions {
        &self.subscriptions
    }

    /// The served project's session.
    pub fn session(&self) -> &ProjectSession {
        &self.session
    }

    /// The served project's root: `None` while no project is served.
    pub fn project_root(&self) -> Option<&Path> {
        self.session.root()
    }

    /// How many times the served project changed since the server started:
    /// every update applied to it and every replacement counts once. A call
    /// that found nothing changed on disk leaves it as it was.
    pub fn session_generation(&self) -> u64 {
        self.generation
    }

    /// The served project's graph.
    pub fn graph(&self) -> &Graph {
        self.session.graph()
    }

    /// The served project's registries, rules, declarations and surfaces.
    /// What a call reads goes through its target's view
    /// ([`crate::target::Call::view`]); this is the served session's, for
    /// the server itself (the surface table, the lifecycle answers).
    pub fn registries(&self) -> &RegistryBuild {
        &self.session.environment().registries
    }

    /// The served project's `specforge.json`.
    pub fn config(&self) -> &ProjectConfig {
        &self.session.environment().config
    }

    /// Where the served project's `.spec` files live: spans are relative
    /// to it. None while no project on disk is served.
    pub fn spec_root(&self) -> Option<&Path> {
        self.session.spec_root()
    }

    /// Everything the server reports for the served project: what
    /// `specforge check` reports, then the contributions of its extensions
    /// MCP does not serve under their names (I017).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.session.diagnostics();
        diagnostics.extend(self.surfaces.diagnostics().iter().cloned());
        diagnostics
    }

    /// What MCP serves from the served project's extensions: listed after
    /// the core tools and resources, and looked up by a call that names no
    /// core one.
    pub fn surfaces(&self) -> &ExtensionSurfaceTable {
        &self.surfaces
    }

    /// Whether requests are served: after `initialize`, or for a stateless
    /// request, which needs no handshake.
    pub fn is_initialized(&self) -> bool {
        self.phase == ServerPhase::Initialized
            || (self.request_revision.is_some() && self.phase != ServerPhase::ShuttingDown)
    }

    /// The revision the current request is served under: its own, for a
    /// stateless request, else the one `initialize` negotiated.
    pub fn revision(&self) -> &'static str {
        self.request_revision.unwrap_or(self.protocol_version)
    }

    /// Whether the session accepts JSON-RPC batches: only a 2025-03-26
    /// session does, since later revisions removed batching.
    pub fn accepts_batches(&self) -> bool {
        self.phase == ServerPhase::Initialized
            && self.protocol_version == crate::lifecycle::BATCHING_PROTOCOL_VERSION
    }

    /// The JSON-RPC code of a resource that does not exist, in the revision
    /// the request in hand speaks: -32002 in a handshake session (MCP
    /// 2025-03-26 to 2025-11-25, server/resources), -32602 in a 2026-07-28
    /// request, which says "Invalid Params" and asks clients to accept
    /// -32002 as earlier revisions used it.
    pub fn resource_not_found_code(&self) -> i64 {
        if crate::lifecycle::MODERN_PROTOCOL_VERSIONS.contains(&self.revision()) {
            crate::protocol::error_codes::INVALID_PARAMS
        } else {
            crate::protocol::error_codes::RESOURCE_NOT_FOUND
        }
    }

    /// Whether tool results carry `structuredContent` (2025-06-18 on).
    pub fn sends_structured_content(&self) -> bool {
        self.revision() >= crate::lifecycle::STRUCTURED_CONTENT_PROTOCOL_VERSION
    }

    /// Record an event. Object payloads without a `timestamp` get one (RFC
    /// 3339, UTC) — except `mcp_initialized`, whose spec payload has none.
    pub fn push_event(&mut self, name: impl Into<String>, params: serde_json::Value) {
        self.events.push(McpEvent::new(name, params));
    }

    /// Serve the project at `root` as it is on disk now, its config and
    /// extensions included: the served session reloads its environment
    /// and rebuilds from the sources (a fresh compile), or a session is
    /// opened when `root` is not the project served. The graph,
    /// diagnostics, registries, config and runtime change together, with
    /// the session; the extension surface table is built again from this
    /// project's declarations, so nothing a previous load contributed
    /// survives, and nothing is listed twice.
    /// Subscribed clients learn what changed. The one place the served
    /// project is replaced (initialize, adopting a call's path, the
    /// directory `init` created).
    pub fn serve(&mut self, root: &Path) {
        let before = self.heard_diagnostics();
        // The served session reloads when it is the project on disk at
        // `root`; any other root is opened.
        let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let reloads = self
            .project_root()
            .is_some_and(|served| canonical(served) == canonical(root));
        let update = if reloads {
            self.session.reload_environment()
        } else {
            let next = match &self.extension_runtime {
                Some(runtime) => ProjectSession::open_with_runtime(root, Some(Arc::clone(runtime))),
                None => ProjectSession::open(root),
            };
            let previous = std::mem::replace(&mut self.session, next);
            self.session.replaced(&previous)
        };
        self.applied(update, before);
    }

    /// Bring the served project up to date with disk (behavior
    /// `bring_session_up_to_date`): exactly what changed since it was last
    /// built is applied, an update for changed sources, an environment
    /// reload (its extension tools and resources registered again) for a
    /// changed `specforge.json`, `specforge.lock` or extension module, a
    /// re-check for a changed check input. Subscribed clients learn what
    /// changed. `None` when nothing did.
    pub fn ensure_fresh(&mut self) -> Option<&Update> {
        let before = self.heard_diagnostics();
        let update = self.session.ensure_fresh()?;
        self.applied(update, before);
        self.last_update.as_ref()
    }

    /// What the server reports now, when someone hears about the
    /// diagnostics: an update compares it with what it reports after.
    fn heard_diagnostics(&self) -> Option<Vec<Diagnostic>> {
        self.subscriptions
            .hears_diagnostics()
            .then(|| self.diagnostics())
    }

    /// Record `update`, applied to the served session: the extension
    /// surface table is built again when its environment loaded again, and
    /// subscribed clients learn what changed.
    fn applied(&mut self, update: Update, before: Option<Vec<Diagnostic>>) {
        // Every session verifies its updates in a debug build (ADR 0035):
        // a divergence from a cold rebuild is a bug, and this is the one
        // place every update of the served project passes.
        if let Some(divergence) = update.divergence() {
            debug_assert!(
                false,
                "an update of the served project diverged from a cold rebuild: {divergence}"
            );
        }
        self.generation += 1;
        if update.kind == UpdateKind::Environment {
            self.surfaces = ExtensionSurfaceTable::build(
                self.registries(),
                crate::tools::CORE_TOOLS,
                crate::resources::CORE_RESOURCES,
            );
            let stats = self.surfaces.stats();
            if stats.declared > 0 {
                self.push_event(
                    "commands_auto_promoted",
                    serde_json::json!({
                        "promotedCount": stats.promoted,
                        "conflictCount": stats.conflicts,
                    }),
                );
            }
        }
        // Subscribers hear what changed (ADR 0024 D6), here because every
        // update of the served project passes here (ADR 0035 D3).
        let after = before.as_ref().map(|_| self.diagnostics());
        let diagnostics = before.as_deref().zip(after.as_deref());
        self.subscriptions
            .updated(&Changes::of(&update, diagnostics), &mut self.events);
        self.last_update = Some(update);
    }

    /// Every subscription and stream ends, each recorded; the session
    /// detaches. How many subscriptions ended.
    pub fn shutdown(&mut self) -> usize {
        self.phase = ServerPhase::ShuttingDown;
        // The queue stays: the host sends it after the shutdown response.
        let released = self.subscriptions.disconnect(&mut self.events);
        self.session = ProjectSession::detached();
        self.generation += 1;
        self.surfaces = ExtensionSurfaceTable::empty();
        released
    }
}
