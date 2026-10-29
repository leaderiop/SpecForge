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

const RESOLVED: &[Field] = &[str("by"), ts("at"), date("on")];
const WITH_TS_MEMBERS: Kind = Kind::new(
    "stamped_blocks",
    "an authored block, an authored list and a harness block, each with a timestamp member",
    &[
        block("resolved", RESOLVED),
        blocks("history", RESOLVED),
        block("completion", RESOLVED).h(),
    ],
);

#[test]
fn an_authored_ts_member_gets_e107_and_a_harness_blocks_member_does_not() {
    let rules = rules_of(&[&WITH_TS_MEMBERS]);
    let mut fields: Vec<&str> = rules
        .iter()
        .map(|r| r["field"].as_str().expect("a field"))
        .collect();
    fields.sort_unstable();
    assert_eq!(fields, ["history.at", "resolved.at"], "{rules:?}");
    assert!(
        rules.iter().all(|r| r["code"].to_string().contains("E107")),
        "{rules:?}"
    );
}

#[test]
fn a_local_time_is_e107() {
    let pattern = regex::Regex::new(RFC3339_UTC).expect("a pattern");
    for refused in [
        "2026-10-28T09:30:00+02:00",
        "2026-10-28 09:30:00Z",
        "2026-10-28",
        "2026-10-28T09:30Z",
        "2026-10-28T09:30:00",
        "2026-10-28T09:30:00z",
        "\u{662}\u{660}\u{662}\u{666}-10-28T09:30:00Z",
        "2026-13-45T25:61:61Z",
        "2026-00-10T09:30:00Z",
    ] {
        assert!(!pattern.is_match(refused), "{refused} is refused");
    }
    for accepted in ["2026-10-28T09:30:00Z", "2026-10-28T09:30:00.123Z"] {
        assert!(pattern.is_match(accepted), "{accepted} is accepted");
    }
}

const PEER: &[Field] = &[rf("peer", "stamped").r(), str("reason")];
const SETTLED: &[Field] = &[en("rating", &["a"]), rf("settled_by", "stamped")];
const WITH_REFERENCE_MEMBERS: Kind = Kind::new(
    "linked",
    "a kind with a reference member of a block list and of a block",
    &[blocks("coupling", PEER), block("reversibility", SETTLED)],
);

#[test]
fn a_reference_member_declares_its_edge() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@gsd/test", "0.0.0"));
    declare(&mut c, &[&WITH_REFERENCE_MEMBERS]);
    let text = MockHost::new(c).describe_json("edges").expect("edges");
    let edges: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let mut labels: Vec<String> = edges["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|e| e["label"].as_str().unwrap_or_default().to_string())
        .collect();
    labels.sort();
    assert_eq!(
        labels,
        ["LinkedCouplingPeer", "LinkedReversibilitySettledBy"],
        "{edges}"
    );
}

const CHECKED: &[Field] = &[str("ref").r(), en("status", &["pass", "fail"]).c()];
const WITH_COMPUTED_MEMBER: Kind = Kind::new(
    "checked",
    "a computed field, an authored block with a computed member and a computed block",
    &[
        str("duration").c(),
        blocks("checks", CHECKED),
        block("actuals", &[int("tasks")]).c(),
    ],
);

#[test]
fn a_computed_member_is_listed_as_kind_field_member() {
    assert_eq!(
        computed(&[&WITH_COMPUTED_MEMBER]),
        [
            "checked.duration",
            "checked.checks.status",
            "checked.actuals"
        ]
    );
}
