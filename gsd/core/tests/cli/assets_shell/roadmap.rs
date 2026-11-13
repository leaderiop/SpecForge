//! The roadmapper's write step, and the roadmap edit it sends: the template's `json` fence, filled and sent as one MCP
//! `specforge.edit` call on a taskboard copy.

use serde_json::{Value, json};

use super::{asset, block, mcp_edit};
use crate::harness::taskboard;

/// The roadmapper's write step (`gsd-roadmapper.md`).
fn write_step() -> String {
    block(
        &asset("agents/gsd-roadmapper.md"),
        "## Step 7: Write the Roadmap Immediately",
        "## Step 8",
    )
}

/// The values the model writes into the roadmap edit: milestone v1.1, phases 04 and 05, one new requirement each.
const FILL: &[(&str, &str)] = &[
    ("{the milestone version, e.g. v1.1}", "v1.1"),
    ("{the milestone name}", "Sync"),
    ("{the milestone goal}", "Two machines keep one board"),
    ("{the first phase's requirement, e.g. SYNC-02}", "SYNC-02"),
    ("{the second phase's requirement, e.g. SYNC-03}", "SYNC-03"),
    (
        "{the first requirement's text}",
        "A board exports to a file",
    ),
    ("{the second requirement's text}", "A board imports a file"),
    ("{the requirements' category}", "Sync"),
    ("{the first phase's number, e.g. 04}", "04"),
    ("{the second phase's number, e.g. 05}", "05"),
    ("{the first phase's name}", "Export"),
    ("{the second phase's name}", "Import"),
    ("{the first phase's slug}", "export"),
    ("{the second phase's slug}", "import"),
    ("{the first phase's goal}", "A board can be exported"),
    ("{the second phase's goal}", "A board can be imported"),
    (
        "{an observable behavior the first phase makes true}",
        "taskboard export writes the board to a file",
    ),
    (
        "{another observable behavior of the first phase}",
        "an exported file holds every task",
    ),
    (
        "{an observable behavior the second phase makes true}",
        "taskboard import reads a board from a file",
    ),
    (
        "{another observable behavior of the second phase}",
        "an import keeps the tasks already there",
    ),
    ("{standard or mvp}", "standard"),
];

/// The roadmap edit of `templates/roadmap.md` (its `json` fence), filled with [`FILL`].
pub(super) fn roadmap_edit() -> Value {
    let text = asset("gsd-core/templates/roadmap.md");
    let mut edit = gsd_kit::markdown::fences(&text)
        .into_iter()
        .find(|f| f.language == "json")
        .expect("the roadmap edit's json fence")
        .text;
    for (key, value) in FILL {
        assert!(edit.contains(key), "no {key} in the roadmap edit");
        edit = edit.replace(key, value);
    }
    assert!(!edit.contains("\"{"), "a value left unfilled: {edit}");
    serde_json::from_str(&edit).unwrap_or_else(|e| panic!("the roadmap edit is JSON ({e}): {edit}"))
}

/// The step writes the roadmap in one `specforge.edit` call, never phase by phase. On a taskboard copy the template's
/// roadmap edit, filled, writes milestone v1.1 with phases 04 and 05 (05 depending on 04) and the new v1 requirements
/// SYNC-02 and SYNC-03, one per phase, in one call; `check` gains no finding and reports no `gsd/E101`. The same edit
/// with SYNC-03 in no phase is refused whole, and nothing is written.
#[test]
fn the_roadmap_write_step() {
    let step = write_step();
    assert!(
        step.contains("**one** MCP `specforge.edit` call")
            && !step.contains("`specforge new phase`"),
        "{step}"
    );
    let p = taskboard();
    let before = p.codes();
    let mut orphan = roadmap_edit();
    let phases = orphan.as_array_mut().expect("a list of edits");
    let second = phases
        .iter_mut()
        .find(|e| e["kind"] == "phase" && e["id"] == "05")
        .expect("phase 05");
    second["fields"]["requirements"] = json!([]);
    let refused = mcp_edit(&p, &orphan);
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(refused.to_string().contains("gsd/E101"), "{refused}");
    assert!(p.nodes_of("milestone").iter().all(|m| m["id"] != "v1.1"));
    assert!(
        p.nodes_of("requirement")
            .iter()
            .all(|r| r["id"] != "SYNC-02")
    );
    let written = mcp_edit(&p, &roadmap_edit());
    assert_ne!(written["isError"], true, "{written}");
    assert_eq!(p.fields_of("v1.1/04")["requirements"], json!(["SYNC-02"]));
    assert_eq!(p.fields_of("v1.1/05")["requirements"], json!(["SYNC-03"]));
    assert_eq!(p.fields_of("v1.1/05")["depends_on"], json!(["v1.1/04"]));
    assert_eq!(p.fields_of("SYNC-03")["tier"], "v1");
    let mut after = p.codes();
    for code in &before {
        if let Some(at) = after.iter().position(|c| c == code) {
            after.remove(at);
        }
    }
    assert!(after.is_empty(), "added: {after:?}");
    assert!(p.codes().iter().all(|c| c != "gsd/E101"));
}

/// A revision that moves a requirement to another phase changes both phases in one `specforge.edit` call: a `link`
/// alone leaves the requirement in two phases, and `gsd/E101` refuses it.
#[test]
fn a_revision_moves_a_requirement_in_one_edit() {
    let roadmapper = asset("agents/gsd-roadmapper.md");
    let revision = block(
        &roadmapper,
        "## Step 9: Handle Revision",
        "</execution_flow>",
    );
    let p = taskboard();
    let linked = p.run(&["link", "v1.0/03", "requirements", "TASK-01"]);
    assert_ne!(linked.code, 0, "{}", linked.all());
    assert!(linked.all().contains("gsd/E101"), "{}", linked.all());
    for text in [revision, asset("agents/gsd-roadmapper.compact.md")] {
        assert!(
            text.contains("in one `specforge.edit` call") && text.contains("gsd/E101"),
            "{text}"
        );
    }
    let moved = mcp_edit(
        &p,
        &json!([
            {"op": "set", "entity_id": "v1.0/01", "field": "requirements", "value": ["TASK-02"]},
            {"op": "set", "entity_id": "v1.0/03", "field": "requirements", "value": ["TASK-03", "LIST-01", "TASK-01"]},
        ]),
    );
    assert_ne!(moved["isError"], true, "{moved}");
    assert_eq!(p.fields_of("v1.0/01")["requirements"], json!(["TASK-02"]));
    assert!(p.codes().iter().all(|c| c != "gsd/E101"));
}

/// ingest-docs' merge writes its new v1 requirements with the phases that hold them, in the roadmap edit: a v1
/// requirement recorded alone in a project that has phases is in no phase, and `gsd/E101` refuses it.
#[test]
fn the_ingest_merge_writes_requirements_with_their_phases() {
    let merge = block(
        &asset("gsd-core/workflows/ingest-docs.md"),
        "<step name=\"route_merge_mode\">",
        "</step>",
    );
    let p = taskboard();
    let alone = p.run(&[
        "new",
        "requirement",
        "SYNC-09",
        "A board syncs",
        "--in",
        "spec/requirements.spec",
        "--text",
        "A board syncs",
        "--category",
        "Sync",
        "--tier",
        "v1",
    ]);
    assert_ne!(alone.code, 0, "{}", alone.all());
    assert!(alone.all().contains("gsd/E101"), "{}", alone.all());
    assert!(
        merge.contains("templates/roadmap.md")
            && merge.contains("gsd/E101")
            && !merge.contains("`specforge new phase`"),
        "{merge}"
    );
}
