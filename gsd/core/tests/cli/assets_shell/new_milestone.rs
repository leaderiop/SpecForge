//! new-milestone's milestone and requirements steps, run as the assets hold them on a project that has a roadmap,
//! under bash with only `specforge` on the `PATH`.

use super::{Shell, asset, block, fence_after, typed};
use crate::harness::taskboard;

/// new-milestone's "Define Requirements" step.
fn requirements_step() -> String {
    block(
        &asset("gsd-core/workflows/new-milestone.md"),
        "## 9. Define Requirements",
        "## 10. Create Roadmap",
    )
}

/// Step 9's requirement fence, filled for one requirement of `tier`; `--reason` left out for a v2 one, as the step
/// says.
fn requirement(id: &str, text: &str, tier: &str) -> String {
    let fence = fence_after(&requirements_step(), "**Record the requirements:**");
    let fence = if tier == "v2" {
        fence.replace(r#" --reason "$REASON""#, "")
    } else {
        fence
    };
    typed::filled(
        &fence,
        &[
            ("REQ_ID", id),
            ("TEXT", text),
            ("CATEGORY", "Sync"),
            ("TIER", tier),
            ("REASON", "Accounts are out of scope"),
        ],
    )
}

/// On a taskboard copy, step 4's milestone fence records v1.1 with its goal and two target features, and step 9's
/// fence records a v2 and an out-of-scope requirement. No fence of step 9 records a v1 requirement: the step hands
/// them to the roadmap edit, since `gsd/E101` refuses one recorded alone in a project that has phases. The check
/// reports nothing new.
#[test]
fn the_requirements_step() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let step = requirements_step();
    assert!(
        step.contains("`<requirements>` block") && step.contains("one roadmap edit"),
        "{step}"
    );
    for fence in gsd_kit::markdown::fences(&step) {
        assert!(
            !(fence.text.contains("new requirement") && fence.text.contains("--tier v1")),
            "{}",
            fence.text
        );
    }
    let milestone = typed::filled(
        &fence_after(
            &asset("gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md"),
            "In flat mode",
        ),
        &[
            ("VERSION", "v1.1"),
            ("NAME", "Sync"),
            ("GOAL", "Two machines keep one board"),
            ("FEATURE", "Export a board"),
        ],
    )
    .replace(
        r#"--target_features "$FEATURE""#,
        r#"--target_features "$FEATURE" --target_features "Import a board""#,
    );
    let p = taskboard();
    let mut added = Vec::new();
    for (name, script) in [
        ("milestone", milestone),
        (
            "v2",
            requirement("SYNC-02", "A board merges two edits", "v2"),
        ),
        (
            "out of scope",
            requirement(
                "SYNC-03",
                "A board syncs through an account",
                "out_of_scope",
            ),
        ),
    ] {
        let run = typed::apply(&sh, &p, &script);
        assert_eq!(run.code, 0, "{name}: {}{}", run.out, run.err);
        added.extend(run.added);
    }
    assert!(added.is_empty(), "{added:?}");
    let v11 = p.fields_of("v1.1");
    assert_eq!(v11["goal"], "Two machines keep one board", "{v11}");
    assert_eq!(
        v11["target_features"],
        serde_json::json!(["Export a board", "Import a board"]),
        "{v11}"
    );
    assert_eq!(p.fields_of("SYNC-02")["tier"], "v2");
    assert_eq!(
        p.fields_of("SYNC-03")["reason"],
        "Accounts are out of scope"
    );
}

/// With a workstream, step 4 records the milestone scoped by it (`api/v1.1`), and the roadmap edit step 10 sends
/// without a milestone op then writes the phases under it.
#[test]
fn a_workstream_milestone_is_recorded() {
    let step = asset("gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md");
    let p = taskboard();
    let run = p.run(&[
        "new",
        "workstream",
        "api",
        "API",
        "--in",
        "spec/api.spec",
        "--purpose",
        "The public API",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let mut edit = super::roadmap::roadmap_edit();
    let ops = edit.as_array_mut().expect("a list of edits");
    ops.retain(|op| op["kind"] != "milestone");
    for op in ops.iter_mut().filter(|op| op["kind"] == "phase") {
        op["fields"]["milestone"] = serde_json::json!("api/v1.1");
    }
    // pinned: Part A is skipped in a workstream, so nothing records the milestone the phases name
    assert!(step.contains("Skip Part A if `WS`"), "{step}");
    let refused = super::mcp_edit(&p, &edit);
    assert_eq!(refused["isError"], true, "{refused}");
}
