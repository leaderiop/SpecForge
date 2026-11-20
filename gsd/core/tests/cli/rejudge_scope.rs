//! The re-verification scope over every single-file edit of the taskboard twin (ADR 0379): after each edit,
//! `specforge gsd re-verify <phase> --stale-only` lists exactly the verdicts `specforge query <entity> --inputs <rule>`
//! says are not fresh, and a third source, the spec's own fields ([`Project::reads`]), agrees with both. The value is
//! computed from what `specforge` prints, never typed; two controls show a listing that misses and one that
//! over-reaches move it off 1.0.

use std::collections::BTreeSet;

use serde_json::json;

use super::harness::{Project, Verdict, taskboard};

const PHASE: &str = "v1.0/01";

/// The taskboard twin as committed, under gsd-strict (the shipped stance; re-verification is not gated).
fn strict_twin() -> Project {
    let p = taskboard();
    p.settings(json!({ "profile": "gsd-strict" }));
    p
}

/// The `(rule, entity)` of each verdict `re-verify <phase>` lists, all of them or the ones that are not fresh.
fn listed(p: &Project, stale_only: bool) -> BTreeSet<Verdict> {
    let answer = p.re_verify(PHASE, stale_only);
    answer["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            let text = |key: &str| {
                i[key]
                    .as_str()
                    .unwrap_or_else(|| panic!("{key}: {answer:#}"))
                    .to_string()
            };
            (text("rule"), text("entity"))
        })
        .collect()
}

/// The phase's verdicts, asserted to be the seven verifier verdicts of its must-haves.
fn universe(p: &Project) -> BTreeSet<Verdict> {
    let universe = listed(p, false);
    assert_eq!(universe.len(), 7, "{universe:#?}");
    assert!(
        universe.iter().all(|(rule, _)| rule == "gsd/E501"),
        "{universe:#?}"
    );
    universe
}

/// A file every verdict but one reads, and one no verdict reads.
fn two_edits() -> Vec<String> {
    vec!["README.md".to_string(), "src/tasks.js".to_string()]
}

#[test]
fn re_verification_lists_exactly_the_stale_verdicts_after_any_single_file_edit() {
    let p = strict_twin();
    let universe = universe(&p);
    let files = p.fixture_files();
    let scope = p.rejudge_scope(&files, &universe, &|p| listed(p, true));
    let line = scope.line("taskboard");
    eprintln!("{line}");
    // The line ADR 0379 quotes; the edits are the fixture's files, one each.
    assert_eq!(
        line,
        format!(
            "re-verification scope on the taskboard twin: 12 re-judged of 12 stale over {} single-file edits (0 \
             over-judged, 0 under-judged): 1.0",
            files.len()
        )
    );
    assert!(scope.exact(), "{scope:#?}");
    assert_eq!(scope.ratio(), 1.0);
    assert_eq!(scope.edits, files.len());
    // Both cases of the measure occur: an edit that stales nothing, and one that stales some but not all.
    assert!(scope.quiet >= 1 && scope.partial >= 1, "{scope:#?}");
    // The independent oracle: each edit stales exactly the verdicts the spec says read the file.
    let reads: Vec<(Verdict, BTreeSet<String>)> = universe
        .iter()
        .map(|(rule, entity)| ((rule.clone(), entity.clone()), p.reads(rule, entity)))
        .collect();
    for file in &files {
        let expected: BTreeSet<Verdict> = reads
            .iter()
            .filter(|(_, read)| read.contains(file))
            .map(|(verdict, _)| verdict.clone())
            .collect();
        assert_eq!(scope.per_edit[file], expected, "{file}");
    }
    let tasks: BTreeSet<Verdict> = [
        "v1.0/01/01-01/AR1",
        "v1.0/01/01-01/KL1",
        "v1.0/01/01-01/TR1",
        "v1.0/01/01-02/KL1",
        "v1.0/01/01-02/TR1",
        "v1.0/01/01-02/TR2",
    ]
    .iter()
    .map(|e| ("gsd/E501".to_string(), e.to_string()))
    .collect();
    assert_eq!(scope.per_edit["src/tasks.js"], tasks);
}

/// The control of a listing that misses: one taken before the edit names nothing the edit staled.
#[test]
fn a_listing_taken_before_the_edit_scores_below_one() {
    let p = strict_twin();
    let universe = universe(&p);
    let before = listed(&p, true);
    let scope = p.rejudge_scope(&two_edits(), &universe, &|_| before.clone());
    eprintln!(
        "control, a listing taken before the edit: {}",
        scope.line("taskboard")
    );
    assert!(scope.ratio() < 1.0, "{scope:#?}");
    assert!(!scope.under.is_empty() && !scope.exact(), "{scope:#?}");
}

/// The control of a listing that over-reaches: re-verifying the whole phase names verdicts the edit left fresh.
#[test]
fn re_verifying_the_whole_phase_scores_above_one() {
    let p = strict_twin();
    let universe = universe(&p);
    let scope = p.rejudge_scope(&two_edits(), &universe, &|p| listed(p, false));
    eprintln!(
        "control, the whole phase re-verified: {}",
        scope.line("taskboard")
    );
    assert!(scope.ratio() > 1.0, "{scope:#?}");
    assert!(!scope.over.is_empty() && !scope.exact(), "{scope:#?}");
}
