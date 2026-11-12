//! The edge probe of spec-phase's Step 5.5 and plan-phase's spec-less gate, run as the assets write them.

use super::{Shell, asset, fence_after};
use crate::harness::project;

/// Step 5.5's fence on the discuss project, whose phase 02 has a spec and a locked requirement.
// Pin: the fence still resolves upstream's compiled edge-probe.cjs from install dirs SpecForge never creates, so it
// stops. The fix that runs `specforge gsd edge-probe` flips this test.
#[test]
fn the_spec_phase_edge_probe_runs_in_a_project() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = project("discuss");
    let fence = fence_after(
        &asset("gsd-core/workflows/spec-phase.md"),
        "**Runtime coverage compute",
    );
    let (_, err, code) = sh.run_in(
        p.root(),
        &format!("export HOME=\"$PWD/.home\"; unset RUNTIME_DIR; PHASE_ID=v1.0/02\n{fence}"),
    );
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("edge-probe.cjs not found"), "{err}");
}
