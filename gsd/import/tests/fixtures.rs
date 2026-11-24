//! `import`, in process, on the parity fixtures' `.planning/` folders.

mod common;

use std::collections::BTreeSet;

use gsd_import::table;
use serde_json::{Value, json};
use specforge_extension_sdk::PlanOp;
use specforge_extension_sdk::prelude::CommandOutput;
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

/// The creates of `kind`.
fn of_kind(
    creates: Vec<(String, String, String, String, Value)>,
    kind: &str,
) -> Vec<(String, String, String, String, Value)> {
    creates.into_iter().filter(|(k, ..)| k == kind).collect()
}

#[test]
fn taskboard_requirements_reach_one_plan() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let creates = of_kind(creates(&out), "requirement");
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
    assert_eq!(report["counts"]["requirement"], 8);
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
        .filter(|p| {
            ![
                ".planning/REQUIREMENTS.md",
                ".planning/PROJECT.md",
                ".planning/ROADMAP.md",
            ]
            .contains(&p.as_str())
                && !p.ends_with("-PLAN.md")
                && !p.ends_with("-SUMMARY.md")
                && !p.ends_with("-CONTEXT.md")
                && !p.ends_with("-DISCUSSION-LOG.md")
                && !p.ends_with("-RESEARCH.md")
        })
        .collect();
    assert_eq!(served.len(), 7);
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
    assert_eq!(
        reason(".planning/phases/01-core/01-VERIFICATION.md"),
        "verification, verify_gap, override, human_item, and verdicts"
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
        of_kind(creates(&import_fixture(name)), "requirement")
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

/// The fields of the create `kind id`, and its file.
fn create_of(out: &CommandOutput, kind: &str, id: &str) -> (String, Value) {
    creates(out)
        .into_iter()
        .find(|(k, i, ..)| k == kind && i == id)
        .map(|(_, _, file, _, fields)| (file, fields))
        .unwrap_or_else(|| panic!("{kind} {id} is created"))
}

#[test]
fn taskboard_project_and_its_decisions_take_the_twins_values() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let (file, project) = create_of(&out, "project", "taskboard");
    assert_eq!(file, "spec/gsd/project.spec");
    // `what` and `context` hold the whole section; the twin keeps the first sentence (ADR 0391 D13).
    assert_eq!(
        project,
        json!({
            "name": "Taskboard",
            "what": "A small board for tracking tasks from the command line. A user adds a task with a title, marks it complete and lists what is left. It is a Node 20 script with no dependencies, built to prove the GSD model in SpecForge.",
            "core_value": "A task is never lost.",
            "context": "Built to prove the GSD model in SpecForge. Small enough that one person reads all of it.",
            "constraints": [{"kind": "tech_stack", "what": "Node 20, no dependencies", "why": "it ships as one script"}],
            "out_of_scope": [{"item": "Sharing a board by link", "why": "needs accounts, which v1 does not have"}],
            "key_decisions": ["D-01", "D-02"],
        })
    );
    // The twin: project.spec and discuss.spec.
    let (file, d1) = create_of(&out, "decision", "D-01");
    assert_eq!(file, "spec/gsd/project.spec");
    assert_eq!(
        d1,
        json!({"text": "ES modules, no build step", "rationale": "`node src/cli.js` must just run", "outcome": "pending", "origin": "project"})
    );
    let (_, d2) = create_of(&out, "decision", "D-02");
    assert_eq!(
        d2,
        json!({"text": "Tasks keep an integer id", "rationale": "Short to type: `taskboard done 3`", "outcome": "pending", "origin": "project"})
    );
    let entries = payload(&out)["entries"].clone();
    let at = |line: u64| -> Vec<(String, String)> {
        entries
            .as_array()
            .expect("entries")
            .iter()
            .filter(|e| e["file"] == ".planning/PROJECT.md" && e["line"] == line)
            .map(|e| {
                (
                    e["section"].as_str().expect("a section").to_string(),
                    e["outcome"].as_str().expect("an outcome").to_string(),
                )
            })
            .collect()
    };
    assert_eq!(
        at(12),
        [("Requirements".to_string(), "redundant".to_string())]
    );
    assert_eq!(at(14), [("Validated".to_string(), "computed".to_string())]);
    assert_eq!(at(16), [("Validated".to_string(), "ignored".to_string())]);
    assert_eq!(at(18), [("Active".to_string(), "computed".to_string())]);
    assert_eq!(
        at(39),
        [("Key Decisions".to_string(), "filled".to_string())]
    );
    assert_eq!(
        at(45),
        [("*Last updated:".to_string(), "computed".to_string())]
    );
}

#[test]
fn ledger_and_vault_projects_read_their_own_shapes() {
    let ledger = import_fixture("ledger");
    let (_, project) = create_of(&ledger, "project", "ledger");
    // A constraint with no ` — ` has no `why`, as the twin writes it.
    assert_eq!(
        project["constraints"],
        json!([{"kind": "tech_stack", "what": "Rust 2021, the standard library only"}])
    );
    assert_eq!(
        project["out_of_scope"],
        json!([{"item": "Currencies other than one", "why": "a ledger is one currency"}])
    );
    let vault = import_fixture("vault");
    let (_, project) = create_of(&vault, "project", "vault");
    assert_eq!(project["key_decisions"], json!(["D-01"]));
    assert_eq!(project.get("out_of_scope"), None);
    let (_, d1) = create_of(&vault, "decision", "D-01");
    assert_eq!(d1["text"], "AES-256-GCM from `node:crypto`");
}

/// The title of the create `kind id`.
fn title_of(out: &CommandOutput, kind: &str, id: &str) -> String {
    let plan = out.plan.as_ref().expect("import answers a plan");
    plan.ops
        .iter()
        .find_map(|op| match op {
            PlanOp::Create {
                kind: k,
                id: i,
                title,
                ..
            } if k == kind && i == id => Some(title.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{kind} {id} is created"))
}

#[test]
fn roadmap_flat_taskboard_gives_the_twins_milestone_and_phases() {
    let out = import_fixture("taskboard");
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert_eq!(
        create_of(&out, "milestone", "v1.0"),
        ("spec/gsd/v1.0.spec".to_string(), json!({}))
    );
    let phase = |id: &str| {
        let (file, fields) = create_of(&out, "phase", id);
        assert_eq!(file, "spec/gsd/v1.0.spec", "{id}");
        (title_of(&out, "phase", id), fields)
    };
    // The twin, v1.0.spec; success criteria verbatim (the twin cuts their backticks: ADR 0391 D13's comparator folds them).
    assert_eq!(
        phase("01"),
        (
            "Core".to_string(),
            json!({
                "milestone": "v1.0",
                "slug": "core",
                "goal": "Tasks can be added and completed",
                "requirements": ["TASK-01", "TASK-02"],
                "success_criteria": ["A user can add a task with a title", "A user can mark a task complete by its id"],
            })
        )
    );
    assert_eq!(
        phase("02"),
        (
            "Persistence".to_string(),
            json!({
                "milestone": "v1.0",
                "slug": "persistence",
                "goal": "Tasks are saved to disk and read back",
                "depends_on": ["v1.0/01"],
                "requirements": ["STORE-01", "STORE-02"],
                "success_criteria": [
                    "Every change to the board is written to `tasks.json`",
                    "A restarted board shows the tasks it had",
                    "A title comes back exactly as it was written",
                ],
            })
        )
    );
    assert_eq!(
        phase("02.1"),
        (
            "Hotfix: titles survive a reload".to_string(),
            json!({
                "milestone": "v1.0",
                "slug": "hotfix-titles-survive-a-reload",
                "goal": "A title is written to disk and read back unchanged",
                "depends_on": ["v1.0/02"],
                "success_criteria": [
                    "A title with a leading space or a line break comes back unchanged",
                    "An empty title is refused",
                ],
            })
        )
    );
    assert_eq!(
        phase("03"),
        (
            "Listing".to_string(),
            json!({
                "milestone": "v1.0",
                "slug": "listing",
                "goal": "Open tasks are listed and filtered",
                "depends_on": ["v1.0/02"],
                "requirements": ["TASK-03", "LIST-01"],
                "success_criteria": [
                    "`taskboard list` shows the open tasks, oldest first",
                    "`taskboard list <word>` shows only tasks whose title has the word",
                ],
            })
        )
    );
    let report = payload(&out);
    let roadmap: Vec<(u64, String, String)> = lines(&out)
        .into_iter()
        .zip(
            report["entries"]
                .as_array()
                .expect("entries")
                .iter()
                .filter(|e| e["line"].is_u64()),
        )
        .filter(|(_, e)| e["file"] == ".planning/ROADMAP.md")
        .map(|((l, o, _), e)| (l, e["section"].as_str().expect("a section").to_string(), o))
        .collect();
    for want in [
        (1, "Roadmap: Taskboard", "redundant"),
        (1, "(no milestone)", "filled"),
        (3, "Overview", "unmapped"),
        (8, "Phases", "computed"),
        (10, "Phase Numbering", "unmapped"),
        (25, "Phase 1: Core", "ignored"),
        (31, "Phase 1: Core", "computed"),
        (
            52,
            "Phase 02.1: Hotfix: titles survive a reload (INSERTED)",
            "computed",
        ),
        (
            55,
            "Phase 02.1: Hotfix: titles survive a reload (INSERTED)",
            "ignored",
        ),
        (
            64,
            "Phase 02.1: Hotfix: titles survive a reload (INSERTED)",
            "ignored",
        ),
        (77, "Progress", "computed"),
    ] {
        assert!(
            roadmap
                .iter()
                .any(|(l, s, o)| (*l, s.as_str(), o.as_str()) == want),
            "{want:?} in {roadmap:#?}"
        );
    }
    assert!(
        report["authored_after_import"]
            .as_array()
            .expect("authored after import")
            .iter()
            .any(|l| l == "milestone v1.0: its title")
    );
    // The Traceability table agrees with the roadmap: no mismatch is reported.
    assert!(!roadmap.iter().any(|(_, s, _)| s == "Traceability"));
}

#[test]
fn roadmap_flat_ledger_and_vault_give_their_phases() {
    for (name, phases) in [("ledger", 3), ("vault", 2)] {
        let out = import_fixture(name);
        assert_eq!(out.exit_code, 0, "{name}: {}", out.stderr);
        assert_eq!(of_kind(creates(&out), "phase").len(), phases, "{name}");
        assert_eq!(of_kind(creates(&out), "milestone").len(), 1, "{name}");
    }
}

/// `import` of one served file on an empty graph.
fn import_one(path: &str, text: &str) -> CommandOutput {
    common::import(
        &specforge_extension_sdk::prelude::CommandGraph::default(),
        &common::feed(
            vec![common::fact(path, text)],
            false,
            Some(&common::manifest(&["@gsd/core"])),
        ),
    )
}

/// `(line, outcome, reason)` of each entry of `out`.
fn lines(out: &CommandOutput) -> Vec<(u64, String, String)> {
    payload(out)["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter_map(|e| {
            Some((
                e["line"].as_u64()?,
                e["outcome"].as_str()?.to_string(),
                e["reason"].as_str()?.to_string(),
            ))
        })
        .collect()
}

#[test]
fn a_project_reports_what_it_cannot_map() {
    let text = "# Odd Project\n\n## What This Is\n\nA thing.\n\n## Core Value\n\nIt works.\n\n## Requirements\n\n### Out of Scope\n\n- Teleporting\n\n## Context\n\nNone.\n\n## Constraints\n\n- **Mood**: calm — it helps\n- No label at all\n\n## Key Decisions\n\n| Decision | Rationale | Outcome |\n|---|---|---|\n| Use X | because | Maybe |\n\n## Glossary\n\n### Terms\n";
    let out = import_one(".planning/PROJECT.md", text);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let (_, project) = create_of(&out, "project", "odd-project");
    assert_eq!(
        project["constraints"],
        json!([
            {"kind": "other", "what": "Mood: calm", "why": "it helps"},
            {"kind": "other", "what": "No label at all"}
        ])
    );
    assert_eq!(project.get("out_of_scope"), None);
    let (_, d1) = create_of(&out, "decision", "D-01");
    assert_eq!(d1.get("outcome"), None);
    let lines = lines(&out);
    let has = |line: u64, outcome: &str, says: &str| {
        assert!(
            lines
                .iter()
                .any(|(l, o, r)| *l == line && o == outcome && r.contains(says)),
            "{line} {outcome} {says}: {lines:#?}"
        );
    };
    has(15, "unmapped", "needs a reason");
    has(23, "filled", "keeps its words in `what`");
    has(24, "filled", "keeps its words in `what`");
    has(30, "unmapped", "none of Good, Revisit, Pending");
    has(32, "unmapped", "no row of the PROJECT.md table");
    // `### Terms` sits under the reported `## Glossary`: not reported twice.
    assert!(!lines.iter().any(|(l, ..)| *l == 34), "{lines:#?}");
    // A name the id grammar refuses refuses the project, not its decisions.
    let out = import_one(
        ".planning/PROJECT.md",
        &text.replace("# Odd Project", "# 日本"),
    );
    assert!(creates(&out).iter().all(|(k, ..)| k != "project"));
    assert!(creates(&out).iter().any(|(k, ..)| k == "decision"));
    assert!(lines_of(&out, 1, "gives no id"));
}

fn lines_of(out: &CommandOutput, line: u64, says: &str) -> bool {
    lines(out)
        .iter()
        .any(|(l, _, r)| *l == line && r.contains(says))
}

#[test]
fn taskboard_state_maps_nothing_and_reports_each_section() {
    let out = import_fixture("taskboard");
    let entries = payload(&out)["entries"].clone();
    let of_state: Vec<(Value, String, String)> = entries
        .as_array()
        .expect("entries")
        .iter()
        .filter(|e| e["file"] == ".planning/STATE.md")
        .map(|e| {
            (
                e["line"].clone(),
                e["section"].as_str().expect("a section").to_string(),
                e["outcome"].as_str().expect("an outcome").to_string(),
            )
        })
        .collect();
    let want: Vec<(Value, String, String)> = [
        (Value::Null, "", "runtime"),
        (json!(1), "---", "computed"),
        (json!(19), "Project State", "unmapped"),
        (json!(21), "Project Reference", "unmapped"),
        (json!(28), "Current Position", "computed"),
        (json!(37), "Performance Metrics", "unmapped"),
        (json!(44), "Accumulated Context", "unmapped"),
        (json!(46), "Decisions", "unmapped"),
        (json!(50), "Pending Todos", "unmapped"),
        (json!(56), "Blockers/Concerns", "ignored"),
        (json!(58), "Session Continuity", "runtime"),
    ]
    .into_iter()
    .map(|(l, s, o)| (l, s.to_string(), o.to_string()))
    .collect();
    assert_eq!(of_state, want);
}

#[test]
fn a_state_blocker_is_reported_and_a_broken_frontmatter_named() {
    let state = "---\nstatus: [open\n---\n# State\n\n### Blockers/Concerns\n\n- The API key is not set\n- None yet.\n";
    let out = import_one(".planning/STATE.md", state);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    assert!(
        out.plan.as_ref().is_none_or(|p| p.ops.is_empty()),
        "STATE.md writes nothing"
    );
    let entries = payload(&out)["entries"].clone();
    let line = |n: u64| {
        entries
            .as_array()
            .expect("entries")
            .iter()
            .find(|e| e["line"] == n)
            .cloned()
            .unwrap_or_else(|| panic!("an entry at {n}: {entries:#?}"))
    };
    assert_eq!(line(2)["outcome"], "unmapped");
    assert!(
        line(2)["reason"]
            .as_str()
            .is_some_and(|r| r.starts_with("frontmatter not read")),
        "{}",
        line(2)
    );
    assert_eq!(line(8)["outcome"], "unmapped");
    assert_eq!(line(8)["reason"], table::LATER);
    assert_eq!(line(9)["outcome"], "ignored");
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
