//! The verify rules no judged rule or declaration carries: `gsd/E502`, a `verified` verdict is not contradicted by the
//! truth's recorded tests; `gsd/E503`, an override has its author and date; `gsd/W504`, no more than three overrides in a phase; `gsd/E509`, authored UAT evidence that makes a session
//! unpassable (a skipped test with no reason, an issue with no report). A session's status is computed, so it cannot
//! be contradicted: E509 reports what the author left out.
//!
//! D-r39-1: E503 and E314 read only authored data. E509 reads `uat_test.result` and `reported`, which a command or
//! a collector writes: it leaves out a test a collector reported (`source automated`), so a collector's plan can never
//! newly trigger it. A future writer of `result` without a `source`, or an `issue` recorded without `reported`, would.
//!
//! `gsd/E502` needs a fresh passing `gsd/E501` judgement and a recorded report, and both of its inputs are inputs of
//! E501 (`truth self`, `tests test(self)`): a new or changed result stales the verdict first, so a recorded test never
//! makes E502 appear. It appears only when a verifier records `verified` over results that contradict it, and that
//! record is refused (E080 names it): the verifier records `failed` instead. Silent without a report (I205 says so).
//!
//! `gsd/W514` (a refactor `deviation` window while `workflow.windows_enforce` is off) is not built: a pass is not
//! sent the project's settings, and a setting gates a code only while it is on, never while it is off (it needs
//! `SettingDescriptor.gates_when_on`, kernel).

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::plans::{link, of_kind, text};

/// More overrides than this in one verification is W504.
const MAX_OVERRIDES: usize = 3;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("verify_rules", |p| {
        p.phase("check").reads_tests().run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = e502(input);
    let mut per_verification: BTreeMap<&str, usize> = BTreeMap::new();
    for o in of_kind(input, "override") {
        let missing: Vec<&str> = ["by", "at"]
            .into_iter()
            .filter(|field| text(o, field).is_none())
            .collect();
        if !missing.is_empty() {
            found.push(
                PassDiagnostic::new(
                    "E503",
                    PassSeverity::Error,
                    format!(
                        "override '{}' has no {}: an override records who accepted the truth and when",
                        o.id,
                        missing.join(" and ")
                    ),
                )
                .with_suggestion("write `by \"<your name>\"` and `at <date>` on the override")
                .with_entity(&o.id),
            );
        }
        if let Some(v) = link(input, o, "verification") {
            *per_verification.entry(v).or_default() += 1;
        }
    }
    for (verification, n) in per_verification
        .into_iter()
        .filter(|(_, n)| *n > MAX_OVERRIDES)
    {
        found.push(
            PassDiagnostic::new(
                "W504",
                PassSeverity::Warning,
                format!("verification '{verification}' has {n} overrides: more than {MAX_OVERRIDES} hides what the verifier found"),
            )
            .with_suggestion("fix the truths the verifier failed, or defer them, rather than overriding them")
            .with_entity(verification),
        );
    }
    for t in of_kind(input, "uat_test").filter(|t| text(t, "source") != Some("automated")) {
        let gap = match text(t, "result") {
            Some("skipped") if text(t, "reason").is_none() => Some("is skipped with no reason"),
            Some("issue") if text(t, "reported").is_none() => {
                Some("is an issue with nothing reported")
            }
            _ => None,
        };
        if let Some(gap) = gap {
            found.push(
                PassDiagnostic::new("E509", PassSeverity::Error, format!("UAT test '{}' {gap}", t.id))
                    .with_suggestion("a skipped test needs its reason (`Deferred follow-up…` passes) and an issue the person's words in `reported`")
                    .with_entity(&t.id),
            );
        }
    }
    found
}

/// E502: an explicit truth whose fresh passing E501 verdict stands over recorded tests that hold no passed result or a
/// failed one. A backstop truth is never E502, and no report is no finding.
fn e502(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(report) = &input.test_results else {
        return Vec::new();
    };
    of_kind(input, "truth")
        .filter(|t| text(t, "verification") == Some("explicit"))
        .filter(|t| {
            t.judgements
                .iter()
                .any(|j| j.rule == "gsd/E501" && j.state == "fresh" && j.passed)
        })
        .filter_map(|t| {
            let tests = report.of(&t.id);
            let failed = tests.iter().any(|r| r.status == TestStatus::Failed);
            let passed = tests.iter().any(|r| r.status == TestStatus::Passed);
            let why = if failed {
                "a test recorded for it failed"
            } else if !passed {
                "no passing test is recorded for it"
            } else {
                return None;
            };
            Some(
                PassDiagnostic::new(
                    "E502",
                    PassSeverity::Error,
                    format!("explicit truth '{}' is verified, but {why}", t.id),
                )
                .with_suggestion(
                    "record `failed` for the truth, or link a passing test to it with `[gsd:<id>]`",
                )
                .with_entity(&t.id),
            )
        })
        .collect()
}
