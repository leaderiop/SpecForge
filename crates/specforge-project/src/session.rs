//! A long-lived compiled project: what watch, the LSP and MCP hold.

use std::path::Path;
use std::sync::Arc;

use specforge_common::{Diagnostic, ProjectConfig, read_project_config};
use specforge_graph::{Applied, GraphDelta};
use specforge_wasm::WasmRuntime;

use crate::Environment;
use crate::compiled::CompiledProject;
use crate::freshness::DiskSnapshot;
use crate::inputs::{Changes, SessionInputs, UpdateKind};
use crate::sources::Read;

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
    /// The project's own component runtime, with the per-user compile
    /// cache ([`specforge_component::ComponentRuntime::with_user_cache`]):
    /// empty, for the environment's extension load to fill.
    pub fn project() -> Self {
        RuntimeSource::Build(Arc::new(|_, _| {
            Arc::new(specforge_component::ComponentRuntime::with_user_cache())
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

impl Update {
    /// The update of `kind` that applied `applied` (a graph build's apply,
    /// [`CompiledProject::replacing`], or nothing) and now reports
    /// `diagnostics`: the one place an update is assembled.
    fn of(
        kind: UpdateKind,
        inputs_changed: bool,
        applied: Applied,
        diagnostics: Vec<Diagnostic>,
    ) -> Update {
        let Applied {
            files,
            delta,
            changed_diagnostic_files,
            verification,
        } = applied;
        Update {
            kind,
            inputs_changed,
            delta,
            rebuilt_files: files,
            changed_diagnostic_files,
            diagnostics,
            verification,
        }
    }

    /// How the incremental graph differs from a cold rebuild, when this
    /// update was verified and the two differ: what every surface reports
    /// where it reports (watch in its event, the LSP in its log, MCP as a
    /// debug assertion; ADR 0035).
    pub fn divergence(&self) -> Option<&str> {
        match &self.verification {
            Some(Err(divergence)) => Some(divergence),
            Some(Ok(())) | None => None,
        }
    }
}

/// A compiled project kept current, and what it is built from: its inputs
/// (ADR 0030), the stamps of what it last read, and where each environment
/// load gets its extension runtime. It is seeded by one cold build, then
/// kept current incrementally (ADR 0006, 0032): only the changed files are
/// re-read and re-parsed and the graph build applies them whole
/// ([`specforge_graph::GraphBuild`]), the imports of every parse are
/// resolved again (E025, I004, W113, W027), and the graph checks re-run on
/// the patched graph. After every update that runs the checks its compiled
/// project reports what a fresh compile of the same sources reports, in the
/// same order (ADR 0047).
pub struct ProjectSession {
    /// What it compiled, kept current.
    project: CompiledProject,
    /// The runtime the project's extensions run in (none: no extension
    /// loads).
    runtime: Option<SharedRuntime>,
    /// Where each environment load gets its runtime: a built one is fresh
    /// for every load (the extensions' `.wasm` files may have changed).
    source: RuntimeSource,
    /// What the session depends on (ADR 0030). Detached (an editor with no
    /// workspace folder), files are buffers keyed by absolute path, with no
    /// spec root to resolve their imports against and no environment to
    /// reload.
    inputs: SessionInputs,
    /// What the session last built from, as it was when read: what
    /// [`Self::stale`] compares with disk.
    snapshot: DiskSnapshot,
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
        let mut project = CompiledProject::read(self.env, &discovered);
        project.set_verify(cfg!(debug_assertions));
        let mut session = ProjectSession {
            project,
            runtime: self.runtime,
            source: self.source,
            inputs: self.inputs,
            snapshot,
        };
        session.check();
        session
    }
}

impl ProjectSession {
    /// A session with no project: no config, no extension and no file
    /// until a buffer is added.
    pub fn detached() -> Self {
        ProjectSession {
            project: CompiledProject::detached(),
            runtime: None,
            source: RuntimeSource::Fixed(None),
            inputs: SessionInputs::detached(),
            snapshot: DiskSnapshot::default(),
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
        Self::begin_open(root, source).finish()
    }

    /// The first half of [`Self::open_from`]: the environment loaded
    /// (config, extensions, registries; each load getting its runtime from
    /// `source`), no `.spec` file read. What needs only the environment (the
    /// kinds and fields a keyword completion offers) is served from
    /// [`OpeningProject::environment`] while [`OpeningProject::finish`]
    /// reads and builds the sources.
    pub fn begin_open(root: &Path, source: RuntimeSource) -> OpeningProject {
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
        self.project.set_verify(enabled);
    }

    /// Apply a change to the project's sources, then run every check.
    /// Files discovery would not find (an `exclude` entry, a skipped
    /// directory, not `.spec`) are ignored.
    pub fn update(&mut self, change: SourceChange<'_>) -> Update {
        self.update_with(change, CheckMode::Full)
    }

    /// [`Self::update`], running the checks `mode` asks for.
    pub fn update_with(&mut self, change: SourceChange<'_>, mode: CheckMode<'_>) -> Update {
        let reads: Vec<(String, Read)> = match change {
            SourceChange::Disk(paths) => {
                let keys: Vec<String> = paths
                    .iter()
                    .filter(|path| !self.excludes(path))
                    .cloned()
                    .collect();
                // Stamped before they are read again (crate::freshness).
                self.snapshot.stamp_sources(&self.inputs, &keys);
                keys.into_iter()
                    .map(|key| {
                        let read = self.project.read_source(&key);
                        (key, read)
                    })
                    .collect()
            }
            SourceChange::Buffer { path, .. } if self.excludes(path) => Vec::new(),
            SourceChange::Buffer { path, text } => vec![(
                path.to_string(),
                text.map_or(Read::Gone, |text| Read::Text(text.to_string())),
            )],
            SourceChange::Buffers(buffers) => buffers
                .iter()
                .filter(|(path, _)| !self.excludes(path))
                .map(|(path, text)| (path.clone(), Read::Text(text.clone())))
                .collect(),
        };
        let applied = self.project.apply(reads);
        let inputs_changed = match mode {
            CheckMode::SyntaxOnlyIfParseErrorsIn(paths)
                if self.project.has_parse_errors_in(paths) =>
            {
                self.project.skip_checks();
                false
            }
            _ => self.check(),
        };
        Update::of(
            UpdateKind::Sources,
            inputs_changed,
            applied,
            self.project.diagnostics(),
        )
    }

    /// `specforge.json` or an extension changed: load the environment again
    /// and rebuild from the sources on disk.
    pub fn reload_environment(&mut self) -> Update {
        if self.inputs.root().is_none() {
            // Nothing on disk to load again.
            return Update::of(
                UpdateKind::Environment,
                false,
                Applied::default(),
                self.project.diagnostics(),
            );
        }
        let root = self.project.environment().root.clone();
        let mut next = Self::open_from(&root, self.source.clone());
        next.project.set_verify(self.project.verifies());
        let previous = std::mem::replace(self, next);
        self.replaced(&previous)
    }

    /// The update that replacing `previous` with this session amounts to:
    /// every file rebuilt, the delta between the two graphs.
    pub fn replaced(&self, previous: &ProjectSession) -> Update {
        Update::of(
            UpdateKind::Environment,
            previous.inputs != self.inputs,
            self.project.replacing(&previous.project),
            self.project.diagnostics(),
        )
    }

    /// What the session compiled, as it is now: its graph, environment,
    /// root, report, source texts and memo. Every reader goes through it.
    pub fn project(&self) -> &CompiledProject {
        &self.project
    }

    /// What the session depends on now (ADR 0030): what a changed path is,
    /// which directories to watch, which files to report.
    pub fn inputs(&self) -> &SessionInputs {
        &self.inputs
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

    /// The runtime the project's extensions run in (none: no extension
    /// loads).
    pub fn runtime(&self) -> Option<&SharedRuntime> {
        self.runtime.as_ref()
    }

    /// Whether a changed file is outside the project. A session with no
    /// project takes every buffer: it has no spec root to discover files
    /// under.
    fn excludes(&self, path: &str) -> bool {
        self.inputs.excludes(path)
    }

    /// Run every check again on the current graph: a check input changed.
    fn recheck(&mut self) -> Update {
        let inputs_changed = self.check();
        Update::of(
            UpdateKind::Checks,
            inputs_changed,
            Applied::default(),
            self.project.diagnostics(),
        )
    }

    /// Every check on the current graph, over a snapshot of it taken now,
    /// the check inputs (which the snapshot's `file_exists` rules add to)
    /// stamped first: whether the session's inputs changed.
    fn check(&mut self) -> bool {
        let entities = self.project.snapshot_now();
        let mut changed = false;
        if self.inputs.root().is_some() {
            let next = self.inputs.with_named(
                self.project
                    .environment()
                    .registries
                    .files(&entities.rule_input()),
            );
            changed = next != self.inputs;
            self.inputs = next;
            self.snapshot.stamp_checks(&self.inputs);
        }
        self.project.check_over(entities, self.runtime.as_deref());
        changed
    }
}
