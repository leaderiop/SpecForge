//! `injection-blocking-config.test.cjs` (#1577), accounted case by case: each of its five cases is ported, and
//! `tests/cli/injection.rs` runs it. Core's own suite skips the file with "@gsd/security's".

#[allow(
    dead_code,
    reason = "core's accounting serves every suite; this one uses `account`"
)]
#[path = "../../core/src/hooks/accounting.rs"]
mod accounting;

const BLOCKING_CONFIG: &str =
    include_str!("../../parity/upstream/v1.18.0/tests/injection-blocking-config.test.cjs");

/// `(upstream case, the test of tests/cli/injection.rs that runs it)`.
const PORTED: &[(&str, &str)] = &[
    (
        "isValidConfigKey accepts security.injection_blocking",
        "the_key_is_declared_and_defaults_to_false",
    ),
    (
        "bare security section is not a settable leaf key",
        "the_bare_security_section_is_not_a_setting",
    ),
    (
        "CONFIG_DEFAULTS ships injection_blocking = false (advisory by default)",
        "the_key_is_declared_and_defaults_to_false",
    ),
    (
        "config-set writes the nested shape the hook reads, and round-trips",
        "set_then_read_back_and_the_scanner_sees_it",
    ),
    (
        "a fresh project has no injection_blocking key \u{2014} hook sees absent \u{2192} advisory",
        "a_fresh_project_has_no_value_so_the_scanner_is_advisory",
    ),
];

#[test]
fn the_five_cases_of_1577_are_ported() {
    let ported: Vec<String> = PORTED.iter().map(|(case, _)| (*case).to_string()).collect();
    accounting::account(BLOCKING_CONFIG, &ported, &[]);
    let tests = include_str!("cli/injection.rs");
    for (case, test) in PORTED {
        assert!(
            tests.contains(&format!("fn {test}()")),
            "{case}: no test {test}"
        );
    }
}
