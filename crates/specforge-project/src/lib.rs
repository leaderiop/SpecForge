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
//!   definition, what `specforge check` reports.
//!
//! [`CompilationContext`] is the flat view older callers read; it is built
//! from a compiled project with [`CompiledProject::into_context`].

use std::path::{Path, PathBuf};

use specforge_common::{Diagnostic, ProjectConfig, load_project_config};
use specforge_emitter::compile::{GraphChecks, check_graph, load_extensions};
use specforge_graph::{Graph, GraphConfig, build_graph_with_config};
use specforge_registry::{RegistryBuild, build_registries};
use specforge_resolver::{ResolvedProject, resolve_project};
use specforge_wasm::WasmRuntime;

pub use specforge_emitter::compile::CompilationContext;

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
    /// Extension loading diagnostics (E028, manifest validation, peer
    /// consistency), in load order.
    pub load_diagnostics: Vec<Diagnostic>,
}

impl Environment {
    /// Read the project's config and load its extensions through `runtime`
    /// (none without one), then build the registries from them.
    pub fn load(root: &Path, runtime: Option<&dyn WasmRuntime>) -> Self {
        let config = load_project_config(root);
        let mut load_diagnostics = Vec::new();
        let manifests = match runtime {
            Some(runtime) => load_extensions(&config.extensions, runtime, &mut load_diagnostics),
            None => Vec::new(),
        };
        let registries = build_registries(manifests);
        let spec_root = match &config.spec_root {
            Some(spec_root) => root.join(spec_root),
            None => root.to_path_buf(),
        };
        Environment {
            root: root.to_path_buf(),
            config,
            spec_root,
            registries,
            load_diagnostics,
        }
    }

    /// The inputs every graph of this project is built with.
    pub fn graph_config(&self) -> GraphConfig {
        specforge_emitter::compile::graph_config(&self.registries)
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

    /// Discover, parse and resolve the project's `.spec` files.
    pub fn resolve(&self) -> ResolvedProject {
        resolve_project(&self.spec_root)
    }
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
    /// registry checks and the extensions' rules.
    pub check_diagnostics: Vec<Diagnostic>,
}

impl CompiledProject {
    /// Compile the project at `root`, running its extensions in `runtime`.
    /// Without a runtime no extension is loaded.
    pub fn compile(root: &Path, runtime: Option<&dyn WasmRuntime>) -> Self {
        let env = Environment::load(root, runtime);
        let resolved = env.resolve();
        let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
        let (graph, graph_diagnostics) = build_graph_with_config(&spec_files, &env.graph_config());
        let check_diagnostics = check_graph(&graph, &env.checks(runtime));
        CompiledProject {
            env,
            resolved,
            graph,
            graph_diagnostics,
            check_diagnostics,
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

    /// The flat view older callers read.
    pub fn into_context(self) -> CompilationContext {
        let diagnostics = self.diagnostics();
        let graph_config = self.env.graph_config();
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
            graph_config,
        }
    }
}
