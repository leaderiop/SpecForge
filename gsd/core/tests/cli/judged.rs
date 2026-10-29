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
