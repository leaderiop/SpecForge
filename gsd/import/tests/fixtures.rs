//! `import`, in process, on the parity fixtures' `.planning/` folders.

mod common;

use std::collections::BTreeSet;

use gsd_import::table;
use serde_json::{Value, json};
use specforge_extension_sdk::PlanOp;
use specforge_extension_sdk::testing::MockHost;

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
        let of_file: Vec<&Value> = entries
            .iter()
            .filter(|e| e["file"] == *path && e["line"].is_null() && e["section"] == "")
            .collect();
        assert_eq!(of_file.len(), 1, "{path}: {of_file:#?}");
        let (_, fate) = table::fate_of(path.trim_start_matches(".planning/")).expect("a fate");
        assert_eq!(
            of_file[0]["outcome"],
            table::outcome(fate).as_str(),
            "{path}"
        );
        assert_eq!(of_file[0]["line"], Value::Null, "{path}");
    }
    let reason = |path: &str| {
        entries
            .iter()
            .find(|e| e["file"] == path)
            .map(|e| e["reason"].clone())
            .expect("an entry")
    };
    // A Tier 1 file with no reader yet gives the kit's kinds column; the extras their fate's reason.
    assert_eq!(reason(".planning/ROADMAP.md"), "milestone, phase");
    assert_eq!(
        reason(".planning/phases/01-core/01-01-PLAN.md"),
        "plan, task, checkpoint, truth, artifact, key_link, prohibition, disposition, user_setup"
    );
    assert_eq!(
        reason(".planning/state.json"),
        "gsd-tools' derived state (`state json`): STATE.md and the graph compute it"
    );
    assert_eq!(
        reason(".planning/phases/03-listing/.gitkeep"),
        "an empty directory's placeholder"
    );
}

#[test]
fn a_security_file_needs_gsd_security_when_the_project_does_not_load_it() {
    let out = import_fixture("vault");
    let entries = payload(&out)["entries"].clone();
    let entry = entries
        .as_array()
        .expect("entries")
        .iter()
        .find(|e| e["file"] == ".planning/phases/01-core/01-SECURITY.md")
        .cloned()
        .expect("an entry");
    assert_eq!(entry["outcome"], "needs-extension");
    assert!(
        entry["reason"]
            .as_str()
            .is_some_and(|r| r.ends_with("@gsd/security's, which the project does not load")),
        "{entry}"
    );
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

/// Every key of a fixture's `config.json` as a dotted path to a leaf (an empty object is a leaf).
fn config_keys(name: &str) -> Vec<String> {
    fn walk(prefix: &str, value: &Value, out: &mut Vec<String>) {
        match value.as_object() {
            Some(map) if !map.is_empty() => {
                for (key, inner) in map {
                    let path = match prefix {
                        "" => key.clone(),
                        _ => format!("{prefix}.{key}"),
                    };
                    walk(&path, inner, out);
                }
            }
            _ => out.push(prefix.to_string()),
        }
    }
    let text = std::fs::read_to_string(fixture(name).join(".planning/config.json"))
        .expect("the fixture's config.json");
    let mut out = Vec::new();
    walk("", &serde_json::from_str(&text).expect("JSON"), &mut out);
    out
}

/// `(extension, key)` of each settings line of an import's report.
fn printed(report: &Value) -> Vec<(String, String)> {
    report["settings"]
        .as_array()
        .expect("settings")
        .iter()
        .map(|line| {
            let words: Vec<&str> = line.as_str().expect("a line").split(' ').collect();
            assert_eq!(words[..3], ["specforge", "settings", "set"], "{line}");
            (words[3].to_string(), words[4].to_string())
        })
        .collect()
}

#[test]
fn every_config_key_has_a_settings_line_or_an_entry() {
    for name in ["taskboard", "ledger", "vault"] {
        let report = payload(&import_fixture(name));
        let printed = printed(&report);
        let reported: BTreeSet<String> = report["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .filter(|e| e["file"] == ".planning/config.json")
            .map(|e| e["section"].as_str().expect("a section").to_string())
            .collect();
        let keys = config_keys(name);
        assert!(keys.len() > 40, "{name}: {}", keys.len());
        for key in keys {
            assert!(
                printed.iter().any(|(_, k)| *k == key) || reported.contains(&key),
                "{name}: {key} has neither a line nor an entry"
            );
        }
    }
}

/// Whether the declared setting `name` (`a.*` a family of one segment per `*`) names `key`.
fn declares(name: &str, key: &str) -> bool {
    let (names, keys): (Vec<&str>, Vec<&str>) =
        (name.split('.').collect(), key.split('.').collect());
    names.len() == keys.len() && names.iter().zip(&keys).all(|(n, k)| *n == "*" || n == k)
}

#[test]
fn every_printed_core_key_is_declared() {
    let host = MockHost::new(gsd_core::build());
    let described: Value =
        serde_json::from_str(&host.describe_json("settings").expect("settings")).expect("JSON");
    let declared: Vec<String> = described["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["kind"] == "setting")
        .map(|item| item["name"].as_str().expect("a name").to_string())
        .collect();
    let mut checked = 0;
    for name in ["taskboard", "ledger", "vault"] {
        for (extension, key) in printed(&payload(&import_fixture(name))) {
            if extension == "@gsd/core" {
                checked += 1;
                assert!(
                    declared.iter().any(|d| declares(d, &key)),
                    "{name}: @gsd/core declares no setting {key}"
                );
            }
        }
    }
    assert!(checked > 100, "{checked}");
}
