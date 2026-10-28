use specforge_extension_sdk::testing::MockHost;

use super::*;

const WITH_AUTHORED_TS: Kind = Kind::new(
    "stamped",
    "a kind with one authored and one computed timestamp",
    &[ts("stamped_at"), ts("updated").c()],
);

fn rules_of(kinds: &[&Kind]) -> Vec<serde_json::Value> {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@gsd/test", "0.0.0"));
    declare(&mut c, kinds);
    let text = MockHost::new(c)
        .describe_json("validation_rules")
        .expect("rules");
    let rules: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    rules["items"].as_array().cloned().unwrap_or_default()
}

#[test]
fn an_authored_ts_field_gets_e107_and_a_computed_one_does_not() {
    let rules = rules_of(&[&WITH_AUTHORED_TS]);
    assert_eq!(rules.len(), 1, "{rules:?}");
    assert_eq!(rules[0]["target_kind"], "stamped");
    assert_eq!(rules[0]["field"], "stamped_at");
    assert!(rules[0]["code"].to_string().contains("E107"), "{rules:?}");
}
