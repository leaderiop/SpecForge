//! Check-phase extension passes: compiler passes an extension declares with
//! `phase: "check"` run with every compile, after the graph checks, and
//! what they report is the compile's (behavior `run_check_phase_passes`).

use crate::passes::{AnalysisContext, call_pass, declared_passes, is_check_phase, pass_input};
use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_registry::ManifestV2;
use specforge_wasm::WasmRuntime;

use crate::Environment;
use crate::build_cache::BuildCache;

/// One check-phase pass: the extension that declares it and its name (its
/// export is `__pass_<name>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckPass {
    pub extension: String,
    pub name: String,
}

/// The check-phase passes the loaded extensions declare, in the order they
/// run: extension by extension in load order, each extension's in the
/// order its after/before constraints give.
pub(crate) fn declared(manifests: &[ManifestV2], runtime: &dyn WasmRuntime) -> Vec<CheckPass> {
    manifests
        .iter()
        .flat_map(|manifest| {
            declared_passes(runtime, &manifest.name)
                .into_iter()
                .filter(is_check_phase)
                .map(|pass| CheckPass {
                    extension: manifest.name.clone(),
                    name: pass.name,
                })
        })
        .collect()
}

/// Run `env`'s check passes over `graph`, handing them the build cache as
/// `previous` when the project has one (W144 when it is invalid). A pass
/// that traps or answers output that does not parse is E028; the others
/// still run.
pub(crate) fn run(env: &Environment, graph: &Graph, runtime: &dyn WasmRuntime) -> Vec<Diagnostic> {
    if env.check_passes.is_empty() {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    // The build cache is the check passes' declared input: read on every
    // compile that runs one, so a cache just written is seen.
    let previous = match BuildCache::read(&env.root) {
        Ok(previous) => previous,
        Err(invalid) => {
            diagnostics.push(invalid);
            None
        }
    };
    let registries = &env.registries;
    let mut input = pass_input(&AnalysisContext {
        graph,
        kind_registry: &registries.kinds,
        field_registry: &registries.fields,
        rules: &registries.rules,
        project_root: Some(&env.root),
        // A compile has no test results and no proof.
        test_results: None,
        proved_claims: None,
    });
    if let (Some(previous), Some(fields)) = (previous, input.as_object_mut()) {
        fields.insert(
            "previous".to_string(),
            serde_json::json!({ "statuses": previous.statuses }),
        );
    }
    let Ok(input) = serde_json::to_vec(&input) else {
        return diagnostics;
    };
    for pass in &env.check_passes {
        match call_pass(runtime, &pass.extension, &pass.name, &input, graph) {
            Ok((findings, _summary)) => diagnostics.extend(findings),
            Err(error) => diagnostics.push(
                Diagnostic::error(
                    "E028",
                    format!(
                        "extension pass '{}:{}' {error}",
                        pass.extension, pass.name
                    ),
                )
                .with_suggestion(format!(
                    "report the failure to the author of '{}', or check it is installed and up to date",
                    pass.extension
                )),
            ),
        }
    }
    diagnostics
}
