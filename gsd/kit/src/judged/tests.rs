use serde_json::{Value, json};
use specforge_extension_sdk::PlanOp;
use specforge_extension_sdk::testing::MockHost;

use super::record::{Check, Proposed, Recorder, declare_record};
use super::*;

const fn rule(code: &'static str, targets: &'static [Target]) -> Judged {
    Judged {
        code,
        message: "thing '{id}' is not judged",
        judge: "gsd-judge",
        outcomes: &["holds", "broken"],
        passing: &["holds"],
        targets,
        values: &[],
        outcome_fields: &[],
    }
}

const fn target(judged_while: Option<Owed>, unless_set: &'static [&'static str]) -> Target {
    Target {
        kind: "thing",
        inputs: &[("thing", "self"), ("file", "file(self.path)")],
        unless_set,
        judged_while,
    }
}

static COMPLETE: &[Judged] = &[rule("W901", &[target(Some(Owed::Complete), &[])])];
static OPEN: &[Judged] = &[rule("W901", &[target(Some(Owed::Open), &[])])];
static EITHER: &[Judged] = &[rule("W901", &[target(None, &[])])];
static UNLESS: &[Judged] = &[rule("W901", &[target(None, &["pattern"])])];

fn severity(code: &str) -> ValidationSeverity {
    match code {
        "E901" => ValidationSeverity::Error,
        "I901" => ValidationSeverity::Info,
        _ => ValidationSeverity::Warning,
    }
}

fn builder() -> ContributionsBuilder {
    ContributionsBuilder::new(ExtensionMeta::new("@gsd/test", "0.0.0"))
}

/// The judged rules `table` declares, as the host reads them.
fn rules_of(table: &'static [Judged]) -> Vec<Value> {
    let mut c = builder();
    declare(&mut c, table, severity);
    let text = MockHost::new(c)
        .describe_json("validation_rules")
        .expect("rules");
    let rules: Value = serde_json::from_str(&text).expect("JSON");
    rules["items"].as_array().cloned().unwrap_or_default()
}

fn only_target(table: &'static [Judged]) -> Value {
    let rules = rules_of(table);
    assert_eq!(rules.len(), 1, "{rules:?}");
    let targets = rules[0]["judged"]["targets"].as_array().cloned();
    let targets = targets.unwrap_or_default();
    assert_eq!(targets.len(), 1, "{rules:?}");
    targets[0].clone()
}

#[test]
fn a_target_judged_once_complete_says_complete() {
    assert_eq!(only_target(COMPLETE)["judged_while"], "complete");
}

#[test]
fn a_target_judged_while_open_says_pending() {
    assert_eq!(only_target(OPEN)["judged_while"], "pending");
}

#[test]
fn a_target_judged_in_either_state_says_nothing_and_declares_no_empty_unless_set() {
    let target = only_target(EITHER);
    assert!(
        target.get("judged_while").is_none_or(Value::is_null),
        "{target}"
    );
    assert!(
        target
            .get("unless_set")
            .is_none_or(|v| v.is_null() || v == &json!([])),
        "{target}"
    );
    assert_eq!(target["kind"], "thing");
    assert_eq!(
        target["inputs"],
        json!([{"name": "thing", "selector": "self"}, {"name": "file", "selector": "file(self.path)"}])
    );
}

#[test]
fn a_target_unless_set_is_declared() {
    assert_eq!(only_target(UNLESS)["unless_set"], json!(["pattern"]));
}

static GRADED: &[Judged] = &[
    rule("E901", &[target(None, &[])]),
    rule("I901", &[target(None, &[])]),
    rule("W902", &[target(None, &[])]),
];

#[test]
fn each_rule_has_the_severity_of_its_local_code() {
    let rules = rules_of(GRADED);
    let graded: Vec<(String, String)> = rules
        .iter()
        .map(|r| (r["code"].to_string(), r["severity"].to_string()))
        .collect();
    assert_eq!(graded.len(), 3, "{rules:?}");
    for (code, expected) in [("E901", "error"), ("I901", "info"), ("W902", "warning")] {
        assert!(
            graded
                .iter()
                .any(|(c, s)| c.contains(code) && s.contains(expected)),
            "{code} is not {expected}: {graded:?}"
        );
    }
    assert_eq!(rules[0]["judged"]["judge"], "gsd-judge");
    assert_eq!(rules[0]["message_template"], "thing '{id}' is not judged");
}

#[test]
fn get_finds_a_rule_by_its_local_code_only() {
    assert_eq!(get(GRADED, "I901").map(|j| j.code), Some("I901"));
    assert!(get(GRADED, "W999").is_none());
    assert!(get(GRADED, "gsd/I901").is_none());
}

// The verdict command.

static BOUND: &[Judged] = &[
    rule("W901", &[target(None, &[])]),
    Judged {
        values: &["thing.scores"],
        ..rule("E901", &[target(None, &[])])
    },
];

/// Refuses a `broken` W901, naming the entity and the kind the command resolved.
fn no_broken_thing(verdict: &Proposed<'_>) -> Result<(), String> {
    if verdict.rule.code == "W901" && verdict.outcome == "broken" {
        return Err(format!(
            "outcome: refused on the {} {} citing {}",
            verdict.kind,
            verdict.entity,
            verdict.cited.len()
        ));
    }
    Ok(())
}

const RECORDER: Recorder = Recorder {
    command: "record_test_verdict",
    namespace: "gsd.test",
    extension: "@gsd/test",
    table: BOUND,
    checks: &[no_broken_thing as Check],
};

fn recorder_builder() -> ContributionsBuilder {
    let mut c = builder();
    declare_record(&mut c, &RECORDER);
    c
}

/// The declared command, as the host reads it.
fn command() -> Value {
    let text = MockHost::new(recorder_builder())
        .describe_json("surfaces")
        .expect("surfaces");
    let surfaces: Value = serde_json::from_str(&text).expect("JSON");
    let found = surfaces.to_string();
    let mut stack = vec![surfaces];
    while let Some(v) = stack.pop() {
        match v {
            Value::Object(map) => {
                if map.get("id") == Some(&json!("record_test_verdict")) && map.contains_key("args")
                {
                    return Value::Object(map);
                }
                stack.extend(map.into_values());
            }
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    panic!("no record_test_verdict command in {found}")
}

#[test]
fn the_command_writes_and_takes_exactly_its_arguments_and_no_judge() {
    let command = command();
    assert_eq!(command["writes"], true, "{command}");
    let args = command["args"].as_array().cloned().unwrap_or_default();
    let names: Vec<&str> = args.iter().filter_map(|a| a["name"].as_str()).collect();
    assert_eq!(
        names,
        [
            "entity", "rule", "outcome", "model", "basis", "findings", "cited", "values"
        ]
    );
    let rule = &args[1];
    assert_eq!(rule["required"], true, "{rule}");
    assert!(rule.to_string().contains(r#"["W901","E901"]"#), "{rule}");
    let text = command.to_string();
    assert!(text.contains("--inputs gsd.test/<code>"), "{text}");
    assert!(
        text.contains("JSON object of the field values the rule binds (default {}), such as the scores of gsd.test/E901"),
        "{text}"
    );
}

#[test]
fn a_table_that_binds_no_value_names_no_example() {
    const UNBOUND: Recorder = Recorder {
        table: EITHER,
        ..RECORDER
    };
    assert_eq!(
        record::values_description(&UNBOUND),
        "JSON object of the field values the rule binds (default {})"
    );
}

fn called(args: Value) -> CommandOutput {
    let nodes = ["p/01/T1", "p/01/T2", "q/02/T1"]
        .iter()
        .map(|id| GraphNode {
            id: (*id).to_string(),
            kind: if id.starts_with("q/") {
                "note"
            } else {
                "thing"
            }
            .to_string(),
            ..Default::default()
        })
        .collect();
    let input = CommandInput {
        args: args.as_object().expect("args are an object").clone(),
        graph: CommandGraph::new(nodes, Vec::new()),
        format: CommandFormat::Json,
        today: "2026-11-19".to_string(),
        now: "2026-11-19T10:00:00Z".to_string(),
        nonce: "0".repeat(32),
        ..Default::default()
    };
    recorder_builder()
        .call_command("cmd__record_test_verdict", &input)
        .expect("the command is declared")
}

fn args(entity: &str, rule: &str, outcome: &str, basis: &str) -> Value {
    json!({"entity": entity, "rule": rule, "outcome": outcome, "model": "a-model", "basis": basis})
}

/// The refusal's exit code and message.
fn refused(args: Value) -> (i32, String) {
    let out = called(args);
    let error: Value = serde_json::from_str(&out.stderr).expect("the refusal is JSON");
    let message = error["message"].as_str().unwrap_or_default().to_string();
    (out.exit_code, message)
}

/// The command's own refusal: `invalid_input`, exit 1.
fn refusal(args: Value) -> String {
    let out = called(args);
    assert_eq!(out.exit_code, 1, "{}", out.stdout);
    let error: Value = serde_json::from_str(&out.stderr).expect("the refusal is JSON");
    assert_eq!(error["code"], "invalid_input", "{error}");
    error["message"].as_str().unwrap_or_default().to_string()
}

/// The rule arg is one of the table's codes, so an unknown rule never reaches the command's own refusal.
#[test]
fn an_unknown_rule_is_refused_naming_the_tables_codes() {
    let (code, why) = refused(args("T2", "W999", "holds", "{}"));
    assert_eq!(
        (code, why.as_str()),
        (2, "rule must be one of W901, E901, got 'W999'")
    );
}

#[test]
fn an_outcome_outside_the_rules_is_refused_naming_the_qualified_rule() {
    let why = refusal(args("T2", "W901", "fine", "{}"));
    assert!(
        why.contains("outcome: 'fine' is no outcome of gsd.test/W901 (holds, broken)"),
        "{why}"
    );
}

#[test]
fn a_basis_that_is_not_json_is_refused_naming_the_query_to_run() {
    let why = refusal(args("T2", "W901", "holds", "not json"));
    assert!(
        why.contains("basis is not the JSON object `specforge query <entity> --inputs gsd.test/W901` printed: "),
        "{why}"
    );
}

#[test]
fn a_json_argument_that_does_not_parse_is_refused_by_its_name() {
    let mut call = args("T2", "W901", "holds", "{}");
    call["cited"] = json!("[{");
    let why = refusal(call);
    assert!(why.contains("cited: not the JSON it takes: "), "{why}");
}

#[test]
fn a_check_refuses_with_its_own_text_and_sees_the_resolved_entity() {
    let mut call = args("T2", "W901", "broken", "{}");
    call["cited"] = json!(r#"[{"file": "src/a.rs"}]"#);
    let why = refusal(call);
    assert!(
        why.contains("outcome: refused on the thing p/01/T2 citing 1"),
        "{why}"
    );
    // Every spelling the host reads names the same entity: its path, rooted or not, a suffix, a padded number.
    for spelling in ["p/01/T2", "/p/01/T2", "01/T2", "p/1/T2", "/p/001/T2"] {
        let why = refusal(args(spelling, "W901", "broken", "{}"));
        assert!(why.contains("on the thing p/01/T2 "), "{spelling}: {why}");
    }
}

/// The command reads a spelling as the host does and fails closed: a spelling that names no entity, several, or one of
/// a kind the rule does not judge is refused before any check runs, so no check passes an entity it could not read.
#[test]
fn an_entity_the_command_cannot_resolve_to_a_target_is_refused() {
    for (spelling, why) in [
        ("T9", "entity: 'T9' names no entity of the project"),
        ("/01/T2", "entity: '/01/T2' names no entity of the project"),
        (
            "T1",
            "entity: 'T1' is ambiguous: p/01/T1, q/02/T1; name one by its path",
        ),
        (
            "q/02/T1",
            "entity: 'q/02/T1' is a note, which gsd.test/W901 does not judge (it judges: thing)",
        ),
    ] {
        assert_eq!(
            refusal(args(spelling, "W901", "holds", "{}")),
            why,
            "{spelling}"
        );
    }
}

#[test]
fn a_passing_call_records_the_verdict_with_the_rules_own_judge() {
    let mut call = args("01/T2", "W901", "holds", r#"{"thing": "sha256:aa"}"#);
    call["cited"] = json!(r#"[{"file": "src/a.rs"}]"#);
    call["values"] = json!(r#"{"n": 1}"#);
    let out = called(call);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    // Recorded on the path the spelling names, the entity the checks saw.
    assert_eq!(out.stdout, "verdict gsd.test/W901 holds on p/01/T2\n");
    let plan = out.plan.expect("a plan");
    let [
        PlanOp::SetVerdict {
            entity,
            rule,
            outcome,
            judge,
            model,
            ..
        },
    ] = plan.ops.as_slice()
    else {
        panic!("one verdict: {plan:?}");
    };
    assert_eq!(
        (
            entity.as_str(),
            rule.as_str(),
            outcome.as_str(),
            judge.as_str(),
            model.as_str()
        ),
        ("p/01/T2", "gsd.test/W901", "holds", "gsd-judge", "a-model")
    );
    let wire = serde_json::to_value(&plan.ops[0]).expect("JSON");
    assert_eq!(wire["cited"], json!([{"file": "src/a.rs"}]), "{wire}");
    assert_eq!(wire["values"], json!({"n": 1}), "{wire}");
    assert_eq!(wire["basis"], json!({"thing": "sha256:aa"}), "{wire}");
}
