//! The GSD-shaped kinds and judged rules the K7 tests share (ADR 0290), served in process and written as the wire
//! carries them, so an attribute the host does not read yet is dropped by the descriptor and the test that needs it
//! fails, not the fixture.
//!
//! `@gsd/core` (its codes print `gsd/..`) declares the plan kinds (`plan` and the kinds that belong to one), the
//! decisions, contexts and research of a phase, the spec kinds (`phase_spec`, `locked_req`, `interview`), `package_audit`
//! and `milestone_audit`, and the eight judged rules of the model with every selector of its inputs table:
//! the plan check (W311, W313, W315, W316), E501 on a truth, an artifact and a key link, E202, W203, W205. `scores` is
//! bound by E202's `values`, `legitimacy` by W205's `outcome_fields`.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_wasm::testing::InProcessRuntime;

use super::TestProject;

/// The extension that declares the kinds and the rules.
pub const GSD_JUDGED: &str = "@gsd/core";

/// What the plan-check rules read of a plan (`rules.md`, K7 D8).
pub const PLAN_CHECK_INPUTS: [(&str, &str); 11] = [
    ("plan", "self"),
    (
        "body",
        "self<task|checkpoint|truth|artifact|key_link|prohibition|user_setup.plan",
    ),
    ("phase", "self.phase"),
    ("requirements", "self.requirements"),
    ("decisions", "self.phase<decision.phase"),
    ("project_decisions", "decision"),
    ("context", "self.phase<context|deferred_idea.phase"),
    ("research", "self.phase<research.phase"),
    (
        "research_items",
        "self.phase<research.phase<finding|stack_choice|pattern|pitfall|dont_hand_roll|open_question.research",
    ),
    ("siblings", "self.phase<plan.phase"),
    ("instructions", "file(\"CLAUDE.md\", \"AGENTS.md\")"),
];

fn reference(name: &str, target: &str) -> Value {
    json!({"name": name, "field_type": "reference", "target_kind": target})
}

fn references(name: &str, target: &str) -> Value {
    json!({"name": name, "field_type": "reference_list", "target_kind": target})
}

fn kind(name: &str, scoped_by: Option<&str>, fields: Vec<Value>) -> Value {
    let mut kind = json!({"name": name, "keyword": name, "fields": fields});
    if let Some(scope) = scoped_by {
        kind["scoped_by"] = json!(scope);
    }
    kind
}

/// The kinds, as the wire carries them.
pub fn gsd_judged_wire() -> Value {
    let mut kinds = vec![
        kind("milestone", None, vec![]),
        kind("requirement", None, vec![]),
        kind(
            "phase",
            Some("milestone"),
            vec![
                reference("milestone", "milestone"),
                references("requirements", "requirement"),
            ],
        ),
        kind(
            "decision",
            Some("phase"),
            vec![
                reference("phase", "phase"),
                json!({"name": "text", "field_type": "string"}),
            ],
        ),
        kind("context", Some("phase"), vec![reference("phase", "phase")]),
        kind(
            "deferred_idea",
            Some("phase"),
            vec![reference("phase", "phase")],
        ),
        kind("research", Some("phase"), vec![reference("phase", "phase")]),
        kind(
            "plan",
            Some("phase"),
            vec![
                reference("phase", "phase"),
                references("requirements", "requirement"),
                json!({"name": "files_modified", "field_type": "string_list", "file_reference": true, "file_base": "project"}),
                json!({"name": "status", "field_type": "string", "writer": "computed"}),
            ],
        ),
        kind(
            "phase_spec",
            Some("phase"),
            vec![
                reference("phase", "phase"),
                json!({"name": "scores", "field_type": "block", "writer": "verdict", "members": [
                    {"name": "goal", "field_type": "integer", "minimum": 0, "maximum": 100},
                    {"name": "ambiguity", "field_type": "integer", "minimum": 0, "maximum": 100},
                ]}),
            ],
        ),
        kind(
            "locked_req",
            Some("spec"),
            vec![
                reference("spec", "phase_spec"),
                references("traces", "requirement"),
            ],
        ),
        kind(
            "interview",
            Some("spec"),
            vec![reference("spec", "phase_spec")],
        ),
        kind(
            "package_audit",
            None,
            vec![
                json!({"name": "legitimacy", "field_type": "enum", "writer": "verdict",
                "enum_values": ["legitimate", "suspicious", "unknown"]}),
            ],
        ),
        kind("truth", Some("plan"), vec![reference("plan", "plan")]),
        kind(
            "artifact",
            Some("plan"),
            vec![
                reference("plan", "plan"),
                json!({"name": "path", "field_type": "string", "file_reference": true, "file_base": "project"}),
            ],
        ),
        kind(
            "key_link",
            Some("plan"),
            vec![
                reference("plan", "plan"),
                json!({"name": "from", "field_type": "string", "file_reference": true, "file_base": "project"}),
                json!({"name": "to", "field_type": "string", "file_reference": true, "file_base": "project"}),
                json!({"name": "pattern", "field_type": "string"}),
            ],
        ),
    ];
    for belongs_to_plan in ["task", "checkpoint", "prohibition", "user_setup"] {
        kinds.push(kind(
            belongs_to_plan,
            Some("plan"),
            vec![reference("plan", "plan")],
        ));
    }
    for of_research in [
        "finding",
        "stack_choice",
        "pattern",
        "pitfall",
        "dont_hand_roll",
        "open_question",
    ] {
        kinds.push(kind(
            of_research,
            Some("research"),
            vec![reference("research", "research")],
        ));
    }
    Value::Array(kinds)
}

/// A local code as text (a host source never writes a core code's spelling as a literal, and these are `@gsd/core`'s).
fn code(letter: char, number: u16) -> String {
    format!("{letter}{number:03}")
}

fn inputs(pairs: &[(&str, &str)]) -> Value {
    pairs
        .iter()
        .map(|(name, selector)| json!({"name": name, "selector": selector}))
        .collect()
}

fn judged(judge: &str, outcomes: &[&str], passing: &[&str], targets: Value) -> Value {
    json!({"judge": judge, "outcomes": outcomes, "passing": passing, "targets": targets})
}

/// The eight judged rules, as the wire carries them.
pub fn gsd_judged_rules() -> Value {
    let mut rules = Vec::new();
    for (code, severity) in [
        (code('W', 311), "warning"),
        (code('W', 313), "warning"),
        (code('W', 315), "warning"),
        (code('W', 316), "warning"),
    ] {
        rules.push(json!({
            "code": code, "severity": severity, "check": "judged",
            "message_template": "the plan check of {id} does not hold",
            "judged": judged("gsd-plan-checker", &["passed", "issues"], &["passed"],
                json!([{"kind": "plan", "inputs": inputs(&PLAN_CHECK_INPUTS)}])),
        }));
    }
    rules.push(json!({
        "code": code('E', 501), "severity": "error", "check": "judged",
        "message_template": "the verification of {id} does not hold",
        "judged": judged("gsd-verifier", &["passed", "failed"], &["passed"], json!([
            {"kind": "truth", "inputs": inputs(&[
                ("truth", "self"), ("plan", "self.plan"), ("phase", "self.plan.phase"),
                ("code", "file(self.plan.files_modified)"), ("tests", "test(self)")])},
            {"kind": "artifact", "inputs": inputs(&[
                ("artifact", "self"), ("plan", "self.plan"), ("code", "file(self.path)")])},
            {"kind": "key_link", "unless_set": ["pattern"], "inputs": inputs(&[
                ("link", "self"), ("plan", "self.plan"), ("from", "file(self.from)"), ("to", "file(self.to)")])},
        ])),
    }));
    let mut e202 = json!({
        "code": code('E', 202), "severity": "error", "check": "judged",
        "message_template": "the phase spec {id} was not judged",
        "judged": judged("gsd-spec-agent", &["passed", "issues"], &["passed"], json!([
            {"kind": "phase_spec", "inputs": inputs(&[
                ("spec", "self"), ("items", "self<locked_req|interview.spec"), ("phase", "self.phase")])},
        ])),
    });
    e202["judged"]["values"] = json!(["phase_spec.scores"]);
    rules.push(e202);
    rules.push(json!({
        "code": code('W', 203), "severity": "warning", "check": "judged",
        "message_template": "the locked requirement {id} was not judged",
        "judged": judged("gsd-spec-agent", &["passed", "issues"], &["passed"], json!([
            {"kind": "locked_req", "inputs": inputs(&[("req", "self"), ("traces", "self.traces")])},
        ])),
    }));
    let mut w205 = json!({
        "code": code('W', 205), "severity": "warning", "check": "judged",
        "message_template": "the package audit {id} was not judged",
        "judged": judged("gsd-package-auditor", &["legitimate", "suspicious", "unknown"], &["legitimate"], json!([
            {"kind": "package_audit", "inputs": inputs(&[("audit", "self")])},
        ])),
    });
    w205["judged"]["outcome_fields"] = json!(["package_audit.legitimacy"]);
    rules.push(w205);
    Value::Array(rules)
}

/// The kinds, the rules and the `plan_check` setting of `@gsd/core`, as `c` contributes them (no pass).
pub fn declare_gsd_judged(c: &mut ContributionsBuilder) {
    c.raw_category(DeclaredCategory::Entities, gsd_judged_wire());
    super::declare_rules(c, gsd_judged_rules());
    // `workflow.plan_check`: while it is false the plan-check rules are gated and never evaluated (K6, K7 U10).
    c.setting("plan_check", |s| {
        s.bool()
            .default(true)
            .describe("Check a plan before it is executed")
            .gates(&[
                code('W', 311).as_str(),
                code('W', 313).as_str(),
                code('W', 315).as_str(),
                code('W', 316).as_str(),
            ]);
    });
}

/// The extension: [`declare_gsd_judged`] and the compute pass `plan_status`.
pub fn gsd_judged_builder() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(GSD_JUDGED, "0.1.0"));
    declare_gsd_judged(&mut c);
    c.raw_category(DeclaredCategory::Passes, gsd_judged_passes());
    c
}

/// The compute pass `plan_status`, which gives a plan its `status` from its judgements.
pub fn gsd_judged_passes() -> Value {
    json!([{"name": "plan_status", "phase": "compute", "computes": ["plan.status"]}])
}

/// `@gsd/core` served in process.
pub fn gsd_judged_runtime() -> InProcessRuntime {
    InProcessRuntime::new().with(gsd_judged_builder).answer_raw(
        GSD_JUDGED,
        "__pass_plan_status",
        specforge_wasm::WasmCallResult::Ok(
            json!({"diagnostics": [], "summary": {}, "values": []})
                .to_string()
                .into_bytes(),
        ),
    )
}

/// The phase, decisions, plan, truth and the like the judged tests read.
pub const GSD_JUDGED_SPEC: &str = r#"milestone v1.0 "One" {}

requirement AUTH-07 "Sessions" {}

phase 03 "Three" {
  milestone v1.0
  requirements [AUTH-07]
}

decision D-01 "Use tokens" {
  phase 03
}

decision D-00 "A project decision" {}

context C3 "Context" {
  phase 03
}

plan 03-02 "Session tokens" {
  phase 03
  requirements [AUTH-07]
  files_modified ["src/auth/session.ts"]
}

plan 03-03 "Another" {
  phase 03
}

task T1 "Task" {
  plan 03-02
}

truth TR1 "Tokens survive a reload" {
  plan 03-02
}

package_audit PA "Audit" {}
"#;

/// A project over `@gsd/core` whose spec is [`GSD_JUDGED_SPEC`] and `extra`, the instructions file `CLAUDE.md` and a
/// source file `src/auth/session.ts` on disk.
pub fn gsd_judged_project(extra: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_JUDGED])
        .installing_enabled()
        .file("a.spec", format!("{GSD_JUDGED_SPEC}{extra}"))
        .file("CLAUDE.md", "# Instructions\n")
        .file("src/auth/session.ts", "export const session = 1;\n")
}
