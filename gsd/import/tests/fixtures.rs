//! `import`, in process, on the parity fixtures' `.planning/` folders.

mod common;

use std::collections::BTreeSet;

use serde_json::{Value, json};
use specforge_extension_sdk::PlanOp;

use common::{fixture, import_fixture, payload, planning_facts};

/// Each create of the plan: `(kind, id, file, revision, fields)`.
fn creates(
    out: &specforge_extension_sdk::prelude::CommandOutput,
) -> Vec<(String, String, String, String, Value)> {
    let plan = out.plan.as_ref().expect("import answers a plan");
    plan.ops
        .iter()
        .map(|op| match op {
            PlanOp::Create {
                kind,
                id,
                file,
                revision,
                fields,
                ..
            } => (
                kind.clone(),
                id.clone(),
                file.clone(),
                revision.clone(),
                json!(fields),
            ),
            other => panic!("import answers creates only, not {other:?}"),
        })
        .collect()
}

#[test]
fn taskboard_requirements_reach_one_plan() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let creates = creates(&out);
    let ids: Vec<&str> = creates.iter().map(|(_, id, ..)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "LIST-01", "OOS-01", "STORE-01", "STORE-02", "SYNC-01", "TASK-01", "TASK-02", "TASK-03"
        ]
    );
    for (kind, id, file, revision, fields) in &creates {
        assert_eq!(kind, "requirement", "{id}");
        assert_eq!(file, "spec/gsd/requirements.spec", "{id}");
        assert_eq!(revision, "absent", "{id}");
        assert_eq!(fields["defined"], "2026-03-02", "{id}");
    }
    let fields = |id: &str| {
        creates
            .iter()
            .find(|(_, i, ..)| i == id)
            .map(|(.., f)| f.clone())
            .expect("created")
    };
    assert_eq!(
        fields("TASK-01"),
        json!({"text": "A user can add a task with a title", "category": "Tasks", "tier": "v1", "defined": "2026-03-02"})
    );
    assert_eq!(fields("SYNC-01")["tier"], "v2");
    assert_eq!(fields("SYNC-01")["category"], "Sync");
    assert_eq!(
        fields("OOS-01"),
        json!({
            "text": "Share a board by link",
            "category": "Out of Scope",
            "tier": "out_of_scope",
            "reason": "Needs accounts, which v1 does not have",
            "defined": "2026-03-02",
        })
    );
    let report = payload(&out);
    assert_eq!(report["format"], 1);
    assert_eq!(report["counts"], json!({"requirement": 8}));
    let entries = report["entries"].as_array().expect("entries");
    assert!(
        entries
            .iter()
            .any(|e| e["file"] == ".planning/REQUIREMENTS.md"
                && e["line"] == 39
                && e["outcome"] == "filled"),
        "the minted OOS-01 is reported filled: {entries:#?}"
    );
}

#[test]
fn every_other_planning_file_has_one_entry() {
    let out = import_fixture("taskboard");
    let report = payload(&out);
    let entries = report["entries"].as_array().expect("entries");
    let served: BTreeSet<String> = planning_facts(&fixture("taskboard"))
        .iter()
        .map(|f| f["path"].as_str().expect("a path").to_string())
        .filter(|p| p != ".planning/REQUIREMENTS.md")
        .collect();
    assert_eq!(served.len(), 17);
    for path in &served {
        let of_file: Vec<&Value> = entries.iter().filter(|e| e["file"] == *path).collect();
        assert_eq!(of_file.len(), 1, "{path}: {of_file:#?}");
        assert_eq!(of_file[0]["outcome"], "unmapped", "{path}");
        assert_eq!(of_file[0]["line"], Value::Null, "{path}");
    }
    let reason = |path: &str| {
        entries
            .iter()
            .find(|e| e["file"] == path)
            .map(|e| e["reason"].clone())
            .expect("an entry")
    };
    // The reason is the kit's kinds column for the file's row, or "unknown file".
    assert_eq!(reason(".planning/ROADMAP.md"), "milestone, phase");
    assert_eq!(
        reason(".planning/phases/01-core/01-01-PLAN.md"),
        "plan, task, checkpoint, truth, artifact, key_link, prohibition, disposition, user_setup"
    );
    assert_eq!(reason(".planning/state.json"), "unknown file");
}

#[test]
fn ledger_and_vault_requirements_reach_their_plans() {
    let ids = |name: &str| -> Vec<String> {
        creates(&import_fixture(name))
            .into_iter()
            .map(|(_, id, ..)| id)
            .collect()
    };
    assert_eq!(
        ids("ledger"),
        [
            "LEDG-01", "LEDG-02", "LEDG-03", "LEDG-04", "OOS-01", "REPT-01"
        ]
    );
    assert_eq!(ids("vault"), ["SEC-01", "SEC-02", "SHARE-01"]);
}
