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
