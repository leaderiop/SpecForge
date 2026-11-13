//! The roadmapper's write step, run as the agent body prescribes it on a project with requirements and no roadmap yet.

use super::{asset, block};
use crate::harness::project;

/// The roadmapper's write step (`gsd-roadmapper.md`).
fn write_step() -> String {
    block(
        &asset("agents/gsd-roadmapper.md"),
        "## Step 7: Write Files Immediately",
        "## Step 8",
    )
}

/// Pins today: the step writes the roadmap one entity at a time (`specforge new`), and the first phase of a roadmap
/// over two v1 requirements is refused whole, since `gsd/E101` waits for the first phase and then binds every v1
/// requirement: no roadmap of two phases can be written.
#[test]
fn the_roadmap_write_step() {
    let step = write_step();
    assert!(
        step.contains("`specforge new`") && !step.contains("specforge.edit"),
        "{step}"
    );
    let p = project("empty");
    let setup: [&[&str]; 4] = [
        &[
            "new",
            "project",
            "demo",
            "Demo",
            "--in",
            "spec/project.spec",
            "--name",
            "Demo",
            "--what",
            "A demo board",
            "--core_value",
            "Tasks are never lost",
            "--context",
            "A greenfield project",
            "--constraints",
            "[]",
        ],
        &[
            "new",
            "requirement",
            "AUTH-01",
            "Sign in",
            "--in",
            "spec/requirements.spec",
            "--text",
            "A user can sign in",
            "--category",
            "Auth",
            "--tier",
            "v1",
        ],
        &[
            "new",
            "requirement",
            "AUTH-02",
            "Sign out",
            "--in",
            "spec/requirements.spec",
            "--text",
            "A user can sign out",
            "--category",
            "Auth",
            "--tier",
            "v1",
        ],
        &[
            "new",
            "milestone",
            "v1.0",
            "MVP",
            "--in",
            "spec/v1.0.spec",
            "--name",
            "MVP",
        ],
    ];
    for args in setup {
        let run = p.run(args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.all());
    }
    let phase = p.run(&[
        "new",
        "phase",
        "01",
        "Sign in",
        "--in",
        "spec/v1.0.spec",
        "--milestone",
        "v1.0",
        "--name",
        "Sign in",
        "--slug",
        "sign-in",
        "--goal",
        "Users can sign in",
        "--requirements",
        "AUTH-01",
        "--success_criteria",
        "A user signs in",
        "--success_criteria",
        "A wrong password is refused",
    ]);
    assert_ne!(phase.code, 0, "{}", phase.all());
    assert!(
        phase.stderr.contains("gsd/E101") && phase.stderr.contains("AUTH-02"),
        "{}",
        phase.all()
    );
    assert!(p.nodes_of("phase").is_empty(), "nothing was written");
}
