//! The verify rules no judged rule or declaration carries: `gsd/E503`, an override has its author and date;
//! `gsd/W504`, no more than three overrides in a phase; `gsd/E509`, authored UAT evidence that makes a session
//! unpassable (a skipped test with no reason, an issue with no report). A session's status is computed, so it cannot
//! be contradicted: E509 reports what the author left out.
//!
//! D-r39-1: every Error here is judged over authored entities. E509 leaves out a test a collector reported
//! (`source automated`), so a collector's plan can never newly trigger it.
//!
//! `gsd/W514` (a refactor `deviation` window while `workflow.windows_enforce` is off) is not built: a pass is not
//! sent the project's settings, and a setting gates a code only while it is on, never while it is off.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::plans::{link, of_kind, text};

/// More overrides than this in one verification is W504.
const MAX_OVERRIDES: usize = 3;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("verify_rules", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
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
