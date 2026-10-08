//! A long-lived compiled project: what watch, the LSP and MCP hold.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use specforge_common::{Diagnostic, ProjectConfig, codes, read_project_config};
use specforge_graph::{
    FileChange, Graph, GraphBuild, GraphConfig, GraphDelta, compute_graph_delta,
};
use specforge_wasm::WasmRuntime;

use crate::coverage::RecordedCoverage;
use crate::freshness::DiskSnapshot;
use crate::inputs::{Changes, SessionInputs, UpdateKind};
use crate::snapshot::EntitySnapshot;
use crate::sources::{self, Read, SourceCache};
use crate::{Environment, SourceBuild};

/// The runtime a session runs its project's extensions in (every
/// [`WasmRuntime`] is `Send + Sync`).
pub type SharedRuntime = Arc<dyn WasmRuntime>;

/// Builds the runtime of the project at a root from the config read there.
pub type BuildRuntime = Arc<dyn Fn(&Path, &ProjectConfig) -> SharedRuntime + Send + Sync>;

/// How a session gets the runtime its project's extensions run in.
#[derive(Clone)]
pub enum RuntimeSource {
    /// Built for each environment load from the config that load read,
    /// after every environment input is stamped: the project's own
    /// component runtime ([`RuntimeSource::project`]), or a test's.
    Build(BuildRuntime),
    /// The same runtime for every load (none: no extension loads).
    Fixed(Option<SharedRuntime>),
}

impl RuntimeSource {
    /// `specforge_component::project_runtime_with`: the builtins the config
    /// enables, the installed extensions from the lock, the `.wasm` file
    /// entries.
    pub fn project() -> Self {
        RuntimeSource::Build(Arc::new(|root, config| {
            Arc::new(specforge_component::project_runtime_with(root, config))
        }))
    }

    /// The runtime of the environment load at `root` with `config`.
    fn runtime_for(&self, root: &Path, config: &ProjectConfig) -> Option<SharedRuntime> {
        match self {
            RuntimeSource::Build(build) => Some(build(root, config)),
            RuntimeSource::Fixed(runtime) => runtime.clone(),
        }
    }
}

/// What changed in a session's sources.
pub enum SourceChange<'a> {
    /// Files changed, created or deleted on disk, by path relative to the
    /// spec root. A file that is gone was deleted; one that is there and
    /// cannot be read is E025 and leaves the graph.
    Disk(&'a [String]),
    /// An editor buffer is the truth for one file (`None`: it is gone).
    Buffer {
        path: &'a str,
        text: Option<&'a str>,
    },
    /// Several editor buffers, each the truth for its file (relative to the
    /// spec root), applied as one update with one run of the checks: an edit
    /// the editor applied to several files at once, or every open buffer
    /// again after a reload. Files discovery would not find are ignored.
    Buffers(&'a [(String, String)]),
}

/// Which checks an update runs on the updated graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMode<'a> {
    /// Every check `specforge check` runs.
    Full,
    /// The editor's fast path while typing: when any of these files
    /// (relative to the spec root) has parse errors the graph is broken and
    /// the checks would evaluate garbage, so they are skipped and only the
    /// parse layer is reported until they parse again. Otherwise,
    /// [`Self::Full`].
    SyntaxOnlyIfParseErrorsIn(&'a [&'a str]),
}

/// What one update of a session did.
#[derive(Debug)]
pub struct Update {
    /// What was applied: sources, the checks alone, or the environment.
    pub kind: UpdateKind,
    /// The session's inputs changed: a file the checks read was named or
    /// dropped, or the environment loaded again with other inputs. What an
    /// adapter watches must follow [`ProjectSession::inputs`] (ADR 0030).
    pub inputs_changed: bool,
    pub delta: GraphDelta,
    /// The files re-parsed or dropped: exactly the changed ones (sorted).
    pub rebuilt_files: Vec<String>,
    /// Files whose graph-build diagnostics changed (sorted).
    pub changed_diagnostic_files: Vec<String>,
    /// Everything the session reports now: [`ProjectSession::diagnostics`].
    pub diagnostics: Vec<Diagnostic>,
    /// The comparison of the incremental graph with a cold rebuild, when
    /// the session verifies its updates.
    pub verification: Option<Result<(), String>>,
}

/// A compiled project that accepts source changes and environment reloads.
///
/// It is seeded by one cold build, then kept current incrementally: only
/// the changed files are re-read and re-parsed and the graph build applies
/// them whole ([`specforge_graph::GraphBuild`]), the imports of every parse are
/// resolved again (E025, I004, W113, W027), and the graph checks re-run on
/// the patched graph. After any sequence of updates,
/// [`ProjectSession::diagnostics`] is the set
/// [`crate::CompiledProject::diagnostics`] reports for the same sources.
pub struct ProjectSession {
    /// Shared, so a reader can keep the environment an update started
    /// from while the session itself is busy.
    env: Arc<Environment>,
    runtime: Option<SharedRuntime>,
    /// Where each environment load gets its runtime: a built one is fresh
    /// for every load (the extensions' `.wasm` files may have changed).
    source: RuntimeSource,
    /// The text of every source as read, the retained trees and the
    /// sources that could not be read.
    sources: SourceCache,
    /// The graph of the sources and what building it reported (ADR 0032).
    graph: GraphBuild,
    import_diagnostics: Vec<Diagnostic>,
    check_diagnostics: Vec<Diagnostic>,
    verify_incremental: bool,
    /// What the session depends on (ADR 0030). Detached (an editor with no
    /// workspace folder), files are buffers keyed by absolute path, with no
    /// spec root to resolve their imports against and no environment to
    /// reload.
    inputs: SessionInputs,
    /// What the session last built from, as it was when read: what
    /// [`Self::stale`] compares with disk.
    snapshot: DiskSnapshot,
    /// The current graph's entity snapshot, the recorded test report and
    /// the coverage of the graph against it: a fresh memo after every update
    /// and reload. A check seeds it with the snapshot it read; after an
    /// update that skipped the checks it is made over the graph on first use
    /// ([`Self::recorded`], the one place).
    recorded: OnceLock<RecordedCoverage>,
}

/// A project whose environment is loaded and whose sources are not read yet
/// ([`ProjectSession::begin_open`]). Its environment is shared, so a reader
/// can serve what needs only it (an editor's keyword completion) while
/// [`Self::finish`] reads and builds the sources.
pub struct OpeningProject {
    env: Arc<Environment>,
    runtime: Option<SharedRuntime>,
    /// Where each environment load gets its runtime.
    source: RuntimeSource,
    /// What the loaded environment depends on.
    inputs: SessionInputs,
    /// Stamped for the environment; the sources are stamped by `finish`.
    snapshot: DiskSnapshot,
}

impl OpeningProject {
    /// The loaded environment: what the finished session will have, until
    /// the environment is loaded again.
    pub fn environment(&self) -> &Arc<Environment> {
        &self.env
    }

    /// Read every source, build the graph and run the checks: the session
    /// [`ProjectSession::open`] returns.
    pub fn finish(self) -> ProjectSession {
        let mut snapshot = self.snapshot;
        let discovered = self.inputs.discover();
        snapshot.stamp_all_sources(&self.inputs, &discovered);
        // What was stamped is exactly what is read.
        let SourceBuild {
            sources,
            mut graph,
            imports,
        } = self.env.build_sources(&discovered);
        graph.set_verify(cfg!(debug_assertions));

        let mut session = ProjectSession {
            env: self.env,
            runtime: self.runtime,
            source: self.source,
            sources,
            graph,
            import_diagnostics: imports,
            check_diagnostics: Vec::new(),
            verify_incremental: cfg!(debug_assertions),
            inputs: self.inputs,
            snapshot,
            recorded: OnceLock::new(),
        };
        session.check_diagnostics = session.checked().0;
        session
    }
}

impl ProjectSession {
    /// A session with no project: no config, no extension and no file
    /// until a buffer is added.
    pub fn detached() -> Self {
        ProjectSession {
            env: Arc::new(Environment::empty()),
            runtime: None,
            source: RuntimeSource::Fixed(None),
            sources: SourceCache::empty(),
            graph: {
                let mut graph = GraphBuild::new(GraphConfig::default());
                graph.set_verify(cfg!(debug_assertions));
                graph
            },
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            verify_incremental: cfg!(debug_assertions),
            inputs: SessionInputs::detached(),
            snapshot: DiskSnapshot::default(),
            recorded: OnceLock::new(),
        }
    }

    /// Open the project at `root`, running its extensions in the project's
    /// own runtime ([`RuntimeSource::project`]).
    pub fn open(root: &Path) -> Self {
        Self::open_from(root, RuntimeSource::project())
    }

    /// Open the project at `root` with `runtime` (none: no extension
    /// loads). A reload keeps using the same runtime.
    pub fn open_with_runtime(root: &Path, runtime: Option<SharedRuntime>) -> Self {
        Self::open_from(root, RuntimeSource::Fixed(runtime))
    }

    /// Open the project at `root`, each environment load getting its
    /// runtime from `source`.
    pub fn open_from(root: &Path, source: RuntimeSource) -> Self {
        Self::begin_open_from(root, source).finish()
    }

    /// The first half of [`Self::open`]: the environment loaded (config,
    /// extensions, registries), no `.spec` file read. What needs only the
    /// environment (the kinds and fields a keyword completion offers) is
    /// served from [`OpeningProject::environment`] while
    /// [`OpeningProject::finish`] reads and builds the sources.
    pub fn begin_open(root: &Path) -> OpeningProject {
        Self::begin_open_from(root, RuntimeSource::project())
    }

    /// [`Self::begin_open`] with the runtime `source` gives.
    pub fn begin_open_from(root: &Path, source: RuntimeSource) -> OpeningProject {
        // Everything is stamped before anything reads it (crate::freshness):
        // the config before its one read, the lock and the modules before
        // the runtime and the environment read them, the sources by `finish`.
        let mut snapshot = DiskSnapshot::default();
        snapshot.stamp_config(&root.join("specforge.json"));
        let read = read_project_config(root);
        let inputs = SessionInputs::opened(root, &read.config);
        snapshot.stamp_environment(&inputs);
        let runtime = source.runtime_for(root, &read.config);
        let env = Environment::from_read(root, read, runtime.as_deref());
        let inputs = inputs.with_check_passes(env.registries.check_passes().next().is_some());
        OpeningProject {
            env: Arc::new(env),
            runtime,
            source,
            inputs,
            snapshot,
        }
    }

    /// Compare every update's graph with a cold rebuild (costly). A debug
    /// build verifies every session by default; this turns it on in release
    /// (watch's `--verify-incremental`) or off.
    pub fn set_verify_incremental(&mut self, enabled: bool) {
        self.verify_incremental = enabled;
        self.graph.set_verify(enabled);
    }

    /// Apply a change to the project's sources, then run every check.
    /// Files discovery would not find (an `exclude` entry, a skipped
    /// directory, not `.spec`) are ignored.
    pub fn update(&mut self, change: SourceChange<'_>) -> Update {
        self.update_with(change, CheckMode::Full)
    }

    /// [`Self::update`], running the checks `mode` asks for.
    pub fn update_with(&mut self, change: SourceChange<'_>, mode: CheckMode<'_>) -> Update {
        let changes: Vec<FileChange> = match change {
            SourceChange::Disk(paths) => {
                let keys: Vec<String> = paths
                    .iter()
                    .filter(|path| !self.excludes(path))
                    .cloned()
                    .collect();
                // Stamped before they are read again (crate::freshness).
                self.snapshot.stamp_sources(&self.inputs, &keys);
                keys.iter()
                    .map(|path| {
                        self.sources
                            .change(path, sources::read(&self.env.spec_root, path))
                    })
                    .collect()
            }
            SourceChange::Buffer { path, .. } if self.excludes(path) => Vec::new(),
            SourceChange::Buffer { path, text } => vec![self.sources.change(
                path,
                text.map_or(Read::Gone, |text| Read::Text(text.to_string())),
            )],
            SourceChange::Buffers(buffers) => {
                let held: Vec<&(String, String)> = buffers
                    .iter()
                    .filter(|(path, _)| !self.excludes(path))
                    .collect();
                held.into_iter()
                    .map(|(path, text)| self.sources.change(path, Read::Text(text.clone())))
                    .collect()
            }
        };
        let applied = self.graph.apply(changes);
        self.recorded = OnceLock::new();
        self.import_diagnostics = self.resolve_imports();
        let (check_diagnostics, inputs_changed) = match mode {
            CheckMode::SyntaxOnlyIfParseErrorsIn(paths)
                if paths.iter().any(|path| {
                    self.graph
                        .file_diagnostics(path)
                        .iter()
                        .any(|d| d.is(codes::E001))
                }) =>
            {
                (Vec::new(), false)
            }
            _ => self.checked(),
        };
        self.check_diagnostics = check_diagnostics;
        Update {
            kind: UpdateKind::Sources,
            inputs_changed,
            delta: applied.delta,
            rebuilt_files: applied.files,
            changed_diagnostic_files: applied.changed_diagnostic_files,
            diagnostics: self.diagnostics(),
            verification: applied.verification,
        }
    }

    /// `specforge.json` or an extension changed: load the environment again
    /// and rebuild from the sources on disk.
    pub fn reload_environment(&mut self) -> Update {
        if self.inputs.root().is_none() {
            // Nothing on disk to load again.
            return Update {
                kind: UpdateKind::Environment,
                inputs_changed: false,
                delta: GraphDelta::default(),
                rebuilt_files: Vec::new(),
                changed_diagnostic_files: Vec::new(),
                diagnostics: self.diagnostics(),
                verification: None,
            };
        }
        let root = self.env.root.clone();
        let mut next = Self::open_from(&root, self.source.clone());
        next.set_verify_incremental(self.verify_incremental);
        let previous = std::mem::replace(self, next);
        self.replaced(&previous)
    }

    /// The update that replacing `previous` with this session amounts to:
    /// every file rebuilt, the delta between the two graphs.
    pub fn replaced(&self, previous: &ProjectSession) -> Update {
        Update {
            kind: UpdateKind::Environment,
            inputs_changed: previous.inputs != self.inputs,
            delta: compute_graph_delta(previous.graph(), self.graph()),
            rebuilt_files: self
                .graph
                .files()
                .map(|(path, _)| path.to_string())
                .collect(),
            changed_diagnostic_files: self.graph.diagnostic_files(),
            diagnostics: self.diagnostics(),
            verification: None,
        }
    }

    /// Everything the project reports now, the set `specforge check`
    /// reports: the environment's, the imports', the graph build's, the
    /// graph checks', then surface conflicts.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.env
            .diagnostics()
            .chain(&self.import_diagnostics)
            .cloned()
            .chain(self.graph.diagnostics().iter().cloned())
            .chain(self.check_diagnostics.iter().cloned())
            .chain(self.env.surface_diagnostics().iter().cloned())
            .collect()
    }

    pub fn graph(&self) -> &Graph {
        self.graph.graph()
    }

    /// The text the session's current build parsed `path` (relative to the
    /// spec root) from: the file as it was read, or the buffer as it was
    /// given, at the last update that touched it. A span of the graph or of
    /// a diagnostic is a position in this text, not in the file on disk or
    /// the buffer now. `None` for a file the session does not hold.
    pub fn source_text(&self, path: &str) -> Option<Arc<str>> {
        self.sources.text(path).cloned()
    }

    /// [`Self::source_text`] of every file, shared rather than copied: what
    /// a reader keeps while the session is out for an update.
    pub fn source_texts(&self) -> HashMap<String, Arc<str>> {
        self.sources.texts()
    }

    /// What the session depends on now (ADR 0030): what a changed path is,
    /// which directories to watch, which files to report.
    pub fn inputs(&self) -> &SessionInputs {
        &self.inputs
    }

    /// The project root: `None` with no project (detached).
    pub fn root(&self) -> Option<&Path> {
        self.inputs.root()
    }

    /// Where `.spec` files are keyed from: `None` unless the project was
    /// opened from disk.
    pub fn spec_root(&self) -> Option<&Path> {
        self.inputs.spec_root()
    }

    /// A `.spec` path's key in this session: relative to the spec root
    /// when the file is under it, else the path itself.
    pub fn source_key(&self, path: &Path) -> String {
        self.env.source_key(path)
    }

    /// Apply `changes`: the environment first (a reload rebuilds
    /// everything), else the sources (an update re-runs every check), else
    /// the checks alone. `None` when there is nothing to apply: `changes` is
    /// empty.
    pub fn apply(&mut self, changes: &Changes) -> Option<Update> {
        if changes.environment && self.inputs.root().is_some() {
            Some(self.reload_environment())
        } else if !changes.sources.is_empty() {
            Some(self.update(SourceChange::Disk(&changes.sources)))
        } else if changes.check_inputs && self.inputs.root().is_some() {
            Some(self.recheck())
        } else {
            None
        }
    }

    /// What changed on disk since the session last built (behavior
    /// `bring_session_up_to_date`): the sources discovery finds now against
    /// those it read, and every environment and check input against what
    /// it read. A session that was not opened from disk reports nothing.
    pub fn stale(&self) -> Changes {
        self.snapshot.changes(&self.inputs)
    }

    /// Bring the session up to date with disk, without a watcher: apply
    /// exactly what [`Self::stale`] finds. Afterwards its graph and
    /// diagnostics are what a fresh compile of the files on disk gives.
    /// `None` when nothing changed.
    pub fn ensure_fresh(&mut self) -> Option<Update> {
        let changes = self.stale();
        self.apply(&changes)
    }

    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// The recorded test report and the coverage of the current graph
    /// against it, memoized until the next update or reload.
    pub fn recorded(&self) -> &RecordedCoverage {
        self.recorded
            .get_or_init(|| RecordedCoverage::over(self.graph.graph(), &self.env))
    }

    /// The current graph's entity snapshot (ADR 0019): the one its last
    /// check read, or, when the last update skipped the checks, one taken
    /// on first use.
    pub fn entities(&self) -> &EntitySnapshot {
        self.recorded().entities()
    }

    /// The environment, shared: it stays valid after the session reloads.
    pub fn shared_environment(&self) -> Arc<Environment> {
        Arc::clone(&self.env)
    }

    /// The runtime the project's extensions run in (none: no extension
    /// loads).
    pub fn runtime(&self) -> Option<&SharedRuntime> {
        self.runtime.as_ref()
    }

    /// The graph build's diagnostics: duplicates, unresolved references,
    /// reference cycles, parse errors (no import's, no check's).
    pub fn graph_diagnostics(&self) -> Vec<Diagnostic> {
        self.graph.diagnostics().to_vec()
    }

    /// The graph build's diagnostics in one file (relative to the spec root).
    pub fn file_diagnostics(&self, path: &str) -> &[Diagnostic] {
        self.graph.file_diagnostics(path)
    }

    /// Every file with graph-build diagnostics (sorted).
    pub fn diagnostic_files(&self) -> Vec<String> {
        self.graph.diagnostic_files()
    }

    /// How many `.spec` files the project has.
    pub fn file_count(&self) -> usize {
        self.graph.files().len()
    }

    /// Whether a changed file is outside the project. A session with no
    /// project takes every buffer: it has no spec root to discover files
    /// under.
    fn excludes(&self, path: &str) -> bool {
        self.inputs.excludes(path)
    }

    /// Run every check again on the current graph: a check input changed.
    fn recheck(&mut self) -> Update {
        self.recorded = OnceLock::new();
        let (check_diagnostics, inputs_changed) = self.checked();
        self.check_diagnostics = check_diagnostics;
        Update {
            kind: UpdateKind::Checks,
            inputs_changed,
            delta: GraphDelta::default(),
            rebuilt_files: Vec::new(),
            changed_diagnostic_files: Vec::new(),
            diagnostics: self.diagnostics(),
            verification: None,
        }
    }

    fn resolve_imports(&self) -> Vec<Diagnostic> {
        if self.inputs.root().is_none() {
            return Vec::new();
        }
        self.env.import_diagnostics(&self.sources, &self.graph)
    }

    /// A snapshot of the current graph, taken now (ADR 0019).
    fn snapshot_now(&self) -> Arc<EntitySnapshot> {
        Arc::new(self.env.entity_snapshot(self.graph.graph()))
    }

    /// Run every check on the current graph over `entities`, its snapshot:
    /// the coverage memo starts again from that snapshot.
    fn check_over(&mut self, entities: Arc<EntitySnapshot>) -> Vec<Diagnostic> {
        let diagnostics =
            self.env
                .run_checks(self.graph.graph(), &entities, self.runtime.as_deref());
        self.recorded = OnceLock::from(RecordedCoverage::of(entities));
        diagnostics
    }

    /// Every check on the current graph, over a snapshot of it taken now,
    /// the check inputs (which the snapshot's `file_exists` rules add to)
    /// stamped first; also whether the session's inputs changed.
    fn checked(&mut self) -> (Vec<Diagnostic>, bool) {
        let entities = self.snapshot_now();
        let mut changed = false;
        if self.inputs.root().is_some() {
            let next = self
                .inputs
                .with_named(self.env.registries.files(&entities.rule_input()));
            changed = next != self.inputs;
            self.inputs = next;
            self.snapshot.stamp_checks(&self.inputs);
        }
        (self.check_over(entities), changed)
    }
}
