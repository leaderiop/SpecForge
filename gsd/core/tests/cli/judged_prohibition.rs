//! `gsd/W525`: a judgment-tier prohibition (one that names no enforcing test) owes the verifier a verdict once its
//! plan is complete, as upstream's probe marks such an item unverified until it is judged.

use super::harness::{Project, judged, project};

/// The fixture's one prohibition: `verification judgment`, no `test`, on plan 02-01.
const PROHIBITION: &str = "v1.0/02/02-01/PR1";

/// The `verify` project with plan 02-01 executed: a summary with `status complete` completes it.
fn executed() -> Project {
    let p = project("verify");
    p.write(
        "spec/summary.spec",
        "summary 02-01-SUMMARY \"Save to disk: summary\" {\n  plan      v1.0/02/02-01\n  status    complete\n  one_liner \"Tasks are saved\"\n  subsystem \"store\"\n}\n",
    );
    p
}

/// `(severity, message)` of each `gsd/W525` diagnostic of `check --format json`.
fn w525(p: &Project) -> Vec<(String, String)> {
    let run = p.run(&["check", "--format", "json"]);
    let all: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    all.as_array()
        .expect("a list")
        .iter()
        .filter(|d| d["code"] == "gsd/W525")
        .map(|d| {
            let text = |key: &str| d[key].as_str().unwrap_or_default().to_string();
            (text("severity").to_lowercase(), text("message"))
        })
        .collect()
}

#[test]
fn a_judgment_tier_prohibition_of_a_complete_plan_is_w525() {
    let p = executed();
    assert_eq!(
        w525(&p),
        [(
            "warning".to_string(),
            format!("prohibition '{PROHIBITION}' has no fresh verifier verdict: no verdict")
        )]
    );
    let owed: Vec<(String, String)> = judged(&p.report_all())
        .into_iter()
        .filter(|(code, _)| code == "gsd/W525")
        .collect();
    assert_eq!(owed, [("gsd/W525".to_string(), PROHIBITION.to_string())]);
}

#[test]
fn a_held_verdict_silences_w525() {
    let p = executed();
    let run = p.record(PROHIBITION, "W525", "held", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(w525(&p), []);
}

#[test]
fn a_prohibition_that_names_its_test_owes_no_w525_verdict() {
    let p = executed();
    p.edit(
        "spec/plans.spec",
        "verification judgment",
        "verification test\n  test         \"node --test test/store.test.js\"",
    );
    assert_eq!(w525(&p), []);
}

#[test]
fn a_prohibition_of_an_open_plan_owes_no_w525_verdict() {
    let p = project("verify");
    assert_eq!(w525(&p), []);
}

#[test]
fn a_violated_or_unverified_w525_verdict_is_the_rules_finding() {
    for outcome in ["violated", "unverified"] {
        let p = executed();
        let run = p.record(
            PROHIBITION,
            "W525",
            outcome,
            &[
                "--findings",
                r#"[{"severity": "warning", "message": "the store writes in place"}]"#,
            ],
        );
        assert_eq!(run.code, 0, "{outcome}: {}", run.all());
        let found = w525(&p);
        assert_eq!(found.len(), 1, "{outcome}: {found:?}");
        assert_eq!(found[0].0, "warning", "{outcome}: {found:?}");
        assert!(
            found[0].1.contains(&format!("is '{outcome}' (1 finding)")),
            "{outcome}: {found:?}"
        );
    }
}
