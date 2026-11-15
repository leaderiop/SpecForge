//! The risk kind, end to end (`spec/governance.md`): the `governance` project's four risks, what `specforge` computes
//! for them and what it refuses.

use crate::harness::{Project, project};

/// `rpn` and `post_rpn` of `id`, as `specforge query` answers them.
fn rpns(p: &Project, id: &str) -> (serde_json::Value, serde_json::Value) {
    let fields = p.fields_of(id);
    (fields["rpn"].clone(), fields["post_rpn"].clone())
}

/// The one gsd/W532 the fixture says: RK3, rated 350 and naming no mitigation.
const RK3_W532: &str = "risk 'RK3' has rpn 350, at or above specforge.risk_block_on (100), and names nothing in mitigated_by";

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
    assert_eq!(
        p.report(),
        [("gsd/W532".to_string(), RK3_W532.to_string())],
        "the fixture's one finding"
    );
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

/// RK1 without its post block, its task done: mitigated and still blocking at its rpn, and not gsd/W532 (it names its
/// mitigation); writing the post rating its done mitigation earns clears it.
#[test]
fn a_mitigated_risk_without_a_post_block_blocks_until_it_is_rated_again() {
    let post = "  post {\n    occurrence 1\n  }\n  mitigated_by [v1.0/01/01-01/T1]\n";
    let p = project("governance");
    let plans = p.read("spec/plans.spec");
    p.write("spec/plans.spec", &format!("{plans}{SUMMARY}"));
    p.edit(
        "spec/risks.spec",
        post,
        "  mitigated_by [v1.0/01/01-01/T1]\n",
    );
    assert_eq!(state(&p, "RK1"), ("mitigated".into(), true), "current 175");
    let w532: Vec<String> = p
        .findings()
        .into_iter()
        .filter(|(code, _)| code == "gsd/W532")
        .map(|(_, message)| message)
        .collect();
    assert_eq!(w532, [RK3_W532], "RK3's alone: RK1 names its mitigation");
    p.edit(
        "spec/risks.spec",
        "  mitigated_by [v1.0/01/01-01/T1]\n",
        post,
    );
    assert_eq!(state(&p, "RK1"), ("mitigated".into(), false), "current 35");
}

/// RK1's task, once plan 01-01 is complete, spelled as the kernel reads a reference from an unscoped risk: a short
/// `T1` beside the full path is E003 and keeps RK1 mitigating; the rooted full path is the task.
#[test]
fn a_mitigation_counts_as_the_kernel_resolves_it() {
    let full = "  mitigated_by [v1.0/01/01-01/T1]\n";
    let p = project("governance");
    let plans = p.read("spec/plans.spec");
    p.write("spec/plans.spec", &format!("{plans}{SUMMARY}"));
    p.edit(
        "spec/risks.spec",
        full,
        "  mitigated_by [v1.0/01/01-01/T1, T1]\n",
    );
    assert!(p.codes().contains(&"E003".to_string()), "{:#?}", p.report());
    assert_eq!(
        state(&p, "RK1"),
        ("mitigating".into(), true),
        "T1 resolves to nothing"
    );
    p.edit(
        "spec/risks.spec",
        "  mitigated_by [v1.0/01/01-01/T1, T1]\n",
        "  mitigated_by [/v1.0/01/01-01/T1]\n",
    );
    assert!(
        !p.codes().contains(&"E003".to_string()),
        "{:#?}",
        p.report()
    );
    assert_eq!(
        state(&p, "RK1"),
        ("mitigated".into(), false),
        "the rooted path is the task"
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

/// Phase 01 with two verifications and an override accepting TR1 on one of them: RK4 is mitigated whichever holds
/// it, so the override is not lost to the other.
#[test]
fn an_override_on_either_verification_of_a_phase_mitigates() {
    let verifications = "verification 01-VERIFICATION \"Store\" {\n  phase v1.0/01\n}\n\n\
        verification 01-VERIFICATION-2 \"Store again\" {\n  phase v1.0/01\n}\n";
    let mut seen = Vec::new();
    for on in ["01-VERIFICATION", "01-VERIFICATION-2"] {
        let p = project("governance");
        p.write(
            "spec/verify.spec",
            &format!(
                "{verifications}\noverride OV1 \"Accepted\" {{\n  verification v1.0/01/{on}\n  \
                 truth        01-01/TR1\n  reason       \"checked by hand\"\n}}\n"
            ),
        );
        seen.push(state(&p, "RK4"));
    }
    assert_eq!(
        seen,
        [("mitigated".into(), false), ("mitigated".into(), false)],
        "current 30 either way"
    );
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

#[test]
fn a_risk_at_the_threshold_with_no_mitigation_is_w532() {
    let p = project("governance");
    let w532: Vec<(String, String)> = p
        .findings()
        .into_iter()
        .filter(|(code, _)| code == "gsd/W532")
        .collect();
    assert_eq!(w532, [("gsd/W532".to_string(), RK3_W532.to_string())]);
    let checked = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value =
        serde_json::from_str(&checked.stdout).expect("check prints JSON");
    let on: Vec<&serde_json::Value> = found
        .as_array()
        .expect("a list")
        .iter()
        .filter(|d| d["code"] == "gsd/W532")
        .collect();
    assert_eq!(on.len(), 1, "{found:#}");
    assert_eq!(on[0]["severity"], "Warning", "{}", on[0]);
    assert_eq!(on[0]["data"]["entity"], "RK3", "on the risk: {}", on[0]);
}

/// RK1 and RK4 name a mitigation (not yet done), RK2 rates 6: none is W532, and RK3 is not once the threshold is
/// above its 350. The prose `mitigation` is no evidence: RK3 writing one is still W532.
#[test]
fn a_mitigated_or_low_risk_is_not_w532() {
    let w532 = |p: &Project| -> Vec<String> {
        p.findings()
            .into_iter()
            .filter(|(code, _)| code == "gsd/W532")
            .map(|(_, message)| message)
            .collect()
    };
    let p = project("governance");
    assert_eq!(w532(&p), [RK3_W532], "RK1, RK2 and RK4 are not W532");
    p.edit(
        "spec/risks.spec",
        "  milestone  v1.0\n",
        "  milestone  v1.0\n  mitigation \"finish a rotation before it starts the next\"\n",
    );
    assert_eq!(w532(&p), [RK3_W532], "prose is no mitigation");
    p.settings(serde_json::json!({ "specforge.risk_block_on": 400 }));
    assert_eq!(w532(&p), Vec::<String>::new(), "350 is under 400");
}

/// `risk_level` of `id`, `None` when absent.
fn risk_level(p: &Project, id: &str) -> Option<i64> {
    p.fields_of(id)["risk_level"].as_i64()
}

/// Phase 01 is reached by RK1 (175, through T1) and RK4 (125, through TR1); milestone v1.0 by its phase and by RK3
/// (350), which names it. Once plan 01-01 is complete, RK1 is mitigated at 35 and the phase drops to RK4's 125.
#[test]
fn phase_and_milestone_carry_their_risk_level() {
    let p = project("governance");
    assert_eq!(risk_level(&p, "v1.0/01"), Some(175));
    assert_eq!(risk_level(&p, "v1.0"), Some(350));
    let plans = p.read("spec/plans.spec");
    p.write("spec/plans.spec", &format!("{plans}{SUMMARY}"));
    assert_eq!(risk_level(&p, "v1.0/01"), Some(125));
    assert_eq!(risk_level(&p, "v1.0"), Some(350));
    p.edit("spec/risks.spec", "  milestone  v1.0\n", "");
    assert_eq!(risk_level(&p, "v1.0"), Some(125), "from its phase alone");
}

#[test]
fn a_phase_that_writes_risk_level_is_e082() {
    let p = project("governance");
    p.edit(
        "spec/v1.0.spec",
        "  slug             \"store\"\n",
        "  slug             \"store\"\n  risk_level       5\n",
    );
    let report = p.report();
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "E082" && message.contains("risk_level")),
        "{report:#?}"
    );
    assert_eq!(
        risk_level(&p, "v1.0/01"),
        Some(175),
        "the computed value stands"
    );
}

/// A task, a truth or an invariant reads back the risks that name it as `mitigates`: written by hand it is E079 naming
/// `risk.mitigated_by` with the edit, and a typed edit of it is refused `other_side`; the owning side writes.
#[test]
fn a_mitigates_written_on_a_task_is_e079_naming_mitigated_by() {
    let p = project("governance");
    p.edit(
        "spec/plans.spec",
        "  done   \"A key is encrypted at rest\"\n",
        "  done   \"A key is encrypted at rest\"\n  mitigates [RK2]\n",
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "{}", check.all());
    let report = p.report();
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "E079" && message.contains("risk.mitigated_by")),
        "{report:#?}"
    );
    assert!(check.all().contains("mitigated_by ["), "{}", check.all());

    let p = project("governance");
    let refused = p.run(&["link", "v1.0/01/01-01/TR1", "mitigates", "RK2"]);
    assert_ne!(refused.code, 0, "{}", refused.all());
    assert!(
        refused.all().contains("other_side") && refused.all().contains("mitigated_by"),
        "{}",
        refused.all()
    );
    let linked = p.run(&["link", "RK2", "mitigated_by", "v1.0/01/01-01/TR1"]);
    assert_eq!(linked.code, 0, "{}", linked.all());
    assert_eq!(
        p.report(),
        [("gsd/W532".to_string(), RK3_W532.to_string())],
        "the fixture's one finding, and nothing more"
    );
    let query = p.run(&[
        "query",
        "v1.0/01/01-01/TR1",
        "--depth",
        "1",
        "--format",
        "json",
    ]);
    assert!(
        query.stdout.contains("\"RK2\"") && query.stdout.contains("\"RK4\""),
        "the truth reads back both risks it mitigates: {}",
        query.all()
    );
}
