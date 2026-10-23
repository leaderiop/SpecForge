//! The builtin edge rules beside references that do not resolve, through
//! the real software and product extensions (ADR 0232): one mistyped id
//! is E003 alone.

use specforge_project::testing::TestProject;
use specforge_test_macros::test as spec;

/// One entity per builtin edge rule, each writing its rule's field with an
/// id nothing declares; `quiet` writes no `features` at all.
const MISTYPED: &str = "\
behavior pay \"Pay\" {\n  category command\n  features [nope_f]\n  contract \"The system MUST take payment\"\n}\n\n\
behavior quiet \"Quiet\" {\n  category command\n  contract \"The system MUST stay quiet\"\n}\n\n\
deliverable app \"App\" {\n  artifact_type cli\n  journeys [nope_j]\n  modules [nope_m]\n}\n\n\
module core \"Core\" {\n  features [nope_mf]\n}\n\n\
journey checkout \"Checkout\" {\n  flow [\"step\"]\n  channels [nope_c]\n}\n\n\
release v1 \"V1\" {\n  version \"1.0.0\"\n  deliverables [nope_d]\n  milestones [nope_ms]\n}\n\n\
term ledger \"Ledger\" {\n  definition \"a book of accounts\"\n  see_also [nope_t]\n}\n";

/// The builtins' edge rules: software's W001 and product's W043, W046,
/// I067, I055, I082, I083 (`no_outgoing_edges`), I010 (`no_edges`).
const EDGE_RULES: [&str; 8] = [
    "W001", "W043", "W046", "I067", "I055", "I082", "I083", "I010",
];

#[spec(
    behavior = "execute_validation_pattern",
    verify = "one mistyped id is E003 alone: no builtin edge rule reports the entity whose field writes it"
)]
fn one_mistyped_id_is_e003_alone() {
    let compiled = TestProject::new()
        .enabling(&["@specforge/software", "@specforge/product"])
        .file("main.spec", MISTYPED)
        .compile();
    let mut reported: Vec<(String, String)> = compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code == "E003" || EDGE_RULES.contains(&d.code.as_str()))
        .map(|d| (d.code.clone(), d.message.clone()))
        .collect();
    reported.sort();
    let e003 = |id: &str, entity: &str| {
        (
            "E003".to_string(),
            format!("unresolved reference '{id}' in entity '{entity}'"),
        )
    };
    assert_eq!(
        reported,
        [
            e003("nope_c", "checkout"),
            e003("nope_d", "v1"),
            e003("nope_f", "pay"),
            e003("nope_j", "app"),
            e003("nope_m", "app"),
            e003("nope_mf", "core"),
            e003("nope_ms", "v1"),
            e003("nope_t", "ledger"),
            // `quiet` writes no feature: its W001 is no cascade.
            (
                "W001".to_string(),
                "behavior 'quiet' does not implement any feature".to_string()
            ),
        ]
    );
}
