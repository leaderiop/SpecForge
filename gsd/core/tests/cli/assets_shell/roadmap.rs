//! The roadmapper's write step, and the roadmap edit it sends: the template's `json` fence, filled and sent as one MCP
//! `specforge.edit` call on a taskboard copy.

use serde_json::{Value, json};

use super::{asset, block};
use crate::harness::{Project, taskboard};

/// The roadmapper's write step (`gsd-roadmapper.md`).
fn write_step() -> String {
    block(
        &asset("agents/gsd-roadmapper.md"),
        "## Step 7: Write the Roadmap Immediately",
        "## Step 8",
    )
}

/// Sends `edits` to `specforge mcp .` in `p` as one `specforge.edit` call: the call's result.
fn mcp_edit(p: &Project, edits: &Value) -> Value {
    let messages = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "specforge.edit", "arguments": {"edits": edits}}}),
    ];
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
    let run = p.run_stdin(&["mcp", "."], &input);
    run.stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|m| m["id"] == 2)
        .unwrap_or_else(|| panic!("no answer to the call: {}", run.all()))["result"]
        .clone()
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
fn roadmap_edit() -> Value {
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
