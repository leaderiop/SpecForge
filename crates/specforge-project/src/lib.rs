//! The compiled project (architecture plan 01, ADR 0004 D1-d).
//!
//! Every surface (`specforge check` and the other CLI commands, watch, the
//! LSP, MCP) used to assemble "config, extensions, registries, resolve,
//! graph, checks" on its own, and each copy drifted. This crate owns that
//! assembly:
//! - an [`Environment`] is everything derived from `specforge.json` and the
//!   loaded extensions before any `.spec` file is read;
//! - a [`CompiledProject`] is an environment plus the resolved sources and
//!   the built graph. Its [`CompiledProject::diagnostics`] are, by
//!   definition, what `specforge check` reports;
//! - a [`ProjectSession`] is a long-lived compiled project that knows what
//!   it is built from: its inputs ([`SessionInputs`], `ProjectSession::inputs`)
//!   say what a changed path is ([`InputRole`]), and it applies changes as
//!   an update, an environment reload or a re-check
//!   (watch, the LSP and MCP each hold one). After any sequence of updates
//!   its diagnostics are the set a fresh compile reports.

mod build_cache;
mod check_passes;
pub mod compile;
pub mod coverage;
pub mod field_types;
mod freshness;
mod inputs;
pub mod passes;
mod policy;
mod session;
pub mod snapshot;
mod sources;
pub mod verdicts;

use std::sync::Arc;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sources::SourceCache;

use compile::{GraphChecks, check_graph, load_extensions};
use coverage::RecordedCoverage;
use snapshot::EntitySnapshot;
use specforge_common::{
    ConfigProblem, ConfigRead, Diagnostic, ProjectConfig, codes, discover_spec_files,
    is_discovered, read_project_config,
};
use specforge_graph::{Graph, GraphBuild, GraphConfig};
use specforge_parser::SpecFile;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{
    RegistryBuild, build_registries, load_provider_configurations, register_provider_schemes,
};
use specforge_resolver::{ResolveConfig, resolve_parsed};
use specforge_wasm::{LockState, WasmRuntime};

pub use build_cache::{BUILD_CACHE_FILE, BUILD_CACHE_FORMAT, BuildCache, CachedStatus};
pub use compile::EnabledExtension;
pub use inputs::{Changes, InputRole, SessionInputs, UpdateKind, WatchRoot, Watched, source_key};
pub use policy::{
    DiagnosticPolicy, LINT_PROFILE_NAMES, LintProfile, UnknownLintProfile, apply_policy,
};
pub use session::{
    CheckMode, OpeningProject, ProjectSession, RuntimeSource, SharedRuntime, SourceChange, Update,
};
pub use specforge_graph::{
    EdgeChange, GraphDelta, ModifiedNodeChange, NodeChange, compute_graph_delta,
};

/// Everything derived from `specforge.json` and the loaded extensions,
/// before any `.spec` file is read.
pub struct Environment {
    /// The project root (where `specforge.json` lives).
    pub root: PathBuf,
    pub config: ProjectConfig,
    /// How `specforge.json` is not used as written, in file order: each is
    /// reported as E069, first. With one that
    /// [`loads_nothing`](ConfigProblem::loads_nothing), the compile loaded
    /// no extension.
    pub config_problems: Vec<ConfigProblem>,
    /// `specforge.json` exists at the root. `false` for [`Self::empty`],
    /// [`Self::from_declarations`] and [`Self::with_registries`] (no file
    /// was read).
    pub config_found: bool,
    /// What `specforge.lock` held when the environment was read (absent,
    /// read, or unreadable with its problem): one read per environment,
    /// which every operation over the project reads instead of the disk.
    /// A changed lock reloads the environment ([`SessionInputs`]).
    pub lock: LockState,
    /// What each `specforge.json` `extensions` entry enables, in order, as
    /// the runtime loaded it (a `.wasm` file entry by the name its
    /// component declares).
    pub enabled: Vec<EnabledExtension>,
    /// Where `.spec` files are discovered: `spec_root` from the config,
    /// relative to the project root, or the project root itself.
    pub spec_root: PathBuf,
    /// The registries, rules, passes and graph inputs built from the loaded
    /// declarations.
    pub registries: RegistryBuild,
    /// The ref schemes the configured providers registered (ADR 0004
    /// D3-c): with any registered, a ref with another scheme is I005.
    pub provider_schemes: HashSet<String>,
    /// Extension loading diagnostics: one E069 per config problem, the
    /// runtime's load failures (E028/E033) in load order, then the
    /// declarations' unknown keys (W138).
    pub load_diagnostics: Vec<Diagnostic>,
    /// After the registry build: provider registration (W118/E057), then
    /// I002 when no extension loaded.
    pub setup_diagnostics: Vec<Diagnostic>,
}

impl Environment {
    /// No project: the default config, no spec root, no extension.
    pub fn empty() -> Self {
        Environment {
            root: PathBuf::new(),
            config: ProjectConfig::default(),
            config_problems: Vec::new(),
            config_found: false,
            lock: LockState::Absent,
            enabled: Vec::new(),
            spec_root: PathBuf::new(),
            registries: RegistryBuild::default(),
            provider_schemes: HashSet::new(),
            load_diagnostics: Vec::new(),
            setup_diagnostics: Vec::new(),
        }
    }

    /// An environment of `declarations` (in load order) and no project:
    /// the default config, no spec root, the registry build of exactly
    /// these declarations.
    pub fn from_declarations(declarations: Vec<ExtensionDeclaration>) -> Self {
        Environment {
            registries: build_registries(declarations),
            ..Environment::empty()
        }
    }

    /// An environment of exactly `registries` and no project: the default
    /// config, nothing enabled, no spec root (tests, a graph built in
    /// memory).
    pub fn with_registries(registries: RegistryBuild) -> Self {
        Environment {
            registries,
            ..Environment::empty()
        }
    }

    /// Read the project's config and load its extensions through `runtime`
    /// (none without one), then build the registries from them.
    pub fn load(root: &Path, runtime: Option<&dyn WasmRuntime>) -> Self {
        Self::from_read(root, read_project_config(root), runtime)
    }

    /// The environment of the config `read` (the one read of
    /// `specforge.json`), its extensions loaded through `runtime`.
    pub fn from_read(root: &Path, read: ConfigRead, runtime: Option<&dyn WasmRuntime>) -> Self {
        let config = read.config;
        let enabled = config
            .extensions
            .iter()
            .map(|entry| EnabledExtension::of(entry, runtime))
            .collect();
        let mut load_diagnostics: Vec<Diagnostic> = read
            .problems
            .iter()
            .map(config_problem_diagnostic)
            .collect();
        let declarations = match runtime {
            Some(runtime) => load_extensions(&config.extensions, runtime, &mut load_diagnostics),
            None => Vec::new(),
        };
        let mut registries = build_registries(declarations);
        // A custom rule's wasm_function is resolved against its extension
        // now, so a name it does not export is reported once (W112).
        if let Some(runtime) = runtime {
            let probes = registries
                .rules
                .probe(&verdicts::WasmVerdicts::probe_only(runtime));
            registries.registry_diagnostics.extend(probes);
        }
        let mut setup_diagnostics = Vec::new();
        let provider_schemes = register_providers(&config, &registries, &mut setup_diagnostics);
        if registries.declarations().is_empty() {
            setup_diagnostics.push(structural_only_notice(&config.extensions, &read.problems));
        }
        let spec_root = config.spec_root_in(root);
        Environment {
            root: root.to_path_buf(),
            config,
            config_problems: read.problems,
            config_found: read.found,
            lock: LockState::at(root),
            enabled,
            spec_root,
            registries,
            provider_schemes,
            load_diagnostics,
            setup_diagnostics,
        }
    }

    /// The inputs every graph of this project is built with.
    pub fn graph_config(&self) -> GraphConfig {
        GraphConfig {
            known_provider_schemes: self.provider_schemes.clone(),
            ..compile::graph_config(&self.registries)
        }
    }

    /// The entity snapshot of `graph`, built in this environment: its
    /// registries decide each entity's standing and its spec root resolves
    /// the relative paths the rules read. The one place a snapshot is
    /// taken from an environment.
    pub fn entity_snapshot(&self, graph: &Graph) -> EntitySnapshot {
        EntitySnapshot::of(graph, &self.registries, &self.spec_root)
    }

    /// What the checks on a built graph need from this environment, with
    /// the graph's entity snapshot.
    pub fn checks<'a>(
        &'a self,
        entities: &'a EntitySnapshot,
        runtime: Option<&'a dyn WasmRuntime>,
    ) -> GraphChecks<'a> {
        GraphChecks {
            spec_root: &self.spec_root,
            registries: &self.registries,
            entities,
            runtime,
        }
    }

    /// Every check a compile runs on a built graph, over its entity
    /// snapshot `entities`: the graph checks (core validation, the
    /// registry checks, the extensions' rules), then the check-phase
    /// passes.
    pub fn run_checks(
        &self,
        graph: &Graph,
        entities: &EntitySnapshot,
        runtime: Option<&dyn WasmRuntime>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = check_graph(graph, &self.checks(entities, runtime));
        if let Some(runtime) = runtime {
            diagnostics.extend(check_passes::run(self, graph, entities, runtime));
        }
        diagnostics
    }

    /// The diagnostics reported before any source, in this order: why
    /// `specforge.json` is not used as written (E069), the runtime's load
    /// failures (E028/E033), unknown declaration keys
    /// (W138), the declarations' own (E030, W021, E027, W145), provider
    /// registration (W118/E057), I002, then the registry build's.
    pub fn diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.load_diagnostics
            .iter()
            .chain(&self.registries.declaration_diagnostics)
            .chain(&self.setup_diagnostics)
            .chain(&self.registries.registry_diagnostics)
    }

    /// Surface registration conflicts (E039), which `check` reports last.
    pub fn surface_diagnostics(&self) -> &[Diagnostic] {
        &self.registries.surface_diagnostics
    }

    /// How imports resolve and which files are discovered: the config's
    /// `exclude` entries apply, relative to the spec root.
    pub fn resolve_config(&self) -> ResolveConfig {
        ResolveConfig {
            exclude: self.config.exclude.clone(),
            ..ResolveConfig::default()
        }
    }

    /// Whether a file (its path relative to the spec root) is left out of
    /// the project, as discovery leaves it out: not a `.spec` file, under a
    /// skipped directory (`target`, `node_modules`, ...) or matched by an
    /// `exclude` entry.
    pub fn excludes(&self, relative: &str) -> bool {
        !is_discovered(relative, &self.config.exclude)
    }

    /// The project sources discovery finds now (`exclude` and the skipped
    /// directories applied).
    pub fn discover(&self) -> Vec<PathBuf> {
        discover_spec_files(&self.spec_root, &self.config.exclude)
    }

    /// Read and parse `discovered`, build their graph and resolve their
    /// imports: the one cold build every compile, session open and
    /// extension-command graph starts from (ADR 0032).
    pub(crate) fn build_sources(&self, discovered: &[PathBuf]) -> SourceBuild {
        let (sources, files) = SourceCache::read_all(&self.spec_root, discovered);
        let graph = GraphBuild::of(files, self.graph_config());
        let imports = self.import_diagnostics(&sources, &graph);
        SourceBuild {
            sources,
            graph,
            imports,
        }
    }

    /// The import diagnostics of what `sources` and `graph` hold: E025 for
    /// each source that could not be read, then the resolver's (E025,
    /// I004, W113, W027). The same function after a cold read and after
    /// every update.
    pub(crate) fn import_diagnostics(
        &self,
        sources: &SourceCache,
        graph: &GraphBuild,
    ) -> Vec<Diagnostic> {
        let files: Vec<(&str, &SpecFile)> = graph.files().collect();
        sources
            .unreadable()
            .cloned()
            .chain(
                resolve_parsed(
                    &self.spec_root,
                    &files,
                    &ResolveConfig::default(),
                    &|path: &Path| path.is_file(),
                )
                .diagnostics,
            )
            .collect()
    }

    /// The graph of the project's sources, as a compile builds it, without
    /// the checks a compile then runs on it: what a query over the project
    /// reads (an extension command, ADR 0008).
    pub fn build_graph(&self) -> Graph {
        self.build_sources(&self.discover()).graph.into_parts().0
    }
}

/// What a compile builds from the sources on disk (ADR 0032).
pub(crate) struct SourceBuild {
    pub sources: SourceCache,
    pub graph: GraphBuild,
    /// E025 for the unreadable sources, then the resolver's (E025, I004,
    /// W113, W027).
    pub imports: Vec<Diagnostic>,
}

/// Register the `providers` specforge.json configures against the loaded
/// extensions, in declaration order, and return the schemes they
/// registered. W118 (a malformed entry, or an extension that is not
/// loaded or contributes no providers) and E057 (a scheme declared twice)
/// go to the load diagnostics.
fn register_providers(
    config: &ProjectConfig,
    registries: &RegistryBuild,
    diagnostics: &mut Vec<Diagnostic>,
) -> HashSet<String> {
    let Some(raw) = config.raw.as_ref() else {
        return HashSet::new();
    };
    let (providers, config_diagnostics) = load_provider_configurations(raw);
    diagnostics.extend(config_diagnostics);
    let (schemes, registration) = register_provider_schemes(&providers, registries.declarations());
    diagnostics.extend(registration);
    schemes.entries.into_iter().map(|e| e.scheme).collect()
}

/// E069: one way `specforge.json` is not used as written. An error: a
/// config that loads nothing must not pass `check`.
fn config_problem_diagnostic(problem: &ConfigProblem) -> Diagnostic {
    let message = if problem.loads_nothing() {
        format!("specforge.json can't be used: {problem}; no extension is loaded")
    } else {
        format!("specforge.json: {problem}; it is ignored")
    };
    Diagnostic::new(codes::E069, message).with_suggestion(format!(
        "fix specforge.json; `specforge explain {}` says what it must be",
        codes::E069
    ))
}

/// I002: no extension loaded, so the compile checks structure only (no
/// kind, field or rule checks). Reported after the errors that caused it,
/// if any: the config problems (E069) that left nothing to load, or the
/// load errors (E028).
fn structural_only_notice(configured: &[String], problems: &[ConfigProblem]) -> Diagnostic {
    let (message, suggestion) = if problems.iter().any(ConfigProblem::loads_nothing) {
        (
            "specforge.json could not be read — operating in structural-only mode".to_string(),
            format!("fix specforge.json ({} above)", codes::E069),
        )
    } else if configured.is_empty() {
        (
            "no extensions configured — operating in structural-only mode".to_string(),
            "enable kind-specific checks with: specforge add @specforge/software".to_string(),
        )
    } else {
        let which = match configured.len() {
            1 => "the configured extension did not load".to_string(),
            n => format!("none of the {n} configured extensions loaded"),
        };
        (
            format!("{which} — operating in structural-only mode"),
            "fix the extension load errors above (`specforge doctor` checks the setup)".to_string(),
        )
    };
    Diagnostic::new(codes::I002, message).with_suggestion(suggestion)
}

/// A one-shot compile: an environment, the sources it read and the graph
/// built from them. What `specforge check` and every CLI command use.
pub struct CompiledProject {
    pub env: Environment,
    /// The text of every source, as read.
    sources: SourceCache,
    /// E025 for the unreadable sources, then the resolver's diagnostics.
    import_diagnostics: Vec<Diagnostic>,
    pub graph: Graph,
    /// What building the graph reported (parse errors, duplicates,
    /// unresolved references, reference cycles).
    pub graph_diagnostics: Vec<Diagnostic>,
    /// What the checks on the built graph reported: core validation, the
    /// registry checks, the extensions' rules, then the check-phase passes.
    pub check_diagnostics: Vec<Diagnostic>,
    /// The graph's entity snapshot: what its checks read (ADR 0019).
    entities: Arc<EntitySnapshot>,
    /// The recorded test report at the root and the coverage of the graph
    /// against it, memoized for the life of this compile, seeded with
    /// `entities`.
    recorded: RecordedCoverage,
}

impl CompiledProject {
    /// Compile the project at `root`, running its extensions in `runtime`.
    /// Without a runtime no extension is loaded.
    pub fn compile(root: &Path, runtime: Option<&dyn WasmRuntime>) -> Self {
        let env = Environment::load(root, runtime);
        let SourceBuild {
            sources,
            graph,
            imports,
        } = env.build_sources(&env.discover());
        let (graph, graph_diagnostics) = graph.into_parts();
        let entities = Arc::new(env.entity_snapshot(&graph));
        let check_diagnostics = env.run_checks(&graph, &entities, runtime);
        CompiledProject {
            env,
            sources,
            import_diagnostics: imports,
            graph,
            graph_diagnostics,
            check_diagnostics,
            recorded: RecordedCoverage::of(Arc::clone(&entities)),
            entities,
        }
    }

    /// Each source's text, by its path relative to the spec root: exactly
    /// what was parsed, for quoting in rendered diagnostics without
    /// reading the disk again.
    pub fn source_texts(&self) -> HashMap<String, String> {
        self.sources
            .texts()
            .into_iter()
            .map(|(path, text)| (path, text.to_string()))
            .collect()
    }

    /// The graph's entity snapshot, the one its checks read.
    pub fn entities(&self) -> &EntitySnapshot {
        &self.entities
    }

    /// Exactly what `specforge check` reports, in its order: the
    /// environment's, the resolver's, the graph build's, the checks', then
    /// surface conflicts.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.env
            .diagnostics()
            .chain(&self.import_diagnostics)
            .chain(&self.graph_diagnostics)
            .chain(&self.check_diagnostics)
            .chain(self.env.surface_diagnostics())
            .cloned()
            .collect()
    }

    /// The recorded test report at the project root and the coverage of the
    /// graph against it, memoized for this compile.
    pub fn recorded(&self) -> &RecordedCoverage {
        &self.recorded
    }
}
