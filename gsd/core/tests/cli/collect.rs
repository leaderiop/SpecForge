//! The collectors, end to end: `specforge collect --runner gsd-git` records commits in their plan's file.

use crate::harness::{Project, Run, project};

const T0: &str = "2026-10-29T09:00:00Z";
const T1: &str = "2026-10-29T09:30:00Z";
const T2: &str = "2026-10-29T10:00:00Z";

fn json(run: &Run) -> serde_json::Value {
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("collect prints JSON ({e}): {}", run.all()))
}

/// The commit nodes' ids, sorted.
fn recorded(p: &Project) -> Vec<String> {
    let mut names: Vec<String> = p
        .nodes_of("commit")
        .iter()
        .filter_map(|n| n["id"].as_str().map(String::from))
        .collect();
    names.sort();
    names
}

/// The execute project, committed once (the baseline), ready for `gsd-git`.
fn repository() -> Project {
    let p = project("execute");
    p.commit();
    p
}

#[test]
fn a_commit_naming_03_02_is_written_into_its_plan_file() {
    // The first use of a wasm guest's writing collector that creates harness-written entities.
    let p = repository();
    let hash = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let name = &hash[..12];
    assert_eq!(recorded(&p), [name]);
    let fields = p.fields_of(name);
    assert_eq!(fields["hash"], hash.as_str(), "{fields}");
    assert_eq!(fields["task"], "v1.0/01/01-01/T1", "{fields}");
    assert_eq!(fields["plan"], "v1.0/01/01-01", "{fields}");
    assert_eq!(fields["type"], "feat", "{fields}");
    assert_eq!(fields["subject"], "feat(01-01): Store module", "{fields}");
    assert_eq!(fields["files"], serde_json::json!(["src/a.js"]), "{fields}");
    assert_eq!(
        fields["created"],
        serde_json::json!(["src/a.js"]),
        "{fields}"
    );
    assert_eq!(fields["insertions"], 1, "{fields}");
    assert_eq!(fields["deletions"], 0, "{fields}");
    assert_eq!(fields["at"], T0, "{fields}");
    // It is in the plan's own file, with a ledger line per field, and the project checks clean.
    let node = &p.nodes_of("commit")[0];
    assert_eq!(node["file"], "spec/plans.spec", "{node}");
    let ledger = p.read("specforge-harness");
    assert_eq!(
        ledger.lines().filter(|l| l.starts_with(name)).count(),
        10,
        "{ledger}"
    );
    p.passing_tests();
    assert_eq!(p.codes(), Vec::<String>::new());
    let report = json(&run);
    assert_eq!(
        report["runners"][0]["plan"]["entities"],
        serde_json::json!([name])
    );
}

#[test]
fn an_unrelated_commit_is_skipped() {
    let p = repository();
    p.git_commit("feat(gsd): x", &[("a.txt", "1")], T0);
    p.git_commit("docs(pause): y", &[("b.txt", "2")], T1);
    p.git_commit("feat(99-99): a plan nobody has", &[("c.txt", "3")], T2);
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(recorded(&p), Vec::<String>::new());
    assert!(
        json(&run)["runners"][0].get("plan").is_none(),
        "{}",
        run.stdout
    );
    p.passing_tests();
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_metadata_commit_names_the_plan_only() {
    let p = repository();
    let hash = p.git_commit(
        "docs(01-01): complete Task store plan",
        &[("notes.md", "done\n")],
        T0,
    );
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    let fields = p.fields_of(&hash[..12]);
    assert_eq!(fields["plan"], "v1.0/01/01-01", "{fields}");
    assert_eq!(fields["type"], "docs", "{fields}");
    assert!(fields.get("task").is_none(), "{fields}");
    p.passing_tests();
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_second_collect_records_nothing_new() {
    let p = repository();
    p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    let before = p.read("spec/plans.spec");
    let ledger = p.read("specforge-harness");
    let again = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(again.code, 0, "{}", again.all());
    assert!(
        json(&again)["runners"][0].get("plan").is_none(),
        "{}",
        again.stdout
    );
    assert_eq!(p.read("spec/plans.spec"), before);
    assert_eq!(p.read("specforge-harness"), ledger);
    assert_eq!(recorded(&p).len(), 1);
}

#[test]
fn a_commit_after_the_plan_file_changed_is_recorded_on_the_next_run() {
    let p = repository();
    let hash = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let before = p.read("spec/plans.spec");
    let dry = p.collect(&["--runner", "gsd-git", "--dry-run"]);
    assert_eq!(dry.code, 0, "{}", dry.all());
    assert_eq!(
        json(&dry)["runners"][0]["plan"]["dry_run"],
        true,
        "{}",
        dry.stdout
    );
    assert_eq!(
        p.read("spec/plans.spec"),
        before,
        "a dry run writes nothing"
    );
    assert_eq!(recorded(&p), Vec::<String>::new());
    p.edit(
        "spec/plans.spec",
        "plan 01-02 \"Completion\"",
        "plan 01-02 \"Completion step\"",
    );
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    assert_eq!(recorded(&p), [&hash[..12]]);
    assert_eq!(p.read("spec/plans.spec").matches("commit ").count(), 1);
}

#[test]
fn an_ambiguous_plan_scope_is_skipped() {
    let p = repository();
    p.write(
        "spec/v2.0.spec",
        "milestone v2.0 \"Taskboard 2.0\" {\n  name \"Taskboard 2.0\"\n}\n\nrequirement SHARE-02 \"Share a board\" {\n  text \"A board is shared by link\"\n  category \"Sharing\"\n  tier v2\n}\n\nphase 01 \"Sharing\" {\n  milestone v2.0\n  name \"Sharing\"\n  slug \"sharing\"\n  goal \"Boards are shared\"\n  requirements [SHARE-02]\n  success_criteria [\"a\", \"b\"]\n}\n\nplan 01-01 \"Share link\" {\n  phase v2.0/01\n  type execute\n  files_modified [\"src/share.js\"]\n  requirements [SHARE-02]\n  objective \"A board has a share link\"\n}\n\ntruth TR1 \"A link\" {\n  plan v2.0/01/01-01\n  text \"A board has a link\"\n}\n\nartifact AR1 \"Link\" {\n  plan v2.0/01/01-01\n  path \"src/share.js\"\n  provides \"share()\"\n}\n",
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    p.commit();
    p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let hash = p.git_commit(
        "feat(v2.0/01/01-01): Share link",
        &[("src/share.js", "y\n")],
        T1,
    );
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    // `01-01` names two plans, so it names none; the full path names one.
    assert_eq!(recorded(&p), [&hash[..12]]);
    assert_eq!(p.fields_of(&hash[..12])["plan"], "v2.0/01/01-01");
}

#[test]
fn a_repository_without_commits_is_e045() {
    // `git log` fails in a repository with no commit, and in a directory that is none: no output, so no report.
    for init in [false, true] {
        let p = project("execute");
        if init {
            p.init_repository();
        }
        let run = p.collect(&["--runner", "gsd-git"]);
        assert_ne!(run.code, 0, "init {init}: {}", run.all());
        assert!(
            run.all().contains("E045") && run.all().contains("gsd-git produced no report"),
            "init {init}: {}",
            run.all()
        );
        assert_eq!(recorded(&p), Vec::<String>::new());
    }
}

const TRUTH: &str = "v1.0/01/01-01/TR1";
const DELIVERABLE: &str = "v1.0/01/01-01/01-01-SUMMARY/D1";

/// The report the collectors wrote, as `{entity: [(test name, status)]}`.
fn report(p: &Project) -> std::collections::BTreeMap<String, Vec<(String, String)>> {
    let report: serde_json::Value =
        serde_json::from_str(&p.read("specforge-report.json")).expect("a JSON report");
    report["results"]
        .as_object()
        .expect("results")
        .iter()
        .map(|(entity, recorded)| {
            let tests = recorded["tests"]
                .as_array()
                .expect("tests")
                .iter()
                .map(|t| {
                    (
                        t["name"].as_str().unwrap_or_default().to_string(),
                        t["status"].as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();
            (entity.clone(), tests)
        })
        .collect()
}

/// The root of the project as the tool that runs the tests prints it: its real path.
fn root_of(p: &Project) -> String {
    p.root()
        .canonicalize()
        .expect("the root exists")
        .display()
        .to_string()
}

fn junit(p: &Project, testcases: &str) -> Run {
    p.write(
        "junit.xml",
        &format!("<?xml version=\"1.0\"?>\n<testsuites>\n<testsuite name=\"s\">\n{testcases}\n</testsuite>\n</testsuites>\n"),
    );
    p.collect(&["--runner", "gsd-junit", "--no-run"])
}

#[test]
fn a_tagged_node_test_links_to_its_truth() {
    let p = project("execute");
    let run = junit(
        &p,
        "<testcase name=\"[gsd:01-01/TR1] adds a task\" classname=\"test\" time=\"0.01\"/>",
    );
    assert_eq!(run.code, 0, "{}", run.all());
    // The tag is a short form, which the host's own `collect` would drop as W115: the collector resolved it.
    assert_eq!(
        report(&p),
        [(
            TRUTH.to_string(),
            vec![(
                "[gsd:01-01/TR1] adds a task".to_string(),
                "passed".to_string()
            )]
        )]
        .into()
    );
    let runner = &json(&run)["runners"][0];
    assert_eq!(
        (runner["entities"].as_i64(), runner["passed"].as_i64()),
        (Some(1), Some(1))
    );
    assert_eq!(
        json(&run)["diagnostics"],
        serde_json::json!([]),
        "{}",
        run.stdout
    );
    // The report names no test for the deliverable's check: it is unknown (W408).
    assert_eq!(p.codes(), ["gsd/W408"]);
}

#[test]
fn an_untagged_test_is_reported_unlinked() {
    let p = project("execute");
    let run = junit(&p, "<testcase name=\"adds a task\" classname=\"test\"/>");
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(report(&p), Default::default());
    assert_eq!(json(&run)["runners"][0]["entities"], 0, "{}", run.stdout);
}

#[test]
fn a_coverage_ref_links_to_its_deliverable() {
    let p = project("execute");
    let file = format!("{}/test/tasks.test.js", root_of(&p));
    let run = junit(
        &p,
        &format!(
            "<testcase name=\"adds a task\" classname=\"test\" time=\"0.02\" file=\"{file}\"/>"
        ),
    );
    assert_eq!(run.code, 0, "{}", run.all());
    // Recorded under the deliverable, named by the reference exactly.
    assert_eq!(
        report(&p),
        [(
            DELIVERABLE.to_string(),
            vec![(
                "test/tasks.test.js#adds a task".to_string(),
                "passed".to_string()
            )]
        )]
        .into()
    );
    // The same test of another file proves nothing.
    let other = junit(
        &p,
        "<testcase name=\"adds a task\" classname=\"test\" file=\"/elsewhere/test/other.test.js\"/>",
    );
    assert_eq!(other.code, 0, "{}", other.all());
    assert_eq!(report(&p), Default::default());
}

#[test]
fn an_ambiguous_tag_is_unlinked() {
    // `TR1` is the truth of both plans of phase 01.
    let p = project("execute");
    let run = junit(&p, "<testcase name=\"[gsd:TR1] which one\"/>");
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(report(&p), Default::default());
    assert_eq!(
        json(&run)["diagnostics"],
        serde_json::json!([]),
        "{}",
        run.stdout
    );
}

#[test]
fn a_failure_child_is_failed_and_skipped_is_not_recorded() {
    let p = project("execute");
    let run = junit(
        &p,
        "<testcase name=\"[gsd:01-01/TR1] fails\"><failure message=\"no\"/></testcase>\n<testcase name=\"[gsd:01-01/AR1] skips\"><skipped/></testcase>\n<testcase name=\"[gsd:01-02/TR1] errors\"><error/></testcase>",
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        report(&p),
        [
            (
                TRUTH.to_string(),
                vec![("[gsd:01-01/TR1] fails".to_string(), "failed".to_string())]
            ),
            (
                "v1.0/01/01-02/TR1".to_string(),
                vec![("[gsd:01-02/TR1] errors".to_string(), "failed".to_string())]
            ),
        ]
        .into()
    );
    let runner = &json(&run)["runners"][0];
    assert_eq!(
        (
            runner["failed"].as_i64(),
            runner["skipped"].as_i64(),
            runner["passed"].as_i64()
        ),
        (Some(2), Some(1), Some(0)),
        "{}",
        run.stdout
    );
}

#[test]
fn a_report_that_stops_short_is_refused() {
    let p = project("execute");
    p.write(
        "junit.xml",
        "<testsuite><testcase name=\"[gsd:01-01/TR1] a\"",
    );
    let run = p.collect(&["--runner", "gsd-junit", "--no-run"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("junit.xml"), "{}", run.all());
    assert!(!p.exists("specforge-report.json"));
}

#[test]
fn the_junit_collector_is_never_run_by_a_plain_collect() {
    // It is named, not detected: a plain `collect` finds no runner to run.
    let p = project("execute");
    p.write("junit.xml", "<testsuite/>");
    let run = p.run(&["collect", "--yes", "--format", "json"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains("E058") && run.all().contains("available: gsd-git, gsd-junit"),
        "{}",
        run.all()
    );
}
