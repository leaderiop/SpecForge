//! What `@gsd/core` declares for the plan kinds (`spec/plan.md`), pinned through the SDK's runtime-free `MockHost`.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use common::{describe, field_of, fields_of};

fn kind_of(kind: &str) -> serde_json::Value {
    describe("entities")["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == kind))
        .unwrap_or_else(|| panic!("no kind {kind}"))
        .clone()
}

#[test]
fn the_plan_kind_declares_the_fields_of_the_model() {
    assert_eq!(
        fields_of("plan"),
        [
            "phase",
            "type",
            "depends_on",
            "files_modified",
            "files_deleted",
            "coupling_justified",
            "requirements",
            "wave0",
            "agent_hint",
            "estimate",
            "objective",
            "purpose",
            "output",
            "context_refs",
            "verification",
            "success_criteria",
            "wave",
            "autonomous",
            "status"
        ]
    );
    for required in ["type", "files_modified", "objective"] {
        assert_eq!(field_of("plan", required)["required"], true, "{required}");
    }
    for computed in ["wave", "autonomous", "status"] {
        assert_eq!(
            field_of("plan", computed)["writer"],
            "computed",
            "{computed}"
        );
    }
    // Paths a plan creates are text, never `file_reference` (the host's E016).
    for paths in ["files_modified", "files_deleted", "context_refs"] {
        assert_ne!(field_of("plan", paths)["file_reference"], true, "{paths}");
    }
}

#[test]
fn the_plan_kind_is_scoped_by_its_phase_which_is_optional() {
    assert_eq!(kind_of("plan")["scoped_by"], "phase");
    let phase = field_of("plan", "phase");
    assert_eq!(phase["immutable"], true);
    assert_ne!(phase["required"], true);
}

#[test]
fn a_coupling_peer_is_a_reference_member() {
    let coupling = field_of("plan", "coupling_justified");
    assert_eq!(coupling["field_type"], "block_list");
    let peer = coupling["members"]
        .as_array()
        .and_then(|m| m.iter().find(|m| m["name"] == "peer"))
        .expect("peer");
    assert_eq!(peer["target_kind"], "plan");
    assert_eq!(peer["required"], true);
    let edges = describe("edges");
    assert!(
        edges["items"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|e| e["label"] == "PlanCouplingJustifiedPeer" && e["source_kind"] == "plan"),
        "{edges}"
    );
}

#[test]
fn the_task_and_checkpoint_kinds_declare_the_fields_of_the_model() {
    assert_eq!(
        fields_of("task"),
        [
            "plan",
            "type",
            "files",
            "read_first",
            "precondition",
            "reversibility",
            "action",
            "verify",
            "verify_text",
            "verify_human",
            "acceptance",
            "done",
            "verify_paths",
            "status"
        ]
    );
    assert_eq!(
        fields_of("checkpoint"),
        [
            "plan",
            "after",
            "kind",
            "gate",
            "what_built",
            "how_to_verify",
            "decision",
            "options",
            "auto_select",
            "instructions",
            "action",
            "verification",
            "resume_signal",
            "answer"
        ]
    );
    for (kind, field) in [
        ("task", "plan"),
        ("task", "type"),
        ("task", "action"),
        ("task", "done"),
        ("checkpoint", "plan"),
        ("checkpoint", "kind"),
        ("checkpoint", "resume_signal"),
    ] {
        assert_eq!(field_of(kind, field)["required"], true, "{kind}.{field}");
    }
    for kind in ["task", "checkpoint"] {
        assert_eq!(kind_of(kind)["scoped_by"], "plan", "{kind}");
        assert_eq!(field_of(kind, "plan")["immutable"], true, "{kind}");
    }
    assert_eq!(field_of("task", "status")["writer"], "computed");
    // A path a task names is text: it may be a file the task creates.
    assert_ne!(field_of("task", "files")["file_reference"], true);
    assert_ne!(field_of("task", "read_first")["file_reference"], true);
}

fn members(field: &serde_json::Value) -> Vec<(String, bool)> {
    field["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| {
            (
                m["name"].as_str().expect("a name").to_string(),
                m["required"] == true,
            )
        })
        .collect()
}

#[test]
fn a_verify_is_a_block_list_of_a_command_and_its_failing_direction() {
    let verify = field_of("task", "verify");
    assert_eq!(verify["field_type"], "block_list");
    assert_eq!(
        members(&verify),
        [
            ("automated".to_string(), true),
            ("fails_when".to_string(), false)
        ]
    );
    let reversibility = field_of("task", "reversibility");
    assert_eq!(
        members(&reversibility),
        [
            ("rating".to_string(), true),
            ("why".to_string(), false),
            ("settled_by".to_string(), false)
        ]
    );
    let edges = describe("edges");
    for label in ["TaskReversibilitySettledBy", "CheckpointAfter", "TaskPlan"] {
        assert!(
            edges["items"]
                .as_array()
                .expect("edges")
                .iter()
                .any(|e| e["label"] == label),
            "{label} in {edges}"
        );
    }
}

#[test]
fn a_checkpoint_answer_is_a_harness_block_with_no_e107_rule() {
    let answer = field_of("checkpoint", "answer");
    assert_eq!(answer["writer"], "harness");
    let names: Vec<String> = members(&answer).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["option", "text", "by", "at"]);
    let rules = describe("validation_rules");
    let on_checkpoint = rules["items"]
        .as_array()
        .expect("rules")
        .iter()
        .filter(|r| r["target_kind"] == "checkpoint" && r["code"].to_string().contains("E107"))
        .count();
    assert_eq!(on_checkpoint, 0, "{rules}");
}

#[test]
fn must_haves_golden() {
    let kinds = ["truth", "artifact", "key_link", "prohibition"];
    let fields: Vec<(&str, Vec<String>)> = kinds.iter().map(|k| (*k, fields_of(k))).collect();
    assert_eq!(
        fields,
        [
            ("truth", vec!["plan", "text", "verification", "criterion"]),
            (
                "artifact",
                vec![
                    "plan",
                    "path",
                    "provides",
                    "min_lines",
                    "exports",
                    "contains"
                ]
            ),
            ("key_link", vec!["plan", "from", "to", "via", "pattern"]),
            (
                "prohibition",
                vec!["plan", "statement", "verification", "test"]
            ),
        ]
        .map(|(k, f)| (k, f.into_iter().map(String::from).collect()))
    );
    let total: usize = fields.iter().map(|(_, f)| f.len()).sum();
    assert_eq!(total, 19);
    for (kind, required) in [
        ("truth", vec!["plan", "text"]),
        ("artifact", vec!["plan", "path", "provides"]),
        ("key_link", vec!["plan", "from", "to", "via"]),
        ("prohibition", vec!["plan", "statement", "verification"]),
    ] {
        assert_eq!(kind_of(kind)["scoped_by"], "plan", "{kind}");
        for field in fields_of(kind) {
            let expected = required.contains(&field.as_str());
            assert_eq!(
                field_of(kind, &field)["required"] == true,
                expected,
                "{kind}.{field}"
            );
            // Every path a must-have names is a file the plan may create.
            assert_ne!(field_of(kind, &field)["file_reference"], true);
        }
        assert_eq!(field_of(kind, "plan")["immutable"], true, "{kind}");
    }
}

#[test]
fn the_disposition_kind_is_append_only_and_every_field_immutable() {
    let disposition = kind_of("disposition");
    assert_eq!(disposition["append_only"], true);
    assert_eq!(disposition["scoped_by"], "plan");
    let fields = disposition["fields"].as_array().expect("fields");
    assert_eq!(fields.len(), 6);
    for field in fields {
        assert_eq!(field["immutable"], true, "{}", field["name"]);
    }
    assert_ne!(kind_of("plan")["append_only"], true);
}

#[test]
fn a_setup_done_is_a_harness_block_and_the_dashboard_a_block_list() {
    assert_eq!(
        fields_of("user_setup"),
        [
            "plan",
            "service",
            "why",
            "env_vars",
            "dashboard",
            "local_dev",
            "done"
        ]
    );
    let done = field_of("user_setup", "done");
    assert_eq!(done["writer"], "harness");
    assert_eq!(done["field_type"], "block");
    assert_eq!(
        field_of("user_setup", "dashboard")["field_type"],
        "block_list"
    );
    assert_eq!(kind_of("user_setup")["scoped_by"], "plan");
}

/// `plan.md` lists 77 fields over its nine kinds. The model fixes: `gap_closure` waits for `verify_gap` and
/// `uat_gap` (r39) and `task.commits` for `commit` (r38), which are 2 fewer, and `task.verify_text`,
/// `task.verify_paths` (gsd/E320), `checkpoint.action` and `checkpoint.verification` are 4 more: 77 - 2 + 4 = 79.
#[test]
fn the_plan_kinds_declare_every_field_of_plan_md() {
    let kinds = [
        "plan",
        "task",
        "checkpoint",
        "truth",
        "artifact",
        "key_link",
        "prohibition",
        "disposition",
        "user_setup",
    ];
    let total: usize = kinds.iter().map(|k| fields_of(k).len()).sum();
    assert_eq!(total, 77 - 2 + 4, "the kinds' fields");
}

/// The host refuses a computed field that two compute passes claim, and one that none does (W021): every `C` field of
/// the model is listed by exactly one compute pass.
#[test]
fn each_computed_field_is_listed_by_one_compute_pass() {
    let passes = describe("passes");
    let mut listed: Vec<String> = passes["items"]
        .as_array()
        .expect("passes")
        .iter()
        .flat_map(|p| p["computes"].as_array().cloned().unwrap_or_default())
        .map(|f| f.as_str().expect("a field").to_string())
        .collect();
    let total = listed.len();
    listed.sort();
    listed.dedup();
    assert_eq!(listed.len(), total, "a field is listed twice: {passes}");
    let mut computed = Vec::new();
    for kind in describe("entities")["items"].as_array().expect("kinds") {
        for field in kind["fields"].as_array().expect("fields") {
            if field["writer"] == "computed" {
                computed.push(format!(
                    "{}.{}",
                    kind["name"].as_str().expect("a kind"),
                    field["name"].as_str().expect("a field")
                ));
            }
        }
    }
    computed.sort();
    assert_eq!(listed, computed);
}
