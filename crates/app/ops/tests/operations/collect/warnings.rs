//! A collector's warnings (ADR 0389): each line its export returns under `warnings` is one W203 naming the
//! collector, escaped, at most 100 per collector and then one counting the rest.

use specforge_common::{Diagnostic, Severity};
use specforge_test_macros::test as specforge_test;

use super::{Fixture, Machine, Runner};

/// Collect `runner` from its existing report: the W203 diagnostics of the outcome.
fn warned(runner: Runner) -> Vec<Diagnostic> {
    let fixture = Fixture::new(vec![runner]);
    fixture.write("out/acme.json", "{}");
    let outcome = fixture
        .no_run(Some("acme"), &Machine::new())
        .expect("the collect runs");
    outcome
        .diagnostics
        .into_iter()
        .filter(|d| d.code == "W203")
        .collect()
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_warning_line_is_one_w203_naming_the_collector() {
    let line = "skipped 1 file outside the project: /elsewhere/a.rs";
    let warned = warned(Runner::new("acme").warning(line));
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert_eq!(warned[0].severity, Severity::Warning);
    let message = &warned[0].message;
    assert!(message.contains("'acme'"), "{message}");
    assert!(message.contains("@acme/acme"), "{message}");
    assert!(message.contains(line), "{message}");
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_warning_line_prints_escaped() {
    let warned = warned(Runner::new("acme").warning("\u{1b}[31mred"));
    assert_eq!(warned.len(), 1, "{warned:?}");
    assert!(!warned[0].message.contains('\u{1b}'), "{:?}", warned[0]);
    assert!(
        warned[0].message.contains("\\u{1b}[31mred"),
        "{:?}",
        warned[0]
    );
}

#[specforge_test(requirement = "FEATURE-128")]
fn past_a_hundred_warning_lines_one_w203_counts_the_rest() {
    let runner = (0..150).fold(Runner::new("acme"), |runner, i| {
        runner.warning(&format!("line {i}"))
    });
    let warned = warned(runner);
    assert_eq!(warned.len(), 101);
    assert!(warned[99].message.ends_with("line 99"), "{:?}", warned[99]);
    assert!(warned[100].message.contains("50 more"), "{:?}", warned[100]);
}

#[specforge_test(requirement = "FEATURE-128")]
fn a_collector_answering_no_warnings_gives_no_w203() {
    assert!(warned(Runner::new("acme").proving("a", "t")).is_empty());
}
