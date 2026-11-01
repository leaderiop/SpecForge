//! The plan 05 re-review (2026-11-01): the lines it found reading the wrong entities, run as the asset holds them
//! under bash with no jq on the `PATH`. The taskboard fixture: phase 02 depends on phase 01, so a query of phase 01
//! at depth 2 also reaches 02's plan 02-01.

use super::{Shell, asset};
use crate::harness::taskboard;

/// The inline code run of `line` that starts with `start`, without its backticks.
fn inline<'a>(line: &'a str, start: &str) -> &'a str {
    let at = line
        .find(&format!("`{start}"))
        .unwrap_or_else(|| panic!("no `{start}… run in {line}"));
    let rest = &line[at + 1..];
    &rest[..rest.find('`').expect("a closing backtick")]
}

/// Item 8: the review's plans and the learnings' plans are the phase's own, the selection the manifest and
/// `plan-coverage` use, not every plan a depth-2 query of the phase reaches.
#[test]
fn the_review_and_the_learnings_read_the_phases_own_plans() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    for (path, marker) in [
        ("gsd-core/workflows/review.md", "For each plan of the phase"),
        (
            "gsd-core/workflows/extract-learnings.md",
            "- the phase's plans",
        ),
    ] {
        let text = asset(path);
        let line = text
            .lines()
            .find(|l| l.trim_start().starts_with(marker))
            .unwrap_or_else(|| panic!("{path}: no line starts with {marker}"));
        let select = inline(line, "specforge query \"$PHASE_ID\"");
        let (out, err, _) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{select}"));
        assert!(
            !out.contains("02-01"),
            "{path}: {select} reaches 02-01:\n{out}{err}"
        );
        assert!(
            out.contains("v1.0/01/01-01") && out.contains("v1.0/01/01-02"),
            "{path}: {select}:\n{out}{err}"
        );
    }
    // the coverage check matches each id as text, it escapes no regex
    assert!(!asset("gsd-core/workflows/review.md").contains("regex-escaped"));
}

/// Item 10: complete-milestone's readiness compares the milestone's computed phase counts; a query that fails (an
/// unknown milestone, a graph without its computed values) prints two empty picks, which once compared equal and set
/// `ALL_PHASES_VERIFIED=true`. Only two counts that SpecForge answered, and agree, make it true.
#[test]
fn a_failed_readiness_query_does_not_verify_the_milestone() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let text = asset("gsd-core/workflows/complete-milestone.md");
    let line = super::line(&text, "ALL_PHASES_VERIFIED=false;");
    // v1.0: phase 01 is complete, 02 is not
    for (milestone, want) in [("v9.9", "false"), ("v1.0", "false")] {
        let (out, err, _) = sh.run_in(
            p.root(),
            &format!("MILESTONE={milestone}\n{line}\nprintf '%s' \"$ALL_PHASES_VERIFIED\""),
        );
        assert_eq!(out, want, "{milestone}: {err}");
    }
    // two counts that agree (a stand-in answers both): verified
    let (out, err, _) = sh.run_in(
        p.root(),
        &format!("MILESTONE=v1.0\nspecforge() {{ echo 2; }}\n{line}\nprintf '%s' \"$ALL_PHASES_VERIFIED\""),
    );
    assert_eq!(out, "true", "{err}");
}

/// Item 12: prose that described upstream's machinery: quick-batch's parse step has no CLI error to print and no
/// dependency DAG (the list is split by the shell, and every item starts in wave 0), and pr-branch reads no
/// `planning.sub_repos` setting, so no exit code says the key is absent.
#[test]
fn the_batch_and_sub_repo_prose_says_what_runs() {
    for (path, stale) in [
        (
            "gsd-core/workflows/quick-batch/steps/batch-init.md",
            &["Print the CLI's error", "dependency DAG"][..],
        ),
        (
            "gsd-core/workflows/pr-branch.md",
            &[
                "using the canonical key path",
                "A non-zero exit code means the key is absent",
            ][..],
        ),
    ] {
        let text = asset(path);
        for s in stale {
            assert!(!text.contains(s), "{path} still says {s:?}");
        }
    }
}
