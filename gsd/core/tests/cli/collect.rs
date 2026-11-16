//! The collectors, end to end: `specforge collect --runner gsd-git` records commits in their plan's file.

use crate::harness::{Project, Run, project, project_at};
use specforge_extension_sdk::prelude::TestStatus;

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
fn a_commit_naming_a_plan_is_written_into_its_plan_file() {
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
    assert_eq!(fields["linked_by"], "title", "{fields}");
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
    // The baseline names no plan: nothing recorded to descend from, and that is written.
    assert_eq!(fields["descends_from"], serde_json::json!([]), "{fields}");
    // It is in the plan's own file, with a ledger line per field, and the project checks clean.
    let node = &p.nodes_of("commit")[0];
    assert_eq!(node["file"], "spec/plans.spec", "{node}");
    let ledger = p.read("specforge-harness");
    assert_eq!(
        ledger.lines().filter(|l| l.starts_with(name)).count(),
        12,
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
fn the_project_code_setting_is_stripped_from_a_scope() {
    // GSD users set `project_code` as a setting; the project entity has no such field here.
    let p = repository();
    p.settings(serde_json::json!({ "project_code": "PRJ" }));
    let hash = p.git_commit("feat(PRJ-01-01): Store module", &[("src/a.js", "x\n")], T0);
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(recorded(&p), [&hash[..12]]);
    assert_eq!(p.fields_of(&hash[..12])["plan"], "v1.0/01/01-01");
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
fn a_dry_run_writes_nothing_and_the_next_run_records_the_commit() {
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
fn a_commit_the_host_cannot_write_costs_only_itself() {
    let p = repository();
    let first = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let ansi = p.git_commit(
        "fix(01-01): colour \u{1b}[31mred\u{1b}[0m \u{1} \u{7f} \u{1f} \u{2028}",
        &[("src/\u{1b}b\u{2028}.js", "y\n")],
        T1,
    );
    // The commit writes its plan's file: the task it names is done, and its files are checked (ADR 0350).
    let last = p.git_commit(
        "feat(01-02): Complete a task",
        &[("src/complete.js", "z\n")],
        T2,
    );
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let mut all = [&first[..12], &ansi[..12], &last[..12]];
    all.sort_unstable();
    assert_eq!(recorded(&p), all);
    let fields = p.fields_of(&ansi[..12]);
    assert!(
        !fields["subject"]
            .as_str()
            .unwrap_or_default()
            .contains('\u{1b}'),
        "{fields}"
    );
    p.passing_tests();
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_subject_cannot_forge_a_commit() {
    let p = repository();
    let forged = p.git_commit(
        "feat(01-01): forged\u{1e}deadbeefdead\u{1f}0\u{1f}feat(01-02): Complete a task",
        &[("src/a.js", "x\n")],
        T0,
    );
    assert_eq!(p.collect(&["--runner", "gsd-git"]).code, 0);
    assert_eq!(recorded(&p), [&forged[..12]]);
    let fields = p.fields_of(&forged[..12]);
    assert_eq!(fields["plan"], "v1.0/01/01-01", "{fields}");
    assert!(fields.get("task").is_none(), "{fields}");
}

/// Named outside any repository, gsd-git fails and says so; in an unborn one it has nothing to record.
#[test]
fn a_named_gsd_git_outside_a_repository_fails_and_an_unborn_one_records_nothing() {
    let p = project("execute");
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains("the project is in no git repository"),
        "{}",
        run.all()
    );
    assert_eq!(recorded(&p), Vec::<String>::new());
    p.init_repository();
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(recorded(&p), Vec::<String>::new());
}

/// A plain `collect --yes` in a repository with a commit runs gsd-git, as a GSD project's own collect does.
#[test]
fn a_plain_collect_in_a_repository_records_the_commits() {
    let p = repository();
    let hash = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let run = p.run(&["collect", "--yes", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(recorded(&p), [&hash[..12]]);
}

/// gsd-git's runner result in a `collect --format json` answer.
fn gsd_git(run: &Run) -> serde_json::Value {
    let answer: serde_json::Value =
        serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()));
    answer["runners"]
        .as_array()
        .and_then(|runners| runners.iter().find(|r| r["name"] == "gsd-git"))
        .cloned()
        .unwrap_or_default()
}

/// A repository with no commit yet is detected by its `.git`, and gsd-git records nothing there.
#[test]
fn a_plain_collect_in_an_unborn_repository_records_nothing() {
    let p = project("execute");
    p.init_repository();
    let run = p.run(&["collect", "--yes", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(gsd_git(&run)["skip_reason"].is_null(), "{}", run.all());
    assert!(recorded(&p).is_empty());
}

/// In a linked worktree `.git` is a file: gsd-git is detected there and records the worktree's commits in its plan
/// files, leaving the main checkout's alone.
#[test]
fn a_plain_collect_in_a_linked_worktree_records_the_commits() {
    let p = project_at("execute", "main");
    p.commit();
    let hash = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    p.git(&["worktree", "add", "-q", "-b", "wt", "../wt"], T0);
    assert!(p.root().join("../wt/.git").is_file());
    let run = p.run(&["collect", "--path", "../wt", "--yes", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let written = p.git(&["-C", "../wt", "diff"], T0);
    assert!(
        written.contains(&hash[..12]),
        "the plan file holds the commit: {written}"
    );
    assert_eq!(
        p.git(&["status", "--porcelain"], T0),
        "",
        "the main checkout is untouched"
    );
}

/// Commits `src/a.js` for plan 01-01 in the repository above the project (never one of its own), at `T1`; its hash.
fn commit_in_parent(p: &Project) -> String {
    p.write("src/a.js", "x\n");
    p.git(&["add", "--", "src/a.js"], T1);
    p.git(&["commit", "-q", "-m", "feat(01-01): Store module"], T1);
    p.git(&["rev-parse", "HEAD"], T1).trim().to_string()
}

/// A project in a subdirectory of its repository has no `.git` of its own: gsd-git is detected by the repository above
/// it and records the commit, its paths project-relative.
#[test]
fn a_plain_collect_in_a_repository_subdirectory_records_the_commits() {
    let p = project_at("execute", "sub");
    p.git(&["-C", "..", "init", "-q", "-b", "main"], T0);
    p.git(&["add", "-A"], T0);
    p.git(&["commit", "-q", "-m", "baseline"], T0);
    let hash = commit_in_parent(&p);
    assert!(!p.root().join(".git").exists());
    let run = p.run(&["collect", "--yes", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(gsd_git(&run)["skip_reason"].is_null(), "{}", run.all());
    assert_eq!(recorded(&p), [&hash[..12]]);
    assert_eq!(
        p.fields_of(&hash[..12])["files"],
        serde_json::json!(["src/a.js"])
    );
}

/// A linked worktree whose project is in a subdirectory: the worktree's `.git` file is above the project, and gsd-git
/// is detected there and records the worktree's commits.
#[test]
fn a_plain_collect_in_a_linked_worktree_subdirectory_records_the_commits() {
    let p = project_at("execute", "main/sub");
    p.git(&["-C", "..", "init", "-q", "-b", "main"], T0);
    p.git(&["add", "-A"], T0);
    p.git(&["commit", "-q", "-m", "baseline"], T0);
    let hash = commit_in_parent(&p);
    p.git(
        &["-C", "..", "worktree", "add", "-q", "-b", "wt", "../wt"],
        T1,
    );
    assert!(p.root().join("../../wt/.git").is_file());
    let run = p.run(&[
        "collect",
        "--path",
        "../../wt/sub",
        "--yes",
        "--format",
        "json",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(gsd_git(&run)["skip_reason"].is_null(), "{}", run.all());
    let written = p.git(&["-C", "../../wt", "diff"], T1);
    assert!(
        written.contains(&hash[..12]),
        "the plan file holds the commit: {written}"
    );
}

/// gsd-git runs no program: a `--no-run` collect records the commits as a run does (ADR 0352).
#[test]
fn a_plain_collect_without_a_run_still_records_the_commits() {
    let p = repository();
    let hash = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let run = p.run(&["collect", "--no-run", "--yes", "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(gsd_git(&run)["skip_reason"].is_null(), "{}", run.all());
    assert_eq!(recorded(&p), [&hash[..12]]);
}

/// gsd-git reads the host's git port, not a `git` on `PATH`: none there, it records the commits all the same.
#[cfg(unix)]
#[test]
fn a_plain_collect_without_git_on_path_records_the_commits() {
    let p = repository();
    let hash = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let empty = tempfile::tempdir().expect("an empty directory");
    let path = empty.path().to_str().expect("a UTF-8 path");
    let run = p.run_with(&["collect", "--yes", "--format", "json"], &[("PATH", path)]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(recorded(&p), [&hash[..12]]);
}

/// The executor's `Gsd-Task:` trailer names the task of a commit whose subject names no plan.
#[test]
fn a_trailer_names_the_task_of_a_commit_without_a_scope() {
    let p = repository();
    let hash = p.git_commit(
        "chore: wire it\n\nGsd-Task: 01-01/T1\n",
        &[("src/w.js", "w\n")],
        T0,
    );
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(recorded(&p), [&hash[..12]]);
    let fields = p.fields_of(&hash[..12]);
    assert_eq!(fields["task"], "v1.0/01/01-01/T1", "{fields}");
    assert_eq!(fields["plan"], "v1.0/01/01-01", "{fields}");
    assert_eq!(fields["linked_by"], "Gsd-Task", "{fields}");
}

/// The trailer is read from the message's last paragraph, a second `-m` as much as one written in the message.
#[test]
fn a_trailer_in_a_second_message_paragraph_is_read() {
    let p = repository();
    p.write("src/w.js", "w\n");
    p.git(&["add", "--", "src/w.js"], T0);
    p.git(
        &[
            "commit",
            "-q",
            "-m",
            "chore: wire it",
            "-m",
            "Gsd-Task: 01-01/T1",
        ],
        T0,
    );
    let hash = p.git(&["rev-parse", "HEAD"], T0).trim().to_string();
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of(&hash[..12]);
    assert_eq!(fields["task"], "v1.0/01/01-01/T1", "{fields}");
    assert_eq!(fields["linked_by"], "Gsd-Task", "{fields}");
}

const TRUTH: &str = "v1.0/01/01-01/TR1";
const DELIVERABLE: &str = "v1.0/01/01-01/01-01-SUMMARY/D1";

/// The file of the `execute` project's deliverable check (`test/tasks.test.js#adds a task`).
const FILE: &str = "test/tasks.test.js";

/// The id `@javascript/node-test` gives the test `name` of [`FILE`] in the `taskboard` package.
fn test_id(name: &str) -> String {
    format!("js:taskboard/{FILE}#{name}")
}

/// The status of each of the deliverable's checks, `-` for none.
fn check_statuses(p: &Project) -> Vec<String> {
    p.fields_of(DELIVERABLE)["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .map(|c| c["status"].as_str().unwrap_or("-").to_string())
        .collect()
}

#[test]
fn a_tagged_node_test_links_to_its_truth() {
    let p = project("execute");
    let run = p.node_tests(&[(FILE, "[gsd:01-01/TR1] adds a task", TestStatus::Passed)]);
    assert_eq!(run.code, 0, "{}", run.all());
    // The tag is a short form: the fact holds it as written and the linker resolves it by its unique suffix.
    assert_eq!(
        p.edges_of("names"),
        [(test_id("[gsd:01-01/TR1] adds a task"), TRUTH.to_string())]
    );
    // The collect says only where it filed the facts (I209: the project is in no repository).
    let collected = json(&run);
    let said: Vec<&str> = collected["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .filter_map(|d| d["code"].as_str())
        .collect();
    assert_eq!(said, ["I209"], "{}", run.stdout);
    // No test fact is the deliverable's check (its name holds the tag): the check is unknown (W408).
    assert_eq!(p.codes(), ["gsd/W408"]);
}

#[test]
fn an_untagged_test_names_nothing() {
    let p = project("execute");
    let run = p.node_tests(&[(FILE, "adds a task", TestStatus::Passed)]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.edges_of("names"), []);
    assert_eq!(p.nodes_of("test").len(), 1);
}

#[test]
fn a_coverage_ref_links_to_its_deliverable() {
    let p = project("execute");
    let run = p.node_tests(&[(FILE, "adds a task", TestStatus::Passed)]);
    assert_eq!(run.code, 0, "{}", run.all());
    // The check names the fact by its file and name.
    assert_eq!(check_statuses(&p), ["pass"]);
    // The same test of another file proves nothing.
    let other = p.node_tests(&[("test/other.test.js", "adds a task", TestStatus::Passed)]);
    assert_eq!(other.code, 0, "{}", other.all());
    assert_eq!(check_statuses(&p), ["unknown"]);
}

#[test]
fn a_test_name_may_hold_a_hash() {
    // The reference is `path#test name`: the first `#` ends the path, whatever the name holds (an issue number).
    let p = project("execute");
    p.edit(
        "spec/plans.spec",
        "ref \"test/tasks.test.js#adds a task\"",
        "ref \"test/tasks.test.js#fixes #42\"",
    );
    let run = p.node_tests(&[(FILE, "fixes #42", TestStatus::Passed)]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(check_statuses(&p), ["pass"]);
}

#[test]
fn an_ambiguous_tag_is_a_dangling_name() {
    // `TR1` is the truth of both plans of phase 01: the name links neither, and `@code/core` says so at the test.
    let p = project("execute");
    let run = p.node_tests(&[(FILE, "[gsd:TR1] which one", TestStatus::Passed)]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.edges_of("names"), []);
    let dangling: Vec<(String, String)> = p
        .checked()
        .into_iter()
        .filter(|(code, _)| code == "code/I402")
        .collect();
    assert_eq!(dangling.len(), 1, "{:?}", p.checked());
    assert!(dangling[0].1.contains("TR1"), "{dangling:?}");
}

#[test]
fn a_failed_and_a_skipped_test_keep_their_status() {
    let p = project("execute");
    let run = p.node_tests(&[
        (FILE, "[gsd:01-01/TR1] fails", TestStatus::Failed),
        (FILE, "[gsd:01-01/AR1] skips", TestStatus::Skipped),
        (FILE, "[gsd:01-02/TR1] errors", TestStatus::Failed),
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let mut statuses: Vec<(String, String)> = p
        .nodes_of("test")
        .iter()
        .map(|n| {
            (
                n["id"].as_str().unwrap_or_default().to_string(),
                n["fields"]["status"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect();
    statuses.sort();
    assert_eq!(
        statuses,
        [
            (test_id("[gsd:01-01/AR1] skips"), "skipped".to_string()),
            (test_id("[gsd:01-01/TR1] fails"), "failed".to_string()),
            (test_id("[gsd:01-02/TR1] errors"), "failed".to_string()),
        ]
    );
    // A skipped test still names its entity: the thread reads it as no evidence.
    assert_eq!(
        p.edges_of("names"),
        [
            (
                test_id("[gsd:01-01/AR1] skips"),
                "v1.0/01/01-01/AR1".to_string()
            ),
            (test_id("[gsd:01-01/TR1] fails"), TRUTH.to_string()),
            (
                test_id("[gsd:01-02/TR1] errors"),
                "v1.0/01/01-02/TR1".to_string()
            ),
        ]
    );
}

/// gsd-git is a gsd project's one runner (`checks` publishes outcomes, it runs nothing), and a project that enabled
/// a single runner chose it: a plain `collect` runs it as `--runner gsd-git` would, so outside a repository it fails
/// and says why, and a JUnit report lying there selects no other.
#[test]
fn a_plain_collect_runs_gsd_git_the_projects_one_runner() {
    let p = project("execute");
    p.write("junit.xml", "<testsuite/>");
    let run = p.run(&["collect", "--yes", "--format", "json"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains("the project is in no git repository"),
        "{}",
        run.all()
    );
    assert!(!p.exists("specforge-report.json"));
}

/// A failing test never refuses the commits `gsd-git` records. Under `gsd-strict` a failing check is an Error
/// (gsd/W408) the test facts alone cause. `collect` runs one runner per call and `node-test` answers no plan, so the
/// tests and the commits are two calls here; the same-call case (the plan judged with what its collect wrote) is
/// proved in the kernel (`collect/report_first.rs` of `specforge-ops`).
#[test]
fn a_failing_test_never_refuses_the_commits_of_a_collect() {
    let p = repository();
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    let truth = "v1.0/01/01-01/TR1";
    assert_eq!(p.record(truth, "E501", "verified", &[]).code, 0);
    let run = p.node_tests(&[
        (FILE, "adds a task", TestStatus::Failed),
        (FILE, "[gsd:01-01/TR1] fails", TestStatus::Failed),
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let hash = p.git_commit("feat(01-01): Store module", &[("src/a.js", "x\n")], T0);
    let run = p.collect(&["--runner", "gsd-git"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(recorded(&p), [&hash[..12]]);
    // The failing test stales the verdict and the strict deliverable check is an Error: reported, not a refusal.
    assert_eq!(p.inputs(truth, "E501")["verdict"]["state"], "stale");
    assert!(p.codes().iter().all(|c| c != "gsd/E502"), "{:?}", p.codes());
}
