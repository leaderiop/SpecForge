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
//!   it is built from: it classifies any changed path ([`InputRole`]) and
//!   applies changes as an update, an environment reload or a re-check
//!   (watch, the LSP and MCP each hold one). After any sequence of updates
//!   its diagnostics are the set a fresh compile reports.
//!
//! [`CompilationContext`] is the flat view older callers read; it is built
//! from a compiled project with [`CompiledProject::into_context`].

mod build_cache;
mod check_passes;
pub mod compile;
pub mod coverage;
mod delta;
pub mod field_types;
mod freshness;
mod incremental;
mod inputs;
pub mod passes;
mod policy;
mod session;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use compile::{GraphChecks, check_graph, load_extensions, probe_custom_rules};
use coverage::RecordedCoverage;
use specforge_common::{Diagnostic, ProjectConfig, is_discovered, load_project_config};
use specforge_graph::{Graph, GraphConfig, build_graph_with_config};
use specforge_parser::SpecFile;
use specforge_registry::{
    ManifestV2, RegistryBuild, build_registries, load_provider_configurations,
    register_provider_schemes,
};
use specforge_resolver::{ResolveConfig, ResolvedProject, resolve_project_with_config};
use specforge_wasm::WasmRuntime;

pub use build_cache::{
    BUILD_CACHE_FILE, BUILD_CACHE_FORMAT, BuildCache, CachedStatus, record_build_cache,
};
pub use check_passes::CheckPass;
pub use compile::CompilationContext;
pub use delta::{EdgeChange, GraphDelta, ModifiedNodeChange, NodeChange, compute_graph_delta};
pub use inputs::{Changes, EnvironmentInputs, InputRole, Origin, UpdateKind, source_key};
pub use policy::{
    DiagnosticPolicy, LINT_PROFILE_NAMES, LintProfile, UnknownLintProfile, apply_policy,
};
pub use session::{CheckMode, ProjectSession, SharedRuntime, SourceChange, Update};

/// Everything derived from `specforge.json` and the loaded extensions,
/// before any `.spec` file is read.
pub struct Environment {
    /// The project root (where `specforge.json` lives).
    pub root: PathBuf,
    pub config: ProjectConfig,
    /// Where `.spec` files are discovered: `spec_root` from the config,
    /// relative to the project root, or the project root itself.
    pub spec_root: PathBuf,
    /// The registries, rules and graph inputs built from the manifests.
    pub registries: RegistryBuild,
    /// The ref schemes the configured providers registered (ADR 0004
    /// D3-c): with any registered, a ref with another scheme is I005.
    pub provider_schemes: HashSet<String>,
    /// Extension loading diagnostics (E028, manifest validation, peer
    /// consistency), in load order.
    pub load_diagnostics: Vec<Diagnostic>,
    /// The compiler passes the extensions declare with `phase: "check"`,
    /// in the order every compile runs them after the graph checks.
    pub check_passes: Vec<CheckPass>,
}

impl Environment {
    /// No project: the default config, no spec root, no extension.
    pub fn empty() -> Self {
        Environment {
            root: PathBuf::new(),
            config: ProjectConfig::default(),
            spec_root: PathBuf::new(),
            registries: RegistryBuild::default(),
            provider_schemes: HashSet::new(),
            load_diagnostics: Vec::new(),
            check_passes: Vec::new(),
        }
    }

    /// Read the project's config and load its extensions through `runtime`
    /// (none without one), then build the registries from them.
    pub fn load(root: &Path, runtime: Option<&dyn WasmRuntime>) -> Self {
        let config = load_project_config(root);
        let mut load_diagnostics = Vec::new();
        let manifests = match runtime {
            Some(runtime) => load_extensions(&config.extensions, runtime, &mut load_diagnostics),
            None => Vec::new(),
        };
        let mut registries = build_registries(manifests);
        // A custom rule's wasm_function is resolved against its extension
        // now, so a name it does not export is reported once (W112).
        if let Some(runtime) = runtime {
            let probes = probe_custom_rules(&registries.rules, runtime);
            registries.registry_diagnostics.extend(probes);
        }
        let provider_schemes = register_providers(&config, &registries, &mut load_diagnostics);
        let check_passes = match runtime {
            Some(runtime) => check_passes::declared(&registries.manifests, runtime),
            None => Vec::new(),
        };
        if registries.manifests.is_empty() {
            load_diagnostics.push(structural_only_notice(&config.extensions));
        }
        let spec_root = match &config.spec_root {
            Some(spec_root) => root.join(spec_root),
            None => root.to_path_buf(),
        };
        Environment {
            root: root.to_path_buf(),
            config,
            spec_root,
            registries,
            provider_schemes,
            load_diagnostics,
            check_passes,
        }
    }

    /// The inputs every graph of this project is built with.
    pub fn graph_config(&self) -> GraphConfig {
        GraphConfig {
            known_provider_schemes: self.provider_schemes.clone(),
            ..compile::graph_config(&self.registries)
        }
    }

    /// What the checks on a built graph need from this environment.
    pub fn checks<'a>(&'a self, runtime: Option<&'a dyn WasmRuntime>) -> GraphChecks<'a> {
        GraphChecks {
            spec_root: &self.spec_root,
            kind_registry: &self.registries.kinds,
            field_registry: &self.registries.fields,
            rules: &self.registries.rules,
            runtime,
        }
    }

    /// Every check a compile runs on a built graph: the graph checks
    /// (core validation, the registry checks, the extensions' rules), then
    /// the check-phase passes.
    pub fn run_checks(&self, graph: &Graph, runtime: Option<&dyn WasmRuntime>) -> Vec<Diagnostic> {
        let mut diagnostics = check_graph(graph, &self.checks(runtime));
        if let Some(runtime) = runtime {
            diagnostics.extend(check_passes::run(self, graph, runtime));
        }
        diagnostics
    }

    /// The diagnostics reported before any source: extension loading, then
    /// the registry build.
    pub fn diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.load_diagnostics
            .iter()
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

    /// Discover, parse and resolve the project's `.spec` files.
    pub fn resolve(&self) -> ResolvedProject {
        resolve_project_with_config(&self.spec_root, &self.resolve_config())
    }

    /// The graph of the project's sources, as a compile builds it, without
    /// the checks a compile then runs on it: what a query over the project
    /// reads (an extension command, ADR 0008).
    pub fn build_graph(&self) -> Graph {
        build_graph_with_config(&source_files(&self.resolve()), &self.graph_config()).0
    }
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
    let manifests: Vec<(String, ManifestV2)> = registries
        .manifests
        .iter()
        .map(|m| (m.name.clone(), m.clone()))
        .collect();
    let (schemes, registration) = register_provider_schemes(&providers, &manifests);
    diagnostics.extend(registration);
    schemes.entries.into_iter().map(|e| e.scheme).collect()
}

/// I002: no extension loaded, so the compile checks structure only (no
/// kind, field or rule checks). Reported after the load errors (E028) that
/// caused it, if any.
fn structural_only_notice(configured: &[String]) -> Diagnostic {
    let (message, suggestion) = if configured.is_empty() {
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
    Diagnostic {
        code: "I002".to_string(),
        severity: specforge_common::Severity::Info,
        message,
        span: None,
        suggestion: Some(suggestion),
        data: None,
    }
}

/// The resolved files a graph is built from, in path order.
fn source_files(resolved: &ResolvedProject) -> Vec<SpecFile> {
    sources_in_path_order(resolved)
        .into_iter()
        .map(|(_, spec_file)| spec_file)
        .collect()
}

/// The resolved files as the graph is built from them: in path order, the
/// order an incremental rebuild applies first-writer-wins in too.
fn sources_in_path_order(resolved: &ResolvedProject) -> Vec<(String, SpecFile)> {
    let mut sources: Vec<(String, SpecFile)> = resolved
        .files
        .iter()
        .map(|f| (f.path.clone(), f.spec_file.clone()))
        .collect();
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    sources
}

/// A one-shot compile: an environment, the resolved sources and the graph
/// built from them. What `specforge check` and every CLI command use.
pub struct CompiledProject {
    pub env: Environment,
    pub resolved: ResolvedProject,
    pub graph: Graph,
    /// What building the graph reported (parse errors, duplicates,
    /// unresolved references, reference cycles).
    pub graph_diagnostics: Vec<Diagnostic>,
    /// What the checks on the built graph reported: core validation, the
    /// registry checks, the extensions' rules, then the check-phase passes.
    pub check_diagnostics: Vec<Diagnostic>,
    /// The recorded test report at the root and the coverage of the graph
    /// against it, memoized for the life of this compile.
    recorded: RecordedCoverage,
}

impl CompiledProject {
    /// Compile the project at `root`, running its extensions in `runtime`.
    /// Without a runtime no extension is loaded.
    pub fn compile(root: &Path, runtime: Option<&dyn WasmRuntime>) -> Self {
        let env = Environment::load(root, runtime);
        let resolved = env.resolve();
        let (graph, graph_diagnostics) =
            build_graph_with_config(&source_files(&resolved), &env.graph_config());
        let check_diagnostics = env.run_checks(&graph, runtime);
        CompiledProject {
            env,
            resolved,
            graph,
            graph_diagnostics,
            check_diagnostics,
            recorded: RecordedCoverage::default(),
        }
    }

    /// Exactly what `specforge check` reports, in its order: the
    /// environment's, the resolver's, the graph build's, the checks', then
    /// surface conflicts.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.env
            .diagnostics()
            .chain(&self.resolved.diagnostics)
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

    /// The flat view older callers read.
    pub fn into_context(self) -> CompilationContext {
        let diagnostics = self.diagnostics();
        let CompiledProject {
            env,
            resolved,
            graph,
            ..
        } = self;
        let registries = env.registries;
        CompilationContext {
            graph,
            kind_registry: registries.kinds,
            field_registry: registries.fields,
            edge_registry: registries.edges,
            diagnostics,
            resolved,
            extension_rules: registries.rules,
            extension_info: registries.extension_info,
            surface_entries: registries.surfaces,
            manifest_surfaces: registries.manifest_surfaces,
            manifests: registries.manifests,
            spec_root: env.spec_root,
        }
    }
}
