//! The judged rules on fixtures: what they report before a verdict, under a gate and under a profile.

use super::harness::{Project, judged, project, unjudged};
use serde_json::json;

const PLAN_CHECK: [&str; 4] = ["W311", "W313", "W315", "W316"];

/// `(code, severity)` of each diagnostic of a code in `codes`, from `check --format json`, sorted.
fn severities(p: &Project, codes: &[&str]) -> Vec<(String, String)> {
    let run = p.run(&["check", "--format", "json"]);
    let all: serde_json::Value = serde_json::from_str(&run.stdout).expect("check prints JSON");
    let mut found: Vec<(String, String)> = all
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| {
            d["code"]
                .as_str()
                .is_some_and(|c| codes.iter().any(|k| c == format!("gsd/{k}")))
        })
        .map(|d| {
            (
                d["code"].as_str().unwrap_or_default().to_string(),
                d["severity"].as_str().unwrap_or_default().to_lowercase(),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_plan_without_a_verdict_reports_w311_to_w316() {
    let p = project("plan");
    let plans: Vec<String> = p
        .nodes_of("plan")
        .iter()
        .map(|n| n["id"].as_str().expect("an id").to_string())
        .collect();
    assert!(plans.len() >= 3, "{plans:?}");
    let report = p.report_all();
    let mut expected: Vec<(String, String)> = plans
        .iter()
        .flat_map(|id| PLAN_CHECK.map(|c| (format!("gsd/{c}"), id.clone())))
        .collect();
    expected.sort();
    assert_eq!(judged(&report), expected, "{report:#?}");
    assert_eq!(unjudged(&report), Vec::new());
    // The message names the rule and the state.
    let w311 = report
        .iter()
        .find(|(c, m)| c == "gsd/W311" && m.contains("v1.0/01/01-02"))
        .expect("W311 on 01-02");
    assert_eq!(
        w311.1,
        "plan 'v1.0/01/01-02' is not checked against its phase goal, scope and settled decisions: no verdict"
    );
}

#[test]
fn plan_check_off_reports_none() {
    let p = project("plan");
    p.settings(json!({ "workflow.plan_check": false }));
    let report = p.report_all();
    assert_eq!(judged(&report), Vec::new(), "{report:#?}");
    p.check_exact(&PLAN_CHECK);
}

#[test]
fn strict_raises_w311_to_error() {
    let p = project("plan");
    let compat = severities(&p, &PLAN_CHECK);
    assert!(compat.len() >= 12, "{compat:?}");
    assert!(compat.iter().all(|(_, s)| s == "warning"), "{compat:?}");
    p.settings(json!({ "profile": "gsd-strict" }));
    let strict = severities(&p, &PLAN_CHECK);
    assert_eq!(strict.len(), compat.len());
    // W311, W313 and W315 are errors under gsd-strict; W316 stays a warning (rules.md).
    for (code, severity) in &strict {
        let want = if code == "gsd/W316" {
            "warning"
        } else {
            "error"
        };
        assert_eq!(severity, want, "{code}");
    }
    assert_ne!(p.run(&["check"]).code, 0);
}

const PLAN: &str = "v1.0/01/01-02";

/// The state the kernel reports for a verdict: `(state, passed)`.
fn state(p: &Project, entity: &str, code: &str) -> (String, bool) {
    let inputs = p.inputs(entity, code);
    (
        inputs["verdict"]["state"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        inputs["verdict"]["passed"].as_bool().unwrap_or(false),
    )
}

#[test]
fn record_verdict_records_a_plan_check_verdict() {
    let p = project("plan");
    assert_eq!(state(&p, PLAN, "W311"), ("missing".to_string(), false));
    let before = judged(&p.report_all());
    assert!(before.contains(&("gsd/W311".to_string(), PLAN.to_string())));

    let run = p.record(PLAN, "W311", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());

    // The verdict member and its ledger line both exist, so E083 is not reported.
    assert!(
        p.read("spec/plans.spec")
            .contains("verdict gsd/W311 passed {")
    );
    assert!(
        p.read("specforge-harness")
            .contains(&format!("{PLAN} verdict:gsd/W311 "))
    );
    let report = p.report_all();
    assert!(
        report.iter().all(|(c, _)| c != "E083" && c != "W168"),
        "{report:#?}"
    );
    // Only that rule on that plan is gone.
    let after = judged(&report);
    let expected: Vec<_> = before
        .into_iter()
        .filter(|pair| *pair != ("gsd/W311".to_string(), PLAN.to_string()))
        .collect();
    assert_eq!(after, expected, "{report:#?}");
    assert_eq!(state(&p, PLAN, "W311"), ("fresh".to_string(), true));
}

/// D-r40-1: recording a non-passing verdict on a rule whose effective severity is an Error. The host re-checks the
/// project the edit would leave, and a judged rule's message changes with the verdict (E080 keys a diagnostic by its
/// message shape).
#[test]
#[ignore = "kernel: a verdict's own judged state is not a new error (K17)"]
fn a_failing_verdict_on_an_error_rule_records() {
    let p = project("plan");
    p.settings(json!({ "profile": "gsd-strict" }));
    let run = p.record(PLAN, "W311", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p, PLAN, "W311"), ("fresh".to_string(), false));
    let w311: Vec<_> = p
        .report_all()
        .into_iter()
        .filter(|(c, m)| c == "gsd/W311" && m.contains(PLAN))
        .collect();
    assert_eq!(w311.len(), 1, "{w311:?}");
    assert!(w311[0].1.ends_with("is 'issues' (0 findings)"), "{w311:?}");
}

/// What the probe above found (E080): the refusal itself, so the day the kernel stops refusing this test fails and the
/// ignored one above is un-ignored. Fail-closed: nothing is written, and the rule keeps saying `no verdict`.
#[test]
fn a_failing_verdict_on_an_error_rule_is_refused_whole() {
    let p = project("plan");
    p.settings(json!({ "profile": "gsd-strict" }));
    let spec = p.read("spec/plans.spec");
    let run = p.record(PLAN, "W311", "issues", &[]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E080"), "{}", run.all());
    assert!(run.all().contains("nothing was written"), "{}", run.all());
    assert_eq!(p.read("spec/plans.spec"), spec);
    assert!(!p.exists("specforge-harness"));
    assert_eq!(state(&p, PLAN, "W311"), ("missing".to_string(), false));
    // A passing verdict on the same Error rule is fine: it adds no error.
    let run = p.record(PLAN, "W311", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p, PLAN, "W311"), ("fresh".to_string(), true));
}

/// Under `gsd-compat` W311 is a warning, so a failing verdict records: it only changes the warning's text.
#[test]
fn a_failing_verdict_on_a_warning_rule_records() {
    let p = project("plan");
    let run = p.record(
        PLAN,
        "W311",
        "issues",
        &[
            "--findings",
            r#"[{"severity":"error","message":"no wiring"}]"#,
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p, PLAN, "W311"), ("fresh".to_string(), false));
    let messages: Vec<String> = p
        .report_all()
        .into_iter()
        .filter(|(c, m)| c == "gsd/W311" && m.contains(PLAN))
        .map(|(_, m)| m)
        .collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    // (The kernel's count reads a list of values, not of blocks, so it says 0 for any findings.)
    assert!(messages[0].contains("is 'issues'"), "{messages:?}");
    assert!(p.read("spec/plans.spec").contains("message \"no wiring\""));
}

#[test]
fn a_stale_basis_is_refused() {
    let p = project("plan");
    let basis = p.inputs(PLAN, "W311")["basis"].to_string();
    p.edit(
        "spec/plans.spec",
        "A task can be marked complete",
        "A task can be marked done",
    );
    let run = p.record_over(PLAN, "W311", "passed", &basis, &[]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("stale_revision"), "{}", run.all());
    assert!(!p.read("spec/plans.spec").contains("verdict gsd/W311"));
}

#[test]
fn an_unknown_rule_is_refused() {
    let p = project("plan");
    let basis = p.inputs(PLAN, "W311")["basis"].to_string();
    let run = p.run(&["gsd", "record-verdict", PLAN, "W999", "passed", "m", &basis]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("W999"), "{}", run.all());
    // a judged rule the entity's kind is not a target of
    let run = p.run(&[
        "gsd",
        "record-verdict",
        "v1.0/01/01-02/TR1",
        "W311",
        "passed",
        "m",
        &basis,
    ]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(!p.exists("specforge-harness"));
}

#[test]
fn malformed_findings_are_invalid_input() {
    let p = project("plan");
    let refusal = |run: &super::harness::Run| -> (String, String) {
        let json: serde_json::Value = serde_json::from_str(&run.stderr)
            .or_else(|_| serde_json::from_str(&run.stdout))
            .unwrap_or_else(|e| panic!("a JSON error ({e}): {}", run.all()));
        (
            json["code"].as_str().unwrap_or_default().to_string(),
            json["message"].as_str().unwrap_or_default().to_string(),
        )
    };
    for (option, bad) in [
        ("--findings", "not json"),
        ("--cited", "{}"),
        ("--values", "[1]"),
    ] {
        let run = p.record(PLAN, "W311", "passed", &[option, bad, "--format", "json"]);
        assert_ne!(run.code, 0, "{option}: {}", run.all());
        let (code, message) = refusal(&run);
        assert_eq!(code, "invalid_input", "{option}: {}", run.all());
        assert!(
            message.starts_with(option.trim_start_matches("--")),
            "{option}: {message}"
        );
    }
    let run = p.run(&[
        "gsd",
        "record-verdict",
        PLAN,
        "W311",
        "passed",
        "m",
        "not json",
        "--format",
        "json",
    ]);
    let (code, message) = refusal(&run);
    assert_eq!(code, "invalid_input", "{}", run.all());
    assert!(message.starts_with("basis"), "{message}");
    assert!(!p.exists("specforge-harness"));
}

#[test]
fn the_judge_is_the_rules_not_the_callers() {
    let p = project("plan");
    // There is no judge argument: naming one is a usage error.
    let basis = p.inputs(PLAN, "W311")["basis"].to_string();
    let run = p.run(&[
        "gsd",
        "record-verdict",
        PLAN,
        "W311",
        "passed",
        "m",
        &basis,
        "--judge",
        "gsd-verifier",
    ]);
    assert_ne!(run.code, 0, "{}", run.all());
    // A recorded verdict names the rule's judge.
    let run = p.record(PLAN, "W311", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(
        p.read("spec/plans.spec")
            .contains("judge \"gsd-plan-checker@")
    );
}

const PLAN_CHECK_RULES: [&str; 4] = ["W311", "W313", "W315", "W316"];

fn status_of(p: &Project, id: &str) -> serde_json::Value {
    p.fields_of(id)["status"].clone()
}

fn record_all(p: &Project, entity: &str) {
    for code in PLAN_CHECK_RULES {
        let run = p.record(entity, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
    }
}

#[test]
fn four_passing_verdicts_make_checked() {
    let p = project("plan");
    assert_eq!(status_of(&p, PLAN), "draft");
    // Three of four is not enough.
    for code in &PLAN_CHECK_RULES[..3] {
        let run = p.record(PLAN, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
        assert_eq!(status_of(&p, PLAN), "draft", "after {code}");
    }
    let run = p.record(PLAN, "W316", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(status_of(&p, PLAN), "checked");
    // The other plans are untouched.
    assert_eq!(status_of(&p, "v1.0/01/01-01"), "draft");
}

#[test]
fn a_failing_check_leaves_the_plan_a_draft() {
    let p = project("plan");
    record_all(&p, PLAN);
    assert_eq!(status_of(&p, PLAN), "checked");
    // A failing verdict replaces a passing one (under gsd-compat W313 is a warning, so the host takes it).
    let run = p.record(PLAN, "W313", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(status_of(&p, PLAN), "draft");
}

#[test]
fn an_edited_input_stales_checked() {
    let p = project("plan");
    record_all(&p, PLAN);
    assert_eq!(status_of(&p, PLAN), "checked");
    p.edit(
        "spec/plans.spec",
        "A task can be marked complete",
        "A task can be marked done",
    );
    assert_eq!(status_of(&p, PLAN), "draft");
    let stale: Vec<_> = p
        .report_all()
        .into_iter()
        .filter(|(c, m)| c == "gsd/W311" && m.contains(PLAN))
        .collect();
    assert_eq!(stale.len(), 1, "{stale:?}");
    assert!(stale[0].1.contains("is stale: "), "{stale:?}");
    assert_eq!(state(&p, PLAN, "W311").0, "stale");
    // Judging it again makes it checked again.
    record_all(&p, PLAN);
    assert_eq!(status_of(&p, PLAN), "checked");
}

#[test]
fn plan_check_off_is_ready() {
    let p = project("plan");
    p.settings(json!({ "workflow.plan_check": false }));
    assert_eq!(status_of(&p, PLAN), "ready");
    assert_eq!(status_of(&p, "v1.0/01/01-01"), "ready");
}
