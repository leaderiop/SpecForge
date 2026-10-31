//! The rules' silence (D-r44-3): the code book's other half. `parity_rules.rs` asks every code for a test that makes it
//! fire and a test that keeps it silent; most codes' silent tests sit beside their firing ones, and these two are for
//! the rest: the verified taskboard twin trips no unjudged rule, and a passing verdict silences each judged one.

use serde_json::json;

use super::harness::{judged, project, taskboard};

#[test]
fn the_clean_twin_is_silent_for_the_unjudged_codes() {
    /// The unjudged codes the taskboard twin must never report, under any of the three profiles. `W407` is the one the
    /// twin reports under strict and plus (its tasks carry no commit), so `task_without_commit_is_w407_...` holds it.
    const SILENT_ON_THE_TWIN: [&str; 52] = [
        "E101", "E102", "W103", "W104", "E105", "E106", "E107", "E108", "E201", "W204", "E206",
        "E301", "E302", "E303", "W304", "W305", "W306", "E307", "W308", "W309", "E310", "E312",
        "E314", "W317", "W318", "E319", "E320", "I321", "E322", "W323", "W324", "E507", "E401",
        "E402", "W403", "W404", "E405", "W406", "W408", "E502", "E503", "W504", "E505", "W506",
        "E509", "W510", "E511", "E512", "E513", "E515", "E516", "E517",
    ];
    for profile in ["gsd-compat", "gsd-strict", "gsd-plus"] {
        let p = taskboard();
        p.settings(json!({ "profile": profile }));
        let reported = p.codes();
        for code in SILENT_ON_THE_TWIN {
            assert!(
                !reported.contains(&format!("gsd/{code}")),
                "{profile}: the twin reports {code}: {reported:?}"
            );
        }
    }
}

/// Whether the report has a judged diagnostic of `code` on `entity`.
fn owed(p: &super::harness::Project, code: &str, entity: &str) -> bool {
    judged(&p.report_all())
        .iter()
        .any(|(c, id)| c == &format!("gsd/{code}") && id == entity)
}

#[test]
fn a_passing_verdict_silences_each_judged_code() {
    // The plan checker's four rules: W311, W313, W315 and W316.
    let p = project("plan");
    let plan = "v1.0/01/01-02";
    for code in ["W311", "W313", "W315", "W316"] {
        assert!(owed(&p, code, plan), "{code} is owed before a verdict");
        let run = p.record(plan, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
        assert!(!owed(&p, code, plan), "{code} is silent after a pass");
    }

    // The spec's ambiguity score (E202), the requirement's traces (W203) and the research's package audit (W205).
    let p = project("discuss");
    let spec = "v1.0/02/02-SPEC";
    let on_gate =
        r#"{"scores":{"goal":75,"boundary":70,"constraint":65,"acceptance":70,"ambiguity":20}}"#;
    assert!(owed(&p, "E202", spec));
    let run = p.record(spec, "E202", "passed", &["--values", on_gate]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!owed(&p, "E202", spec));
    let req = p.nodes_of("locked_req")[0]["id"]
        .as_str()
        .expect("an id")
        .to_string();
    assert!(owed(&p, "W203", &req));
    let run = p.record(&req, "W203", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!owed(&p, "W203", &req));
    let audit = "v1.0/02/02-RESEARCH/PA1";
    assert!(owed(&p, "W205", audit));
    let run = p.record(audit, "W205", "legitimate", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!owed(&p, "W205", audit));

    // The verifier's verdict on a truth (E501) and the milestone audit's integration check (E518).
    let p = project("verify");
    let truth = "v1.0/01/01-01/TR1";
    assert!(owed(&p, "E501", truth));
    let run = p.record(truth, "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!owed(&p, "E501", truth));
    let audit = "v1.0/A1";
    assert!(owed(&p, "E518", audit));
    let run = p.record(audit, "E518", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!owed(&p, "E518", audit));
}
