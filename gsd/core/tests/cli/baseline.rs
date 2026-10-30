//! A verification re-run against a git baseline reopens only what changed (r39 T13, ADR 0300): the baseline's scan
//! reads the files the commit holds, so `previous_status` and `previous_score` are what the commit said and an item
//! is compared by its scan state.

use crate::harness::{Project, project, taskboard};

const VERIFICATION: &str = "v1.0/01/01-VERIFICATION";

fn re(p: &Project, id: &str) -> serde_json::Value {
    p.fields_of(id)["re_verification"].clone()
}

#[test]
fn baseline_reverification() {
    let p = taskboard();
    p.commit();
    // On the unchanged tree the baseline says what now says.
    let verification = p.fields_of(VERIFICATION);
    assert_eq!(verification["status"], "passed", "{verification}");
    assert_eq!(verification["score"], "3/3", "{verification}");
    assert_eq!(
        verification["re_verification"],
        serde_json::json!({ "previous_status": "passed", "previous_score": "3/3" }),
        "{verification}"
    );
    // Breaking the artifact reopens exactly it (both occurrences: `edit` replaces only the first).
    p.write(
        "src/cli.js",
        &p.read("src/cli.js").replace("completeTask", "finishTask"),
    );
    let verification = p.fields_of(VERIFICATION);
    assert_eq!(verification["status"], "gaps_found", "{verification}");
    assert_eq!(
        verification["re_verification"],
        serde_json::json!({
            "previous_status": "passed",
            "previous_score": "3/3",
            "regressions": ["v1.0/01/01-02/AR1"]
        }),
        "{verification}"
    );
}

#[test]
fn baseline_reverification_closes_and_regresses_by_scan() {
    let p = project("verify");
    // Two key links get a literal pattern, so the scan verifies or fails them.
    p.edit(
        "spec/plans.spec",
        "  via  \"require\"\n",
        "  via  \"require\"\n  pattern \"add, list\"\n",
    );
    p.edit(
        "spec/plans.spec",
        "  from \"src/complete.js\"\n",
        "  from \"src/complete.js\"\n  pattern \"done: true\"\n",
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    p.commit();
    assert_eq!(
        re(&p, VERIFICATION),
        serde_json::json!({
            "previous_status": "human_needed",
            "previous_score": "1/2",
            "gaps_remaining": [format!("{VERIFICATION}/VG1")]
        })
    );
    // 01-02's artifact and link fail on both sides.
    let complete = p.read("src/complete.js");
    std::fs::remove_file(p.root().join("src/complete.js")).expect("remove src/complete.js");
    p.commit();
    let verification = p.fields_of(VERIFICATION);
    assert_eq!(verification["status"], "gaps_found", "{verification}");
    assert_eq!(
        verification["re_verification"],
        serde_json::json!({
            "previous_status": "gaps_found",
            "previous_score": "1/2",
            "gaps_remaining": [
                "v1.0/01/01-02/AR1",
                "v1.0/01/01-02/KL1",
                format!("{VERIFICATION}/VG1")
            ]
        }),
        "{verification}"
    );
    // The file is back (01-02's link is wired: closed) and 01-01's link stops being wired (a regression).
    p.write("src/complete.js", &complete);
    p.edit("test/tasks.test.js", "add, list", "list, add");
    let verification = p.fields_of(VERIFICATION);
    assert_eq!(verification["status"], "gaps_found", "{verification}");
    assert_eq!(
        verification["re_verification"],
        serde_json::json!({
            "previous_status": "gaps_found",
            "previous_score": "1/2",
            "gaps_closed": ["v1.0/01/01-02/KL1"],
            "gaps_remaining": [format!("{VERIFICATION}/VG1")],
            "regressions": ["v1.0/01/01-01/KL1"]
        }),
        "{verification}"
    );
}
