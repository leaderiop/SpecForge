//! R1's exit, end to end on the taskboard twin: a commit whose `Gsd-Task:` trailer names `01-01/T1`, a collect of the
//! twin's tests, and `specforge trace TASK-01` prints the requirement's thread from its `proof` through plan, task,
//! commit and the items it changed to the plan's truth and the requirement's coverage item, a tree, read from GSD's own
//! fields and collected data alone (ADR 0371 D7): the twin's truths name no requirement and no proof. The
//! thread is SpecForge's own (gsd-tools has no such command), so its golden is
//! `tests/golden/trace/taskboard-TASK-01.txt`, not one of `goldens/taskboard/`, and `parity_scenario` does not list
//! it. The commit's sha is the one varying part: it is replaced by `<sha>` before the comparison.

use super::harness::{Project, Run, judged, taskboard};
use super::node::node_test_runs;
use specforge_extension_sdk::prelude::TestStatus;

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
pub(super) fn collected_thread() -> (Project, String) {
    let (p, sha) = thread_twin();
    succeeds(&p.collect_node_test_golden());
    succeeds(&p.collect(&["--runner", "javascript-source"]));
    succeeds(&p.collect(&["--runner", "gsd-git"]));
    (p, sha)
}

pub(super) fn succeeds(run: &Run) {
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
    // T1 (done) names two items no collect has stored: W102 once per reference, and no error. The truths cite no
    // test (no `proof`), so no test reference is left dangling.
    assert_eq!(count(&codes, "code/W102"), 2, "{codes:?}");
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
    // The exit as written: detection picks node-test (package.json) and gsd-git (its repository) on its own.
    succeeds(&p.collect(&[]));
    assert_eq!(traced(&p, &sha), THREAD);
    p.check_exact(&[]);
}

/// The failing test is the one the coverage item's check names: the requirement's `proof` carries the failure on the
/// thread's first line, the coverage item's line is the same, and `gsd/W408` reports the check. TR1's verdict stays
/// fresh: none of its inputs reads the test now (its `tests` select nothing, its files did not move).
#[test]
fn the_thread_shows_a_failing_check_as_a_failed_proof() {
    let (p, sha) = collected_thread();
    let passing: Vec<String> = traced(&p, &sha).lines().map(str::to_string).collect();
    fail_the_first_test(&p);

    let line = traced(&p, &sha);
    let lines: Vec<&str> = line.lines().collect();
    assert_eq!(lines.first(), Some(&"TASK-01 (failed)"), "{line}");
    let deliverable = |lines: &[&str]| -> Option<String> {
        lines
            .iter()
            .find(|l| l.trim_start().starts_with("→ deliverable "))
            .map(|l| l.to_string())
    };
    let before: Vec<&str> = passing.iter().map(String::as_str).collect();
    assert_eq!(deliverable(&lines), deliverable(&before), "{line}");
    assert_eq!(
        &lines[1..],
        &before[1..],
        "only the first line moves: {line}"
    );
    assert!(line.contains("→ TR1 verified, fresh\n"), "{line}");
    let report = p.report_all();
    let tr1 = ("gsd/E501".to_string(), "v1.0/01/01-01/TR1".to_string());
    assert!(!judged(&report).contains(&tr1), "{report:#?}");
    let d1 = "v1.0/01/01-01/01-01-SUMMARY/D1";
    assert!(
        report
            .iter()
            .any(|(code, message)| code == "gsd/W408" && message.contains(d1)),
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

/// The commit is the twin's root: it adds every line of `src/tasks.js`, so it changed each item the file holds.
#[test]
fn the_threads_commit_names_the_items_it_changed() {
    let (p, sha) = collected_thread();
    let commit = p.fields_of(&sha[..12]);
    assert_eq!(
        commit["items_changed"],
        serde_json::json!([
            "js:taskboard/src/tasks.js::addTask",
            "js:taskboard/src/tasks.js::completeTask",
            "js:taskboard/src/tasks.js::createBoard"
        ]),
        "{commit:#}"
    );
}

#[test]
fn the_twin_verdicts_are_fresh_over_the_thread() {
    let (p, _) = collected_thread();
    assert_eq!(p.verify_phase("v1.0/01"), 0);
}

/// With the requirement taken off plan 01-01, each item its task names (`addTask`, `createBoard`) and the
/// one only its commit changed (`completeTask`) is anchored by a plan that reaches no requirement.
#[test]
fn the_twins_items_of_a_plan_with_no_requirement_are_w522() {
    let (p, _) = collected_thread();
    p.edit(
        "spec/plans.spec",
        "requirements     [TASK-01]",
        "requirements     []",
    );
    let mut found: Vec<String> = p
        .report_all()
        .into_iter()
        .filter(|(code, _)| code == "gsd/W522")
        .map(|(_, message)| message)
        .collect();
    found.sort();
    let item = |name: &str| {
        format!(
            "item `js:taskboard/src/tasks.js::{name}` is anchored only by plan 'v1.0/01/01-01', which reaches no \
             requirement"
        )
    };
    assert_eq!(
        found,
        [item("addTask"), item("completeTask"), item("createBoard")],
        "{:#?}",
        p.report_all()
    );
}

/// The ids `specforge trace --route gsd/requirement-thread --in <scope>` traces, in its order.
fn traced_within(p: &Project, scope: &str) -> Vec<String> {
    let run = p.run(&[
        "trace",
        "--route",
        "gsd/requirement-thread",
        "--in",
        scope,
        "--partial",
        "--format",
        "json",
    ]);
    succeeds(&run);
    let document: serde_json::Value =
        serde_json::from_str(&run.stdout).expect("trace --format json writes JSON");
    document["traces"]
        .as_array()
        .expect("a list of traces")
        .iter()
        .map(|trace| {
            trace["entity_id"]
                .as_str()
                .expect("an entity id")
                .to_string()
        })
        .collect()
}

#[test]
fn trace_within_a_milestone_follows_its_phases_requirements() {
    let p = taskboard();
    // v1.0 holds no requirement itself: its phases name them, and v2's SYNC-01 and SHARE-01 stay out.
    assert_eq!(
        traced_within(&p, "v1.0"),
        [
            "LIST-01", "STORE-01", "STORE-02", "TASK-01", "TASK-02", "TASK-03"
        ]
    );
    assert_eq!(traced_within(&p, "01"), ["TASK-01", "TASK-02"]);
}

/// The twin's tests as recorded, the first failing: the one that proves TASK-01's truth.
fn fail_the_first_test(p: &Project) {
    succeeds(&p.node_tests(&[
        (
            "test/tasks.test.js",
            "a task is added with the next id",
            TestStatus::Failed,
        ),
        (
            "test/tasks.test.js",
            "a task is completed by its id",
            TestStatus::Passed,
        ),
        (
            "test/tasks.test.js",
            "completing an unknown id throws",
            TestStatus::Passed,
        ),
    ]));
}

#[test]
fn the_requirements_proof_follows_its_thread() {
    let proof = |p: &Project, id: &str| p.fields_of(id)["proof"].clone();
    let p = taskboard();
    assert_eq!(proof(&p, "TASK-01"), "no_commit");
    // No plan lists v2's SYNC-01: it reads its tests alone, and no test fact is collected yet.
    assert_eq!(proof(&p, "SYNC-01"), "not_run");
    let (p, _) = thread_twin();
    succeeds(&p.collect(&["--runner", "gsd-git"]));
    assert_eq!(proof(&p, "TASK-01"), "not_run");
    let (p, _) = collected_thread();
    assert_eq!(proof(&p, "TASK-01"), "proven");
    assert_eq!(proof(&p, "TASK-02"), "no_commit");
    // Test facts are collected, and none names SYNC-01 or is named by a check of its coverage.
    assert_eq!(proof(&p, "SYNC-01"), "no_test");
    fail_the_first_test(&p);
    assert_eq!(proof(&p, "TASK-01"), "failed");
}

/// `satisfied_by` is what the recorded commits of the requirement's tasks changed (ADR 0371 D6): nothing before a
/// commit is collected (T1's own `items` are not read), then the three items the twin's root commit changed, which are
/// the items the thread prints.
#[test]
fn satisfied_by_is_the_items_its_thread_reaches() {
    let items = serde_json::json!([
        "js:taskboard/src/tasks.js::addTask",
        "js:taskboard/src/tasks.js::completeTask",
        "js:taskboard/src/tasks.js::createBoard"
    ]);
    let bare = taskboard().fields_of("TASK-01");
    assert!(bare.get("satisfied_by").is_none(), "{bare:#}");
    let (p, sha) = collected_thread();
    assert_eq!(p.fields_of("TASK-01")["satisfied_by"], items);
    let traced: Vec<String> = traced(&p, &sha)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("→ "))
        .filter(|target| target.starts_with("js:taskboard/src/"))
        .map(str::to_string)
        .collect();
    assert_eq!(serde_json::json!(traced), items);
}
