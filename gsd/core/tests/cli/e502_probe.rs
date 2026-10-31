//! `gsd/E502` (REVIEW-r40 M7, built in K17): an explicit truth is verified only with a passing test recorded for it. The
//! pass needs a fresh passing `gsd/E501` judgement and a recorded report, and both of E501's test inputs stale the
//! verdict first, so a recorded result never makes E502 appear (D-r39-1); it appears only when a verifier records
//! `verified` over results that contradict it, and that record is refused (E080 names `gsd/E502`). The verifier records
//! `failed` instead, which the host lets it do since ADR 0297.

use super::harness::{Project, project};

const TRUTH: &str = "v1.0/01/01-01/TR1";
const FAILING: &str =
    "<testcase name=\"[gsd:01-01/TR1] fails\"><failure message=\"no\"/></testcase>";
const PASSING: &str = "<testcase name=\"[gsd:01-01/TR1] works\"/>";

fn report(p: &Project, case: &str) {
    p.write(
        "junit.xml",
        &format!("<?xml version=\"1.0\"?>\n<testsuites>\n<testsuite name=\"s\">\n{case}\n</testsuite>\n</testsuites>\n"),
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

fn e502(p: &Project) -> usize {
    p.report_all()
        .iter()
        .filter(|(c, _)| c == "gsd/E502")
        .count()
}

/// The honest verifier: a `verified` over a failing test is refused, naming the rule, and nothing is written.
#[test]
fn a_verified_verdict_the_tests_contradict_is_refused_naming_e502() {
    let p = project("execute");
    report(&p, FAILING);
    let spec = p.read("spec/plans.spec");
    let run = p.record(TRUTH, "E501", "verified", &[]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E080"), "{}", run.all());
    assert!(run.all().contains("gsd/E502"), "{}", run.all());
    assert_eq!(p.read("spec/plans.spec"), spec);
}

/// What the verifier records instead: `failed` over the same results.
#[test]
fn a_failed_verdict_the_tests_explain_records() {
    let p = project("execute");
    report(&p, FAILING);
    let run = p.record(TRUTH, "E501", "failed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(e502(&p), 0);
}

#[test]
fn a_verified_verdict_with_a_passing_linked_test_records() {
    let p = project("execute");
    report(&p, PASSING);
    let run = p.record(TRUTH, "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p), "fresh");
    assert_eq!(e502(&p), 0);
}

#[test]
fn e502_is_silent_without_a_report() {
    let p = project("execute");
    let run = p.record(TRUTH, "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(e502(&p), 0);
}

#[test]
fn an_agent_cannot_change_a_truths_verification() {
    let p = project("execute");
    let before = p.read("spec/plans.spec");
    let run = p.run(&["set", TRUTH, "verification", "backstop"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("[human_only]"), "{}", run.all());
    assert_eq!(p.read("spec/plans.spec"), before);
}

#[test]
fn a_backstop_truth_is_never_e502() {
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "verification explicit",
        "verification backstop",
    );
    report(&p, FAILING);
    let run = p.record(TRUTH, "E501", "verified", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(e502(&p), 0);
}

/// A failing report collected after a pass stales it, so E502 (which needs a fresh pass) is never raised by a result.
#[test]
fn a_failing_report_after_a_pass_stales_the_pass_and_is_never_refused() {
    let p = project("execute");
    assert_eq!(p.record(TRUTH, "E501", "verified", &[]).code, 0);
    assert_eq!(state(&p), "fresh");
    report(&p, FAILING);
    assert_eq!(state(&p), "stale");
    assert_eq!(e502(&p), 0);
}
