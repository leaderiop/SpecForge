//! Guard passes (ADR 0289): passes an extension declares with `phase: "guard"` run only when a write is judged, never
//! on a compile, so a hand edit is never guarded. They read the project the write would leave, and the project before
//! it as `previous`; what they report is a finding of the edited project, so a guard error is a new error and refuses
//! the write (E080).

use crate::Environment;
use crate::baseline::Baseline;
use crate::passes::{PassContext, run_extension_passes};
use specforge_common::Diagnostic;
use specforge_registry::{Disk, EntitySnapshot};
use specforge_wasm::WasmRuntime;

/// Run `env`'s guard passes over the entity snapshot `entities` of the edited project, `previous` being the project
/// before the write, restricted to the kinds each pass names. A pass that fails is E028; the others still run.
pub(crate) fn run(
    env: &Environment,
    entities: &EntitySnapshot,
    runtime: &dyn WasmRuntime,
    previous: &Baseline,
    judgements: &crate::judged::Judgements,
    disk: &dyn Disk,
) -> Vec<Diagnostic> {
    if env.registries().guard_passes().next().is_none() {
        return Vec::new();
    }
    let context = PassContext {
        previous: Some(previous),
        judgements: Some(judgements),
        files: Some(env.file_source(disk)),
        ..PassContext::of(entities)
    };
    run_extension_passes(
        env.registries().guard_passes(),
        &context,
        runtime,
        env.registries().codes(),
    )
    .into_iter()
    .flat_map(|report| report.findings)
    .collect()
}
