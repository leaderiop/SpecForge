//! R1's exit, end to end on the taskboard twin: a commit whose `Gsd-Task:` trailer names `01-01/T1`, a collect of the
//! twin's tests, and `specforge trace TASK-01` prints the requirement's thread from plan to verdict, one line. The
//! thread is SpecForge's own (gsd-tools has no such command), so its golden is
//! `tests/golden/trace/taskboard-TASK-01.txt`, not one of `goldens/taskboard/`, and `parity_scenario` does not list
//! it. The commit's sha is the one varying part: it is replaced by `<sha>` before the comparison.

use super::harness::{Project, Run, judged, taskboard};
use super::node::node_test_runs;

/// What `specforge trace TASK-01` prints once the thread is collected, its sha normalised.
const THREAD: &str = include_str!("../golden/trace/taskboard-TASK-01.txt");

/// The taskboard twin with one commit: `package.json` and `src/tasks.js` as the fixture has them (so no verdict's
/// `files` input moves), under a subject that names no plan, so only its `Gsd-Task:` trailer can link it to T1 of
/// `01-01`. Answers the project and the commit's full sha.
fn thread_twin() -> (Project, String) {
    let p = taskboard();
    let files = ["package.json", "src/tasks.js"].map(|path| (path, p.read(path)));
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (*path, text.as_str()))
        .collect();
    let sha = p.git_commit(
        "feat: the task model\n\nGsd-Task: 01-01/T1",
        &files,
        "2026-10-28T09:00:00Z",
    );
    (p, sha)
}

/// [`thread_twin`] with its tests collected from the recorded JUnit report and its commits from git, as a plain
/// `collect` does where node does not run.
fn collected_thread() -> (Project, String) {
    let (p, sha) = thread_twin();
    succeeds(&p.collect_node_test_golden());
    succeeds(&p.collect(&["--runner", "gsd-git"]));
    (p, sha)
}

fn succeeds(run: &Run) {
    assert_eq!(run.code, 0, "{}", run.all());
}

/// `specforge trace TASK-01`'s output, `sha`'s 12-digit name replaced by `<sha>`. No other run of 12 hex digits may be
/// left: a sha the replacement missed is a diff, never a pass.
fn traced(p: &Project, sha: &str) -> String {
    let run = p.run(&["trace", "TASK-01"]);
    succeeds(&run);
    let line = run.stdout.replace(&sha[..12], "<sha>");
    let longest_hex = line
        .split(|c: char| !c.is_ascii_hexdigit())
        .map(str::len)
        .max()
        .unwrap_or(0);
    assert!(longest_hex < 12, "a hash is left in {line}");
    line
}

/// The codes of `specforge check --format json`, nothing left out: the twin's harness quiets `code/W102`.
fn raw_codes(p: &Project) -> Vec<String> {
    p.checked().into_iter().map(|(code, _)| code).collect()
}

fn count(codes: &[String], code: &str) -> usize {
    codes.iter().filter(|c| *c == code).count()
}

#[test]
fn the_twin_checks_green_before_any_collect() {
    let p = taskboard();
    p.check_exact(&[]);
    let codes = raw_codes(&p);
    // Each of the three proofs names a test no collect has stored: W102 once per proof, and no error.
    assert_eq!(count(&codes, "code/W102"), 3, "{codes:?}");
    assert_eq!(
        (count(&codes, "code/E101"), count(&codes, "E003")),
        (0, 0),
        "{codes:?}"
    );
    let run = p.run(&["trace", "TASK-01"]);
    succeeds(&run);
    assert!(run.stdout.contains("incomplete thread"), "{}", run.all());
    assert!(!run.stdout.contains("(passed)"), "{}", run.all());
}

#[test]
fn the_thread_of_task_01_prints_from_the_recorded_report() {
    let (p, sha) = collected_thread();
    assert_eq!(traced(&p, &sha), THREAD);
    p.check_exact(&[]);
    assert_eq!(count(&raw_codes(&p), "code/W102"), 0);
}

#[test]
fn the_thread_of_task_01_prints_from_a_live_node_run() {
    if !node_test_runs() {
        eprintln!(
            "node --test with the JUnit reporter does not run here: the live exit cannot run"
        );
        return;
    }
    let (p, sha) = thread_twin();
    // The exit as written: detection picks node-test (package.json) and gsd-git (the branch's ref) on its own.
    succeeds(&p.collect(&[]));
    assert_eq!(traced(&p, &sha), THREAD);
    p.check_exact(&[]);
}

#[test]
fn the_thread_shows_a_failing_proof_and_stales_its_truth() {
    let (p, sha) = collected_thread();
    p.edit(
        ".specforge/reports/node-test.xml",
        "file=\"test/tasks.test.js\"/>",
        "file=\"test/tasks.test.js\"><failure message=\"x\"/></testcase>",
    );
    succeeds(&p.collect(&["--runner", "node-test", "--no-run"]));

    let line = traced(&p, &sha);
    assert!(
        line.ends_with("with the next id (failed) → TR1 verified, stale\n"),
        "{line}"
    );
    let report = p.report_all();
    let tr1 = ("gsd/E501".to_string(), "v1.0/01/01-01/TR1".to_string());
    assert!(judged(&report).contains(&tr1), "{report:#?}");
    assert!(
        report.iter().any(|(code, message)| code == "gsd/E501"
            && message.contains("'v1.0/01/01-01/TR1'")
            && message.ends_with("tests changed")),
        "{report:#?}"
    );
}

#[test]
fn the_trailer_links_a_commit_whose_subject_names_no_plan() {
    let (p, sha) = collected_thread();
    let commit = p.fields_of(&sha[..12]);
    // The one field `(Gsd-Task)` prints: a route that printed a constant would pass the golden, not this.
    assert_eq!(
        (&commit["task"], &commit["linked_by"]),
        (&"v1.0/01/01-01/T1".into(), &"Gsd-Task".into()),
        "{commit:#}"
    );
}

#[test]
fn the_twin_verdicts_are_fresh_over_the_thread() {
    let (p, _) = collected_thread();
    assert_eq!(p.verify_phase("v1.0/01"), 0);
}
