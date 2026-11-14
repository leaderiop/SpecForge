//! The risk kind, end to end (`spec/governance.md`): the `governance` project's four risks, what `specforge` computes
//! for them and what it refuses.

use crate::harness::{Project, project};

/// `rpn` and `post_rpn` of `id`, as `specforge query` answers them.
fn rpns(p: &Project, id: &str) -> (serde_json::Value, serde_json::Value) {
    let fields = p.fields_of(id);
    (fields["rpn"].clone(), fields["post_rpn"].clone())
}

#[test]
fn a_risk_rates_its_rpn_end_to_end() {
    let p = project("governance");
    for (id, rpn, post_rpn) in [
        ("RK1", 175, 35),
        ("RK2", 6, 6),
        ("RK3", 350, 350),
        ("RK4", 125, 30),
    ] {
        assert_eq!(rpns(&p, id), (rpn.into(), post_rpn.into()), "{id}");
    }
    assert_eq!(p.report(), Vec::new());
}

#[test]
fn a_rating_outside_one_to_ten_is_e061_and_a_written_rpn_is_e082() {
    let p = project("governance");
    p.edit("spec/risks.spec", "  severity   2\n", "  severity   11\n");
    assert!(p.codes().contains(&"E061".to_string()), "{:#?}", p.report());
    assert_eq!(
        rpns(&p, "RK2"),
        (serde_json::Value::Null, serde_json::Value::Null)
    );

    let p = project("governance");
    p.edit(
        "spec/risks.spec",
        "    occurrence 1\n",
        "    occurrence 0\n",
    );
    assert!(p.codes().contains(&"E061".to_string()), "{:#?}", p.report());
    assert_eq!(
        rpns(&p, "RK1"),
        (serde_json::Value::Null, serde_json::Value::Null)
    );

    let p = project("governance");
    p.edit(
        "spec/risks.spec",
        "  detection  1\n",
        "  detection  1\n  rpn        5\n",
    );
    let report = p.report();
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "E082" && message.contains("rpn")),
        "{report:#?}"
    );
    assert_eq!(rpns(&p, "RK2").0, 6, "the computed value stands");
}

#[test]
fn a_risk_is_created_with_new_and_set_refuses_its_rpn() {
    let p = project("governance");
    let run = p.run(&[
        "new",
        "risk",
        "RK9",
        "Keys rot",
        "--in",
        "spec/risks.spec",
        "--cause",
        "a backup keeps an old key",
        "--effect",
        "a rotated key can still be read",
        "--severity",
        "3",
        "--occurrence",
        "3",
        "--detection",
        "3",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(rpns(&p, "RK9"), (27.into(), 27.into()));
    let before = p.read("spec/risks.spec");
    let set = p.run(&["set", "RK9", "rpn", "5"]);
    assert_ne!(set.code, 0, "{}", set.all());
    assert!(set.all().contains("computed"), "{}", set.all());
    assert_eq!(p.read("spec/risks.spec"), before, "nothing is written");
}

/// `status` and `blocking` of `id`, as `specforge query` answers them.
fn state(p: &Project, id: &str) -> (String, bool) {
    let fields = p.fields_of(id);
    (
        fields["status"].as_str().unwrap_or_default().to_string(),
        fields["blocking"]
            .as_bool()
            .unwrap_or_else(|| panic!("{id}: no blocking in {fields}")),
    )
}

/// Plan 01-01's summary, `status complete`: its tasks are done.
const SUMMARY: &str = r#"
summary 01-01-SUMMARY "Encrypted store" {
  plan                   v1.0/01/01-01
  status                 complete
  one_liner              "Keys are encrypted at rest and rotate"
  subsystem              "store"
  requirements_completed [GOV-01, GOV-02]
}
"#;

#[test]
fn a_risk_is_open_mitigating_or_mitigated() {
    let p = project("governance");
    let state_of = |id: &str| state(&p, id);
    assert_eq!(state_of("RK1"), ("mitigating".into(), true));
    assert_eq!(state_of("RK2"), ("open".into(), false));
    assert_eq!(state_of("RK3"), ("open".into(), true));
    assert_eq!(state_of("RK4"), ("mitigating".into(), true));
    let plans = p.read("spec/plans.spec");
    p.write("spec/plans.spec", &format!("{plans}{SUMMARY}"));
    assert_eq!(state(&p, "RK1"), ("mitigated".into(), false), "current 35");
    assert_eq!(
        state(&p, "RK4"),
        ("mitigating".into(), true),
        "a truth is not done by its plan"
    );
}

#[test]
fn a_verified_truth_mitigates_its_risk() {
    let p = project("governance");
    p.write(
        "spec/verify.spec",
        "verification 01-VERIFICATION \"Store\" {\n  phase v1.0/01\n}\n",
    );
    assert_eq!(state(&p, "RK4"), ("mitigating".into(), true));
    let run = p.record("v1.0/01/01-01/TR1", "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p, "RK4"), ("mitigated".into(), false), "current 30");
}

#[test]
fn the_threshold_follows_specforge_risk_block_on() {
    let p = project("governance");
    p.settings(serde_json::json!({ "specforge.risk_block_on": 400 }));
    assert_eq!(state(&p, "RK3"), ("open".into(), false), "350 is under 400");
    assert_eq!(
        state(&p, "RK1"),
        ("mitigating".into(), false),
        "175 is under 400"
    );
    p.settings(serde_json::json!({ "specforge.risk_block_on": 175 }));
    assert_eq!(
        state(&p, "RK1"),
        ("mitigating".into(), true),
        "at the threshold"
    );
    assert_eq!(
        state(&p, "RK4"),
        ("mitigating".into(), false),
        "125 is under 175"
    );
}
