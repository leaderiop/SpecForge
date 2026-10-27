//! Check-phase extension passes: compiler passes an extension declares with
//! `phase: "check"` run with every compile, after the graph checks, and
//! what they report is the compile's (behavior `run_check_phase_passes`).
//! They run through the one pass loop
//! ([`crate::passes::run_extension_passes`]); a pass's summary is not read.

use crate::passes::{PassContext, run_extension_passes};
use specforge_common::Diagnostic;
use specforge_registry::EntitySnapshot;
use specforge_wasm::WasmRuntime;

use crate::Environment;
use crate::baseline::Baseline;

/// Run `env`'s check passes over the entity snapshot `entities`, handing them the baseline (moves applied) as
/// `previous`, restricted to the kinds each names. A pass that fails (it traps, or answers output that does not
/// parse) is E028; the others still run.
pub(crate) fn run(
    env: &Environment,
    entities: &EntitySnapshot,
    runtime: &dyn WasmRuntime,
    baseline: Option<&Baseline>,
) -> Vec<Diagnostic> {
    if env.registries().check_passes().next().is_none() {
        return Vec::new();
    }
    let context = PassContext {
        previous: baseline,
        ..PassContext::of(entities)
    };
    run_extension_passes(
        env.registries().check_passes(),
        &context,
        runtime,
        env.registries().codes(),
    )
    .into_iter()
    .flat_map(|report| report.findings)
    .collect()
}
