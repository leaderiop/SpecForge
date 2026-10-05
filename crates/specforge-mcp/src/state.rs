use specforge_common::{Diagnostic, ProjectConfig};
use specforge_graph::Graph;
use specforge_ops::analyze::ProjectView;
use specforge_project::{CompiledProject, Environment, ProjectSession, SharedRuntime};
use specforge_registry::{RegistryBuild, SurfaceRegistryEntry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::types::{McpEvent, McpPromptDescriptor, McpResourceDescriptor, McpToolDescriptor};

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
    /// Open `subscriptions/listen` streams, by their request id.
    pub listens: Vec<Listen>,
    /// The served project: its environment (config, spec root, registries,
    /// rules, manifests, surfaces), graph, diagnostics and extension
    /// runtime. Detached while no project is served.
    session: ProjectSession,
    /// What registering the served project's surfaces with MCP reported
    /// (auto-promotion conflicts), after the project's own diagnostics.
    pub surface_diagnostics: Vec<Diagnostic>,
    /// The tools MCP auto-promoted from the served project's extension
    /// commands, listed after the project's own surfaces.
    pub promoted_surfaces: Vec<SurfaceRegistryEntry>,
    pub project_root: Option<PathBuf>,
    /// Project compiled when the client's `initialize` names no `projectRoot`
    /// (the `specforge mcp <path>` argument).
    pub default_project_root: Option<PathBuf>,
    pub subscriptions: HashMap<String, Vec<Subscription>>,
    pub previous_diagnostics: Vec<Diagnostic>,
    pub tool_registry: Vec<McpToolDescriptor>,
    pub resource_registry: Vec<McpResourceDescriptor>,
    pub prompt_registry: Vec<McpPromptDescriptor>,
    pub events: Vec<McpEvent>,
    /// Server→client notifications queued for subscribed channels (C9-01),
    /// drained by the host loop via `pending_notifications`.
    pub notification_outbox: Vec<serde_json::Value>,
    /// When the served project was last brought up to date with disk.
    /// Compared against the watch snapshot marker mtime to detect
    /// staleness (C9-07).
    pub loaded_at: Option<SystemTime>,
    /// The Wasm runtime extensions run in, when the host supplies one; by
    /// default the served project's session builds the project's runtime
    /// (`specforge_component::project_runtime`) each time it loads.
    pub extension_runtime: Option<SharedRuntime>,
}

impl McpState {
    /// Path of the watch snapshot marker for this project, if configured.
    pub fn snapshot_marker(&self) -> Option<PathBuf> {
        self.project_root
            .as_ref()
            .map(|root| root.join(".specforge").join("graph.json"))
    }

    /// Reload the served project when watch has written a newer snapshot
    /// (C9-07). No-op without a project root, without a snapshot, or when
    /// fresh.
    pub fn refresh_if_stale(&mut self) {
        let Some(marker) = self.snapshot_marker() else {
            return;
        };
        let Ok(meta) = std::fs::metadata(&marker) else {
            return;
        };
        let mtime = meta.modified().ok();
        let stale = match (self.loaded_at, mtime) {
            (Some(loaded), Some(m)) => m > loaded,
            (None, _) => true, // never compiled against a snapshot
            _ => false,
        };
        if !stale {
            return;
        }
        if let Some(root) = self.project_root.clone() {
            self.reload(&root);
        }
    }
}

/// One `subscriptions/listen` stream (MCP 2026-07-28): the resources it
/// asked to hear about, and the listen request's id every notification on
/// it carries as `io.modelcontextprotocol/subscriptionId`.
#[derive(Debug, Clone, PartialEq)]
pub struct Listen {
    pub id: serde_json::Value,
    pub uris: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub client_id: String,
    pub channel: String,
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
            listens: Vec::new(),
            session: ProjectSession::detached(),
            surface_diagnostics: Vec::new(),
            promoted_surfaces: Vec::new(),
            project_root: None,
            default_project_root: None,
            subscriptions: HashMap::new(),
            previous_diagnostics: Vec::new(),
            tool_registry: Vec::new(),
            resource_registry: Vec::new(),
            prompt_registry: Vec::new(),
            events: Vec::new(),
            notification_outbox: Vec::new(),
            loaded_at: None,
            extension_runtime: None,
        }
    }

    /// The served project's session.
    pub fn session(&self) -> &ProjectSession {
        &self.session
    }

    /// The served project's graph.
    pub fn graph(&self) -> &Graph {
        self.session.graph()
    }

    /// The served project's environment: config, spec root, registries.
    pub fn environment(&self) -> &Environment {
        self.session.environment()
    }

    /// The served project's registries, rules, manifests and surfaces.
    pub fn registries(&self) -> &RegistryBuild {
        &self.environment().registries
    }

    /// The served project's `specforge.json`.
    pub fn config(&self) -> &ProjectConfig {
        &self.environment().config
    }

    /// Where the served project's `.spec` files live: spans are relative
    /// to it. None while no project on disk is served.
    pub fn spec_root(&self) -> Option<&Path> {
        self.session.spec_root()
    }

    /// Everything the server reports for the served project: what
    /// `specforge check` reports, then what registering its surfaces with
    /// MCP reported.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.session.diagnostics();
        diagnostics.extend(self.surface_diagnostics.iter().cloned());
        diagnostics
    }

    /// The surfaces the server serves: the project's, then the tools MCP
    /// auto-promoted from its commands.
    pub fn surface_entries(&self) -> impl Iterator<Item = &SurfaceRegistryEntry> {
        self.registries()
            .surfaces
            .iter()
            .chain(&self.promoted_surfaces)
    }

    /// What an analysis of the served project reads.
    pub fn project_view(&self) -> ProjectView<'_> {
        ProjectView::in_environment(
            self.environment(),
            self.graph(),
            self.project_root.as_deref(),
        )
    }

    /// The runtime extensions of the project at `root` run in: the served
    /// project's session's, which its compile loaded and which serves
    /// every call until the project loads again; the host's; or, for
    /// another project, the project's own, built for the call.
    pub fn wasm_runtime(&self, root: &Path) -> SharedRuntime {
        if !self.serves_other_than(root)
            && let Some(runtime) = self.session.runtime()
        {
            return Arc::clone(runtime);
        }
        self.fresh_runtime(root)
    }

    /// A runtime for `root` built now: the host's, or a new one of the
    /// project's own.
    fn fresh_runtime(&self, root: &Path) -> SharedRuntime {
        match &self.extension_runtime {
            Some(runtime) => Arc::clone(runtime),
            None => Arc::new(specforge_component::project_runtime(root)),
        }
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

    /// Whether tool results carry `structuredContent` (2025-06-18 on).
    pub fn sends_structured_content(&self) -> bool {
        self.revision() >= crate::lifecycle::STRUCTURED_CONTENT_PROTOCOL_VERSION
    }

    /// Record an event. Object payloads without a `timestamp` get one (RFC
    /// 3339, UTC) — except `mcp_initialized`, whose spec payload has none.
    pub fn push_event(&mut self, name: impl Into<String>, mut params: serde_json::Value) {
        let name = name.into();
        if name != "mcp_initialized"
            && let Some(object) = params.as_object_mut()
            && !object.contains_key("timestamp")
        {
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            object.insert("timestamp".into(), serde_json::Value::String(now));
        }
        self.events.push(McpEvent { name, params });
    }

    /// Compile another project at `root` for one call, without serving
    /// it: its extensions run in a runtime built for it, so its modules
    /// are what is on disk now.
    pub fn compile_project(&self, root: &Path) -> CompiledProject {
        let runtime = self.fresh_runtime(root);
        CompiledProject::compile(root, Some(runtime.as_ref()))
    }

    /// Whether `root` names a project other than the one this server
    /// serves. With no project served yet, no path is another's.
    pub fn serves_other_than(&self, root: &Path) -> bool {
        let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        self.project_root
            .as_deref()
            .is_some_and(|served| canonical(served) != canonical(root))
    }

    /// Serve the project at `root` as it is on disk now, its config and
    /// extensions included: the served session reloads its environment
    /// and rebuilds from the sources (a fresh compile), or a session is
    /// opened when `root` is not the project served. The graph,
    /// diagnostics, registries, config and runtime change together, with
    /// the session; the tools and resources listed become the defaults
    /// plus what this project's extensions contribute, so nothing a
    /// previous load contributed survives, and nothing is listed twice.
    /// Subscribed clients learn what changed. The one place the served
    /// project is replaced (initialize, a stale refresh, validate,
    /// analyze, doctor, collect and the mutation tools all come through
    /// here).
    pub fn reload(&mut self, root: &Path) {
        let previous_diagnostics = self.diagnostics();
        let update = if self.serves_session_at(root) {
            self.session.reload_environment()
        } else {
            let next = match &self.extension_runtime {
                Some(runtime) => ProjectSession::open_with_runtime(root, Some(Arc::clone(runtime))),
                None => ProjectSession::open(root),
            };
            let previous = std::mem::replace(&mut self.session, next);
            self.session.replaced(&previous)
        };
        self.project_root = Some(root.to_path_buf());
        self.loaded_at = Some(SystemTime::now());

        self.surface_diagnostics.clear();
        self.promoted_surfaces.clear();
        crate::registry::register_defaults(self);
        let env = self.session.shared_environment();
        crate::registry::register_extension_surfaces(self, &env.registries.manifest_surfaces);
        crate::notifications::enqueue_compile_notifications(
            self,
            &update.delta,
            &previous_diagnostics,
        );
    }

    /// Serve `session`, a project built in memory or opened by the host,
    /// as it is: no surface is registered again and no client notified.
    pub fn serve_session(&mut self, session: ProjectSession) {
        self.session = session;
    }

    /// Serve `graph` with `diagnostics` as its graph build's, in the
    /// served project's environment ([`ProjectSession::from_graph`]).
    pub fn serve_graph(&mut self, graph: Graph, diagnostics: Vec<Diagnostic>) {
        let env = self.session.shared_environment();
        self.session = ProjectSession::from_graph(env, graph, diagnostics);
    }

    /// Serve the served graph as `edit` leaves it, in the same environment
    /// and with the same graph diagnostics, as an in-memory project.
    pub fn edit_graph(&mut self, edit: impl FnOnce(&mut Graph)) {
        let mut graph = self.graph().clone();
        edit(&mut graph);
        let diagnostics = self.session.graph_diagnostics();
        self.serve_graph(graph, diagnostics);
    }

    /// Serve the served graph in the environment `edit` leaves, as an
    /// in-memory project with the same graph diagnostics.
    pub fn edit_environment(&mut self, edit: impl FnOnce(&mut Environment)) {
        let session = std::mem::replace(&mut self.session, ProjectSession::detached());
        let graph = session.graph().clone();
        let diagnostics = session.graph_diagnostics();
        let mut env = session.shared_environment();
        drop(session);
        edit(Arc::get_mut(&mut env).expect("the served environment is shared elsewhere"));
        self.session = ProjectSession::from_graph(env, graph, diagnostics);
    }

    /// Whether the session serves the project on disk at `root`.
    fn serves_session_at(&self, root: &Path) -> bool {
        self.session.origin() == specforge_project::Origin::Disk
            && self.project_root.is_some()
            && !self.serves_other_than(root)
    }

    pub fn shutdown(&mut self) {
        self.phase = ServerPhase::ShuttingDown;
        let clients: std::collections::BTreeSet<String> = self
            .subscriptions
            .values()
            .flatten()
            .map(|s| s.client_id.clone())
            .collect();
        for client in clients {
            crate::subscriptions::unsubscribe_all(self, &client);
        }
        self.subscriptions.clear();
        self.listens.clear();
        // The outbox stays: the host drains it after the shutdown response.
        self.previous_diagnostics = self.diagnostics();
        self.session = ProjectSession::detached();
        self.surface_diagnostics.clear();
        self.promoted_surfaces.clear();
        self.project_root = None;
        self.loaded_at = None;
    }
}
