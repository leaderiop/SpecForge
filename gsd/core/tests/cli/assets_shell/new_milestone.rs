//! new-milestone's requirements step, run as the workflow prescribes it on a project that has a roadmap.

use super::{asset, block};
use crate::harness::taskboard;

/// new-milestone's "Define Requirements" step.
fn requirements_step() -> String {
    block(
        &asset("gsd-core/workflows/new-milestone.md"),
        "## 9. Define Requirements",
        "## 10. Create Roadmap",
    )
}

/// Pins today: the step tells the agent to generate REQUIREMENTS.md, and the write it implies for a v1 requirement
/// is refused in a project whose milestone has phases: `gsd/E101` binds every v1 requirement once a phase exists, and
/// the new one is in none until the roadmap step after it.
#[test]
fn the_requirements_step() {
    let step = requirements_step();
    assert!(step.contains("Generate REQUIREMENTS.md"), "{step}");
    let p = taskboard();
    let run = p.run(&[
        "new",
        "requirement",
        "SYNC-02",
        "Sync two boards",
        "--in",
        "spec/requirements.spec",
        "--text",
        "Two boards stay in sync",
        "--category",
        "Sync",
        "--tier",
        "v1",
    ]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.stderr.contains("gsd/E101") && run.stderr.contains("SYNC-02"),
        "{}",
        run.all()
    );
}
