use super::problem;

#[test]
fn a_missing_statement_is_a_problem() {
    assert_eq!(problem("npm test", None), Some("states no `fails_when`"));
}

#[test]
fn an_empty_statement_is_a_problem() {
    assert_eq!(
        problem("npm test", Some("  ")),
        Some("has an empty `fails_when`")
    );
}

#[test]
fn a_placeholder_is_a_problem_in_any_case() {
    for text in [
        "TBD", "n/a", " - ", "?", "None", "unknown", "TODO", "NA", "tba",
    ] {
        assert_eq!(
            problem("npm test", Some(text)),
            Some("has a placeholder `fails_when`"),
            "{text}"
        );
    }
}

#[test]
fn a_vacuous_but_real_statement_is_clean() {
    assert_eq!(problem("npm test", Some("TBD in the harness output")), None);
    assert_eq!(problem("npm test", Some("it fails")), None);
}

#[test]
fn the_wave_zero_sentinel_has_no_failure_to_state() {
    assert_eq!(problem("MISSING - Wave 0 creates it", None), None);
    assert_eq!(
        problem("MISSING=1 npm test", None),
        Some("states no `fails_when`")
    );
}
