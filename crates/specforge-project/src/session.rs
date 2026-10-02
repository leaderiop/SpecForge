//! A long-lived compiled project: what watch, the LSP and MCP hold.

use std::path::Path;
use std::sync::Arc;

use specforge_common::Diagnostic;
use specforge_graph::{Graph, build_graph_with_config};
use specforge_parser::SpecFile;
use specforge_resolver::resolve_parsed;
use specforge_wasm::WasmRuntime;
use specforge_watch::{GraphDelta, ImportDag, IncrementalPipeline, compute_graph_delta};

use crate::{Environment, sources_in_path_order};

/// The runtime a session runs its project's extensions in (every
/// [`WasmRuntime`] is `Send + Sync`).
pub type SharedRuntime = Arc<dyn WasmRuntime>;

/// What changed in a session's sources.
pub enum SourceChange<'a> {
    /// Files changed, created or deleted on disk, by path relative to the
    /// spec root. A file that can no longer be read was deleted.
    Disk(&'a [String]),
    /// An editor buffer is the truth for one file (`None`: it is gone);
    /// the files its change invalidates are re-read from disk.
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
    pub delta: GraphDelta,
    /// The files re-parsed (sorted).
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
/// It is seeded by one cold build, then kept current incrementally: the
/// graph through the shared [`IncrementalPipeline`], import diagnostics
/// (E025, I004, W113, W027) through the resolver over the cached parses,
/// and the graph checks re-run on the updated graph. After any sequence of
/// updates, [`ProjectSession::diagnostics`] is the set
/// [`crate::CompiledProject::diagnostics`] reports for the same sources.
pub struct ProjectSession {
    /// Shared, so a reader can keep the environment an update started
    /// from while the session itself is busy.
    env: Arc<Environment>,
    runtime: Option<SharedRuntime>,
    /// The session built its runtime, so a reload builds a fresh one (the
    /// extensions' `.wasm` files may have changed).
    owns_runtime: bool,
    pipeline: IncrementalPipeline,
    import_diagnostics: Vec<Diagnostic>,
    check_diagnostics: Vec<Diagnostic>,
    verify_incremental: bool,
    /// No project is open (an editor with no workspace folder): files are
    /// buffers keyed by absolute path, with no spec root to resolve their
    /// imports against and no environment to reload.
    detached: bool,
}

impl ProjectSession {
    /// A session with no project: no config, no extension and no file
    /// until a buffer is added.
    pub fn detached() -> Self {
        ProjectSession {
            env: Arc::new(Environment::empty()),
            runtime: None,
            owns_runtime: false,
            pipeline: IncrementalPipeline::empty(),
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            verify_incremental: false,
            detached: true,
        }
    }

    /// A session serving `graph`, built in memory rather than from
    /// sources, in `env`: a host that assembles its graph itself (and a
    /// test) serves one. It has no file and runs no extension; it reports
    /// the environment's diagnostics and `graph_diagnostics` as its graph
    /// build's. Like a [`Self::detached`] session, it has nothing on disk
    /// to reload.
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
            pipeline: IncrementalPipeline::from_cold_build(
                Vec::new(),
                graph,
                ImportDag::new(),
                graph_diagnostics,
                graph_config,
            ),
            import_diagnostics: Vec::new(),
            check_diagnostics: Vec::new(),
            verify_incremental: false,
            detached: true,
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
        // The one cold build: the pipeline is seeded with its graph.
        let (graph, graph_diagnostics) = build_graph_with_config(&specs, &graph_config);
        let files: Vec<(String, SpecFile)> = paths.into_iter().zip(specs).collect();

        // Every file is known before any import is resolved against them.
        let mut dag = ImportDag::new();
        for (path, _) in &files {
            dag.set_imports(path, Vec::new());
        }
        for (path, spec_file) in &files {
            let imports = spec_file
                .imports
                .iter()
                .map(|i| i.path.to_string())
                .collect();
            dag.set_imports_resolved(path, imports);
        }
        let pipeline = IncrementalPipeline::from_cold_build(
            files,
            graph,
            dag,
            graph_diagnostics,
            graph_config,
        );

        let mut session = ProjectSession {
            env: Arc::new(env),
            runtime,
            owns_runtime: false,
            pipeline,
            import_diagnostics: resolved.diagnostics,
            check_diagnostics: Vec::new(),
            verify_incremental: false,
            detached: false,
        };
        session.check_diagnostics = session.check();
        session
    }

    /// Compare every update's graph with a cold rebuild (costly: a debug
    /// build of watch does it always).
    pub fn set_verify_incremental(&mut self, enabled: bool) {
        self.verify_incremental = enabled;
        self.pipeline.set_verify_incremental(enabled);
    }

    /// Apply a change to the project's sources, then run every check.
    /// Files an `exclude` entry matches are ignored.
    pub fn update(&mut self, change: SourceChange<'_>) -> Update {
        self.update_with(change, CheckMode::Full)
    }

    /// [`Self::update`], running the checks `mode` asks for.
    pub fn update_with(&mut self, change: SourceChange<'_>, mode: CheckMode<'_>) -> Update {
        let spec_root = self.env.spec_root.clone();
        let read = |file: &str| std::fs::read_to_string(spec_root.join(file)).ok();
        let result = match change {
            SourceChange::Disk(paths) => {
                let paths: Vec<String> = paths
                    .iter()
                    .filter(|path| !self.env.excludes(path))
                    .cloned()
                    .collect();
                self.pipeline.rebuild(&paths, read)
            }
            SourceChange::Buffer { path, .. } if self.env.excludes(path) => {
                self.pipeline.rebuild(&[], read)
            }
            SourceChange::Buffer { path, text } => self.pipeline.update_open_file(path, text, read),
        };
        self.import_diagnostics = self.resolve_imports();
        self.check_diagnostics = match mode {
            CheckMode::SyntaxOnlyIfParseErrorsIn(path)
                if self
                    .pipeline
                    .file_diagnostics(path)
                    .iter()
                    .any(|d| d.code == "E001") =>
            {
                Vec::new()
            }
            _ => self.check(),
        };
        Update {
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
        let previous = self.pipeline.graph().clone();
        if self.detached {
            // Nothing on disk to load again.
            return Update {
                delta: compute_graph_delta(&previous, self.graph()),
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
        *self = next;
        let mut rebuilt_files: Vec<String> = self
            .pipeline
            .parsed_files()
            .into_iter()
            .map(|(path, _)| path.to_string())
            .collect();
        rebuilt_files.sort();
        Update {
            delta: compute_graph_delta(&previous, self.graph()),
            rebuilt_files,
            changed_diagnostic_files: self.pipeline.diagnostic_files(),
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
            .chain(self.pipeline.diagnostics())
            .chain(self.check_diagnostics.iter().cloned())
            .chain(self.env.surface_diagnostics().iter().cloned())
            .collect()
    }

    pub fn graph(&self) -> &Graph {
        self.pipeline.graph()
    }

    /// Whether the session has no project on disk ([`Self::detached`],
    /// [`Self::from_graph`]): no spec root to read changed files from and
    /// nothing to reload.
    pub fn is_detached(&self) -> bool {
        self.detached
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

    /// The incremental core the session drives (its import DAG, cached
    /// parses and per-file diagnostics).
    pub fn pipeline(&self) -> &IncrementalPipeline {
        &self.pipeline
    }

    /// How many `.spec` files the project has.
    pub fn file_count(&self) -> usize {
        self.pipeline.parsed_files().len()
    }

    fn resolve_imports(&self) -> Vec<Diagnostic> {
        if self.detached {
            return Vec::new();
        }
        resolve_parsed(
            &self.env.spec_root,
            &self.pipeline.parsed_files(),
            &self.env.resolve_config(),
            &|path: &Path| path.is_file(),
        )
        .diagnostics
    }

    fn check(&self) -> Vec<Diagnostic> {
        self.env
            .run_checks(self.pipeline.graph(), self.runtime.as_deref())
    }
}

fn project_runtime(root: &Path) -> SharedRuntime {
    Arc::new(specforge_component::project_runtime(root))
}
