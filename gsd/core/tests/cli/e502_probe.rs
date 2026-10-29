//! The probe REVIEW-r40 M7 asked for (plan 07, T18 / D-r40-14), and the state it found: `gsd/E502` is not built, so a
//! `verified` records on an explicit truth whose recorded tests include a failing one.
//!
//! The reason E502 waits is NOT that E080 would refuse the `verified` (that refusal is what E502 is for). The probe of
//! the plan's stop rule is the other order, a failing report collected *after* a pass: it changes the truth's `tests`
//! input, so the pass goes stale and E502 (which needs a fresh passing judgement) cannot fire.

use super::harness::{Project, project};

const TRUTH: &str = "v1.0/01/01-01/TR1";
const FAILING: &str =
    "<testcase name=\"[gsd:01-01/TR1] fails\"><failure message=\"no\"/></testcase>";

fn report_failing(p: &Project) {
    p.write(
        "junit.xml",
        &format!("<?xml version=\"1.0\"?>\n<testsuites>\n<testsuite name=\"s\">\n{FAILING}\n</testsuite>\n</testsuites>\n"),
    );
    let run = p.collect(&["--runner", "gsd-junit", "--no-run"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

fn state(p: &Project) -> String {
    p.inputs(TRUTH, "E501")["verdict"]["state"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The fail-open that E502 exists to close: nothing refuses a `verified` over a failing test.
#[test]
fn a_verified_truth_with_a_failing_test_records_because_e502_is_not_built() {
    let p = project("execute");
    report_failing(&p);
    let run = p.record(TRUTH, "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p), "fresh");
}

/// T18's probe: a failing report collected after a pass stales it, so it never raises a new Error.
#[test]
fn a_failing_report_after_a_pass_stales_the_pass_and_is_never_refused() {
    let p = project("execute");
    assert_eq!(p.record(TRUTH, "E501", "verified", &[]).code, 0);
    assert_eq!(state(&p), "fresh");
    report_failing(&p);
    assert_eq!(state(&p), "stale");
}

#[test]
#[ignore = "E502 is not built (REVIEW-r40 M7): an explicit truth is verified only with a passing test"]
fn e502_refuses_a_verified_truth_with_a_failing_test() {
    let p = project("execute");
    report_failing(&p);
    let run = p.record(TRUTH, "E501", "verified", &[]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E502"), "{}", run.all());
}
