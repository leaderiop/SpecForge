//! A long-lived compiled project: what watch, the LSP and MCP hold.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use specforge_common::Diagnostic;
use specforge_graph::{Graph, build_graph_with_config};
use specforge_parser::SpecFile;
use specforge_resolver::resolve_parsed;
use specforge_wasm::WasmRuntime;

use crate::delta::{GraphDelta, compute_graph_delta};
use crate::incremental::IncrementalBuild;
use crate::inputs::{Changes, InputRole, Origin, UpdateKind, canonical};
use crate::{Environment, sources_in_path_order};

/// The runtime a session runs its project's extensions in (every
/// [`WasmRuntime`] is `Send + Sync`).
pub type SharedRuntime = Arc<dyn WasmRuntime>;

/// What changed in a session's sources.
pub enum SourceChange<'a> {
    /// Files changed, created or deleted on disk, by path relative to the
    /// spec root. A file that can no longer be read was deleted.
    Disk(&'a [String]),
    /// An editor buffer is the truth for one file (`None`: it is gone).
    Buffer {
        path: &'a str,
        text: Option<&'a str>,
    },
}

/// Which checks an update runs on the updated graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMode<'a> {
    /// Every check `specforge check` runs.
    Full,
    /// The editor's fast path while typing: when this file (relative to
    /// the spec root) has parse errors the graph is broken and the checks
    /// would evaluate garbage, so they are skipped and only the parse
    /// layer is reported until it parses again. Otherwise, [`Self::Full`].
    SyntaxOnlyIfParseErrorsIn(&'a str),
}

/// What one update of a session did.
#[derive(Debug)]
pub struct Update {
    /// What was applied: sources, the checks alone, or the environment.
    pub kind: UpdateKind,
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
/// the changed files are re-parsed and the graph is patched with them
/// ([`crate::incremental`]), the imports of every cached parse are
/// resolved again (E025, I004, W113, W027), and the graph checks re-run on
/// the patched graph. After any sequence of updates,
/// [`ProjectSession::diagnostics`] is the set
/// [`crate::CompiledProject::diagnostics`] reports for the same sources.
pub struct ProjectSession {
    /// Shared, so a reader can keep the environment an update started
    /// from while the session itself is busy.
    env: Arc<Environment>,
    runtime: Option<SharedRuntime>,
    /// The session built its runtime, so a reload builds a fresh one (the
    /// extensions' `.wasm` files may have changed).
    owns_runtime: bool,
    build: IncrementalBuild,
    import_diagnostics: Vec<Diagnostic>,
    check_diagnostics: Vec<Diagnostic>,
    verify_incremental: bool,
    /// Where the project comes from. With [`Origin::None`] (an editor with
    /// no workspace folder) files are buffers keyed by absolute path, with
    /// no spec root to resolve their imports against and no environment to
    /// reload; with [`Origin::InMemory`] the graph was built by the host.
    origin: Origin,
}

impl ProjectSession {
    /// A session with no project: no config, no extension and no file
    /// until a buffer is added.
    pub fn detached() -> Self {
        ProjectSession {
            env: Arc::new(Environment::empty()),
            runtime: None,
            owns_runtime: false,
            build: IncrementalBuild::empty(),
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            verify_incremental: false,
            origin: Origin::None,
        }
    }

    /// A session serving `graph`, built in memory rather than from
    /// sources, in `env`: a host that assembles its graph itself (and a
    /// test) serves one. It has no file and runs no extension; it reports
    /// the environment's diagnostics and `graph_diagnostics` as its graph
    /// build's. It has nothing on disk to reload ([`Origin::InMemory`]); its
    /// root is the environment's.
    pub fn from_graph(
        env: Arc<Environment>,
        graph: Graph,
        graph_diagnostics: Vec<Diagnostic>,
    ) -> Self {
        let graph_config = env.graph_config();
        ProjectSession {
            env,
            runtime: None,
            owns_runtime: false,
            build: IncrementalBuild::from_cold_build(
                Vec::new(),
                graph,
                &graph_diagnostics,
                graph_config,
            ),
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            verify_incremental: false,
            origin: Origin::InMemory,
        }
    }

    /// Open the project at `root`, running its extensions in the project's
    /// own runtime.
    pub fn open(root: &Path) -> Self {
        let mut session = Self::open_with_runtime(root, Some(project_runtime(root)));
        session.owns_runtime = true;
        session
    }

    /// Open the project at `root` with `runtime` (none: no extension
    /// loads). A reload keeps using the same runtime.
    pub fn open_with_runtime(root: &Path, runtime: Option<SharedRuntime>) -> Self {
        let env = Environment::load(root, runtime.as_deref());
        let resolved = env.resolve();
        let (paths, specs): (Vec<String>, Vec<SpecFile>) =
            sources_in_path_order(&resolved).into_iter().unzip();
        let graph_config = env.graph_config();
        // The one cold build seeds the incremental one.
        let (graph, graph_diagnostics) = build_graph_with_config(&specs, &graph_config);
        let files: Vec<(String, SpecFile)> = paths.into_iter().zip(specs).collect();
        let build =
            IncrementalBuild::from_cold_build(files, graph, &graph_diagnostics, graph_config);

        let mut session = ProjectSession {
            env: Arc::new(env),
            runtime,
            owns_runtime: false,
            build,
            import_diagnostics: resolved.diagnostics,
            check_diagnostics: Vec::new(),
            verify_incremental: false,
            origin: Origin::Disk,
        };
        session.check_diagnostics = session.check();
        session
    }

    /// Compare every update's graph with a cold rebuild (costly: a debug
    /// build of watch does it always).
    pub fn set_verify_incremental(&mut self, enabled: bool) {
        self.verify_incremental = enabled;
        self.build.set_verify(enabled);
    }

    /// Apply a change to the project's sources, then run every check.
    /// Files discovery would not find (an `exclude` entry, a skipped
    /// directory, not `.spec`) are ignored.
    pub fn update(&mut self, change: SourceChange<'_>) -> Update {
        self.update_with(change, CheckMode::Full)
    }

    /// [`Self::update`], running the checks `mode` asks for.
    pub fn update_with(&mut self, change: SourceChange<'_>, mode: CheckMode<'_>) -> Update {
        let changes: Vec<(String, Option<String>)> = match change {
            SourceChange::Disk(paths) => paths
                .iter()
                .filter(|path| !self.excludes(path))
                .map(|path| {
                    let text = std::fs::read_to_string(self.env.spec_root.join(path)).ok();
                    (path.clone(), text)
                })
                .collect(),
            SourceChange::Buffer { path, .. } if self.excludes(path) => Vec::new(),
            SourceChange::Buffer { path, text } => {
                vec![(path.to_string(), text.map(str::to_string))]
            }
        };
        let result = self.build.rebuild(changes);
        self.import_diagnostics = self.resolve_imports();
        self.check_diagnostics = match mode {
            CheckMode::SyntaxOnlyIfParseErrorsIn(path)
                if self
                    .build
                    .file_diagnostics(path)
                    .iter()
                    .any(|d| d.code == "E001") =>
            {
                Vec::new()
            }
            _ => self.check(),
        };
        Update {
            kind: UpdateKind::Sources,
            delta: result.delta,
            rebuilt_files: result.rebuilt_files,
            changed_diagnostic_files: result.changed_diagnostic_files,
            diagnostics: self.diagnostics(),
            verification: result.verification,
        }
    }

    /// `specforge.json` or an extension changed: load the environment again
    /// and rebuild from the sources on disk.
    pub fn reload_environment(&mut self) -> Update {
        if self.origin != Origin::Disk {
            // Nothing on disk to load again.
            return Update {
                kind: UpdateKind::Environment,
                delta: GraphDelta::default(),
                rebuilt_files: Vec::new(),
                changed_diagnostic_files: Vec::new(),
                diagnostics: self.diagnostics(),
                verification: None,
            };
        }
        let root = self.env.root.clone();
        let runtime = if self.owns_runtime {
            Some(project_runtime(&root))
        } else {
            self.runtime.clone()
        };
        let mut next = Self::open_with_runtime(&root, runtime);
        next.owns_runtime = self.owns_runtime;
        next.set_verify_incremental(self.verify_incremental);
        let previous = std::mem::replace(self, next);
        self.replaced(&previous)
    }

    /// The update that replacing `previous` with this session amounts to:
    /// every file rebuilt, the delta between the two graphs.
    pub fn replaced(&self, previous: &ProjectSession) -> Update {
        Update {
            kind: UpdateKind::Environment,
            delta: compute_graph_delta(previous.graph(), self.graph()),
            rebuilt_files: self
                .build
                .parsed_files()
                .into_iter()
                .map(|(path, _)| path.to_string())
                .collect(),
            changed_diagnostic_files: self.build.diagnostic_files(),
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
            .chain(self.build.diagnostics())
            .chain(self.check_diagnostics.iter().cloned())
            .chain(self.env.surface_diagnostics().iter().cloned())
            .collect()
    }

    pub fn graph(&self) -> &Graph {
        self.build.graph()
    }

    /// Where the project comes from: disk, memory, or nowhere.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// The project root: `None` with no project ([`Origin::None`], or a
    /// session built in memory with no root).
    pub fn root(&self) -> Option<&Path> {
        let root = self.env.root.as_path();
        (self.origin != Origin::None && !root.as_os_str().is_empty()).then_some(root)
    }

    /// Where `.spec` files are keyed from: `None` unless the project was
    /// opened from disk.
    pub fn spec_root(&self) -> Option<&Path> {
        (self.origin == Origin::Disk).then_some(self.env.spec_root.as_path())
    }

    /// A `.spec` path's key in this session: relative to the spec root
    /// when the file is under it, else the path itself.
    pub fn source_key(&self, path: &Path) -> String {
        self.env.source_key(path)
    }

    /// What `path` (absolute, or relative to the working directory) is to
    /// this session (behavior `classify_project_changes`). A session built
    /// in memory is never changed by disk; with no project, a `.spec` file
    /// is a buffer source keyed by its path.
    pub fn classify(&self, path: &Path) -> InputRole {
        match self.origin {
            Origin::InMemory => InputRole::Unrelated,
            Origin::None => {
                if path.extension().is_some_and(|ext| ext == "spec") {
                    InputRole::Source(self.source_key(path))
                } else {
                    InputRole::Unrelated
                }
            }
            Origin::Disk => self.classify_on_disk(path),
        }
    }

    /// What a batch of changed paths means to this session.
    pub fn changes<'p>(&self, paths: impl IntoIterator<Item = &'p Path>) -> Changes {
        Changes::from_roles(paths.into_iter().map(|path| self.classify(path)))
    }

    /// Apply `changes`: the environment first (a reload rebuilds
    /// everything), else the sources (an update re-runs every check), else
    /// the checks alone. `None` when there is nothing to apply: `changes` is
    /// empty, or the session was built in memory.
    pub fn apply(&mut self, changes: &Changes) -> Option<Update> {
        match self.origin {
            Origin::InMemory => None,
            _ if changes.environment && self.origin == Origin::Disk => {
                Some(self.reload_environment())
            }
            _ if !changes.sources.is_empty() => {
                Some(self.update(SourceChange::Disk(&changes.sources)))
            }
            _ if changes.check_inputs && self.origin == Origin::Disk => Some(self.recheck()),
            _ => None,
        }
    }

    /// The directories a file watcher must watch to see every change this
    /// session is built from: the root, the spec root when it is outside
    /// the root, and the directory of every input outside both (canonical,
    /// existing, none inside another). Empty unless opened from disk.
    pub fn watch_roots(&self) -> Vec<PathBuf> {
        if self.origin != Origin::Disk {
            return Vec::new();
        }
        let inputs = self.env.inputs();
        let references = self.env.referenced_files(self.graph());
        let candidates = [canonical(&self.env.root), canonical(&self.env.spec_root)]
            .into_iter()
            .chain(
                inputs
                    .modules
                    .iter()
                    .chain(&inputs.check_inputs)
                    .chain(&references)
                    .filter_map(|path| path.parent().map(canonical)),
            )
            .filter(|dir| dir.is_dir());
        let mut roots: Vec<PathBuf> = Vec::new();
        for dir in candidates {
            if roots.iter().any(|root| dir.starts_with(root)) {
                continue;
            }
            roots.retain(|root| !root.starts_with(&dir));
            roots.push(dir);
        }
        roots
    }

    pub fn environment(&self) -> &Environment {
        &self.env
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
        self.build.diagnostics()
    }

    /// The graph build's diagnostics in one file (relative to the spec root).
    pub fn file_diagnostics(&self, path: &str) -> &[Diagnostic] {
        self.build.file_diagnostics(path)
    }

    /// Every file with graph-build diagnostics (sorted).
    pub fn diagnostic_files(&self) -> Vec<String> {
        self.build.diagnostic_files()
    }

    /// How many `.spec` files the project has.
    pub fn file_count(&self) -> usize {
        self.build.parsed_files().len()
    }

    /// Whether a changed file is outside the project. A session with no
    /// project takes every buffer: it has no spec root to discover files
    /// under.
    fn excludes(&self, path: &str) -> bool {
        self.origin == Origin::Disk && self.env.excludes(path)
    }

    /// [`Self::classify`] for a project on disk.
    fn classify_on_disk(&self, path: &Path) -> InputRole {
        let path = canonical(path);
        let inputs = self.env.inputs();
        if inputs
            .environment_paths()
            .any(|input| canonical(input) == path)
        {
            return InputRole::Environment;
        }
        let references = self.env.referenced_files(self.graph());
        let checked = inputs
            .check_inputs
            .iter()
            .chain(&references)
            .map(|input| canonical(input))
            .any(|input| input == path);
        // A missing referenced file's suggestion names a similar file in
        // its directory: a file created or deleted there changes it.
        let suggested = || {
            references
                .iter()
                .filter(|r| !r.exists())
                .any(|missing| missing.parent().map(canonical).as_deref() == path.parent())
        };
        if let Ok(relative) = path.strip_prefix(canonical(&self.env.spec_root)) {
            let key = relative.to_string_lossy().into_owned();
            if !self.env.excludes(&key) {
                return InputRole::Source(key);
            }
        }
        if checked || suggested() {
            return InputRole::CheckInput;
        }
        InputRole::Unrelated
    }

    /// Run every check again on the current graph: a check input changed.
    fn recheck(&mut self) -> Update {
        self.check_diagnostics = self.check();
        Update {
            kind: UpdateKind::Checks,
            delta: GraphDelta::default(),
            rebuilt_files: Vec::new(),
            changed_diagnostic_files: Vec::new(),
            diagnostics: self.diagnostics(),
            verification: None,
        }
    }

    fn resolve_imports(&self) -> Vec<Diagnostic> {
        if self.origin != Origin::Disk {
            return Vec::new();
        }
        resolve_parsed(
            &self.env.spec_root,
            &self.build.parsed_files(),
            &self.env.resolve_config(),
            &|path: &Path| path.is_file(),
        )
        .diagnostics
    }

    fn check(&self) -> Vec<Diagnostic> {
        self.env
            .run_checks(self.build.graph(), self.runtime.as_deref())
    }
}

fn project_runtime(root: &Path) -> SharedRuntime {
    Arc::new(specforge_component::project_runtime(root))
}
