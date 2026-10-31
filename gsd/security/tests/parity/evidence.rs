//! For each code of `@gsd/security`: a test that makes it fire on a named project and a test that keeps it silent on
//! that project's twin (plan 16 T25). Both are `#[test]` fns under `tests/` whose name or body names the code;
//! `parity_rules.rs` checks that they exist, `parity_roster.rs` counts them.

pub const RULE_EVIDENCE: [(&str, &str, &str); 2] = [
    (
        "E101",
        "a_duplicate_is_one_e101_per_claiming_threat_and_the_supply_chain_row_never_counts",
        "no_duplicate_is_an_empty_list_and_a_renumber_silences_it",
    ),
    (
        "W301",
        "a_complete_phase_without_an_audit_says_missing",
        "a_verified_audit_is_silent",
    ),
];
