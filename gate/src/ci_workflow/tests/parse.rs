//! The workflow reader refuses what it cannot place rather than skipping it, and reads a comment as no part of a
//! line: a job or a key it misread would escape every check.

use super::*;

/// A `jobs:` line and a job header may carry a comment or trailing space, and each job keeps its own lines.
#[test]
fn a_comment_after_jobs_or_a_job_header_hides_no_job() {
    let workflow = "name: X\n\njobs:  # every job\n  first: # the first\n    runs-on: a\n  second:  \n    \
                    runs-on: b\n";
    let jobs = jobs(workflow);
    let read: Vec<(&str, Option<String>)> = jobs
        .iter()
        .map(|job| (job.id, job.key("runs-on").map(String::from)))
        .collect();
    assert_eq!(
        read,
        [("first", Some("a".into())), ("second", Some("b".into()))]
    );
}

/// A line at a job's indent that is no job header is refused, not appended to the job before it.
#[test]
#[should_panic(expected = "a line under `jobs:` at a job's indent is no job header")]
fn a_line_at_a_jobs_indent_that_is_no_header_panics() {
    let _ = jobs("jobs:\n  first:\n    runs-on: a\n  not a header\n    runs-on: b\n");
}

/// A comment ends a value, but a `#` inside quotes or not after a space is part of it.
#[test]
fn a_value_ends_at_its_comment() {
    let workflow = "jobs:\n  only:\n    runs-on: ubuntu-latest # the runner\n    steps: # the steps\n      \
                    - name: \"Fix #1\" # quoted\n        uses: Swatinem/rust-cache@v2\n        with: # inputs\n          \
                    shared-key: build # the build's\n          key: 'it''s # x' # single\n          \
                    prefix-key: a#b\n";
    let jobs = jobs(workflow);
    let job = &jobs[0];
    assert_eq!(job.key("runs-on").as_deref(), Some("ubuntu-latest"));
    let steps = job.steps();
    assert_eq!(steps.len(), 1);
    let step = &steps[0];
    assert_eq!(step.key("name").as_deref(), Some("Fix #1"));
    assert_eq!(step.with("shared-key").as_deref(), Some("build"));
    assert_eq!(step.with("key").as_deref(), Some("it's # x"));
    assert_eq!(step.with("prefix-key").as_deref(), Some("a#b"));
}

/// A quoted value reads as YAML reads it: `''` is one `'` in single quotes, and double quotes decode their escapes.
#[test]
fn a_quoted_value_reads_as_yaml_reads_it() {
    assert_eq!(unquote("'it''s'"), "it's");
    assert_eq!(unquote("''''"), "'");
    assert_eq!(
        unquote(r#""a\"b\\c\td\x41\u00e9\_\U0001F600\/""#),
        "a\"b\\c\tdA\u{e9}\u{a0}\u{1F600}/"
    );
    assert_eq!(unquote(r#""C:\\Users""#), r"C:\Users");
    for kept in ["plain", "'no''", r#""a\""#, r#""a" b"#, "\"", ""] {
        assert_eq!(unquote(kept), kept, "{kept}");
    }
    // A value with nothing to decode is the line's own text.
    assert!(matches!(unquote("'as is'"), Cow::Borrowed("as is")));
    assert!(matches!(unquote(r#""as is""#), Cow::Borrowed("as is")));
}

/// An escape YAML does not define is refused, not read as two characters.
#[test]
#[should_panic(expected = "a double-quoted value holds an escape YAML does not define")]
fn an_undefined_escape_panics() {
    let _ = unquote(r#""\q""#);
}

/// Two savers of one key are reported though one names it with a comment after it.
#[test]
fn a_commented_shared_key_is_still_a_duplicate_saver() {
    let workflow = cached(
        PREFIX_ENV,
        &[
            (
                "one",
                "",
                &[PREFIXED, "shared-key: build # the build's", ON_MAIN],
            ),
            ("two", "", &[PREFIXED, "shared-key: build", ON_MAIN]),
        ],
    );
    let found = key_findings("x.yml", &workflow);
    assert!(
        found.len() == 1
            && found[0].contains("the cache key `build` is saved by job `one` and job `two`"),
        "{found:?}"
    );
}

/// GitHub reads an action's owner and repository in any case, and so does the gate.
#[test]
fn a_rust_cache_step_is_one_in_any_case() {
    let workflow = "jobs:\n  only:\n    steps:\n      - uses: swatinem/Rust-Cache@v2\n      - uses: actions/checkout@v4\n";
    let jobs = jobs(workflow);
    let cache: Vec<bool> = jobs[0].steps().iter().map(Step::is_rust_cache).collect();
    assert_eq!(cache, [true, false]);
}
