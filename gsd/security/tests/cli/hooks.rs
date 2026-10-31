//! `specforge gsd security-hooks <point>`: the security capability's hooks, as `loop render-hooks` answered.

use serde_json::{Value, json};

use super::both;

fn hooks(p: &super::harness::Project, point: &str) -> Value {
    let run = p.run(&["gsd", "security-hooks", point, "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

#[test]
fn ship_pre_is_the_gate_with_upstreams_check() {
    let out = hooks(&both(), "ship:pre");
    assert_eq!(
        out,
        json!({
            "activeHooks": [{
                "capId": "security", "kind": "gate", "when": "workflow.security_enforcement",
                "check": { "predicate": { "kind": "artifact-frontmatter-equals", "artifact": "SECURITY.md",
                                          "field": "threats_open", "equals": 0 } },
                "blocking": true, "onError": "halt"
            }],
            "skippedHooks": []
        })
    );
}

#[test]
fn plan_pre_is_the_contribution_with_its_config_values_and_the_repointed_fragment() {
    let out = hooks(&both(), "plan:pre");
    let hook = &out["activeHooks"][0];
    assert_eq!(hook["kind"], "contribution");
    assert_eq!(hook["into"], "planner");
    assert_eq!(
        hook["configValues"],
        json!({ "security_asvs_level": 1, "security_block_on": "high" })
    );
    assert_eq!(hook["consumes"], json!(["CONTEXT.md"]));
    let fragment = hook["fragment"]["inline"].as_str().expect("a fragment");
    assert!(
        fragment.contains("`threat`") && !fragment.contains("PLAN.md"),
        "{fragment}"
    );
}

#[test]
fn verify_post_is_the_secure_phase_step() {
    let out = hooks(&both(), "verify:post");
    assert_eq!(
        out["activeHooks"],
        json!([{
            "capId": "security", "kind": "step", "ref": { "skill": "secure-phase" },
            "when": "workflow.security_enforcement", "produces": ["SECURITY.md"], "consumes": ["SUMMARY.md"],
            "onError": "halt"
        }])
    );
}

#[test]
fn settings_are_read_for_when_and_config_values() {
    let p = both();
    p.settings_of(
        "@gsd/security",
        json!({ "workflow.security_asvs_level": 2, "workflow.security_block_on": "medium" }),
    );
    assert_eq!(
        hooks(&p, "plan:pre")["activeHooks"][0]["configValues"],
        json!({ "security_asvs_level": 2, "security_block_on": "medium" })
    );
    p.settings_of(
        "@gsd/security",
        json!({ "workflow.security_enforcement": false }),
    );
    for point in ["plan:pre", "verify:post", "ship:pre"] {
        assert_eq!(
            hooks(&p, point),
            json!({ "activeHooks": [], "skippedHooks": [] }),
            "{point}"
        );
    }
}

#[test]
fn another_point_has_no_hook_and_a_bad_point_is_refused() {
    assert_eq!(hooks(&both(), "execute:pre")["activeHooks"], json!([]));
    let run = both().run(&["gsd", "security-hooks", "nowhere", "--format", "json"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("Invalid loop point"), "{}", run.all());
}
