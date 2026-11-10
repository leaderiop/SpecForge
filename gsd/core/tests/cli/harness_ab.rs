//! The GSD A/B's copies (`evaluation/gsd-ab/`): an arm's copy of a task's fixture, made by the protocol's own
//! `seed.py` (so the frozen configuration in `tasks.json` is what runs), and what arm B's checker says on it.
//!
//! Python 3 is a test-only requirement: a machine without `python3` fails these tests naming it.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use super::{Project, Run, required};

/// The repository's `evaluation/gsd-ab/`.
pub fn ab_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evaluation/gsd-ab")
}

/// `tasks.json`, parsed.
pub fn ab_tasks() -> serde_json::Value {
    let path = ab_dir().join("tasks.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("tasks.json is JSON: {e}"))
}

/// `python3 -I <script> <args>` in `evaluation/gsd-ab/`.
pub fn python(script: &str, args: &[&str]) -> Output {
    Command::new("python3")
        .arg("-I")
        .arg(ab_dir().join(script))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "python3 does not run ({e}): the gsd A/B tests run evaluation/gsd-ab's scripts with Python 3, a \
                 test-only requirement; install python3"
            )
        })
}

/// The recorded test report of a fixture, as `(collectors, runner, golden)`: what its tests report when nothing is
/// seeded, read by `runner` after the `collectors` whose facts it reads have run.
type Recorded = (&'static [&'static str], &'static str, PathBuf);

fn recorded_report(fixture: &str) -> Option<Recorded> {
    let goldens = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/goldens");
    match fixture {
        "taskboard" => Some((&[], "node-test", goldens.join("taskboard/junit.xml"))),
        "vault" => Some((&[], "node-test", goldens.join("vault/junit.xml"))),
        // The ledger's CI is red: the recorded run is the one with `total_beyond_i32_is_exact` failing.
        "ledger" => Some((
            &["rust-cargo", "rust-source"],
            "rust-test",
            goldens.join("ledger/rust-test.xml"),
        )),
        _ => None,
    }
}

/// `report` with the seeded runs' differences: a test named `failed` fails, one named `absent` is gone. Each name
/// must be a test case of the report: a seed that names a test the fixture does not have fails here.
fn seeded_report(report: &str, runs: &[(String, String)]) -> String {
    let mut text = report.to_string();
    for (name, outcome) in runs {
        let open = format!("<testcase name=\"{name}\"");
        let start = text
            .find(&open)
            .unwrap_or_else(|| panic!("the recorded report has no test `{name}`"));
        let end = start + text[start..].find("/>").expect("a self-closed test case") + 2;
        let case = &text[start..end];
        let replaced = match outcome.as_str() {
            "failed" => format!(
                "{}><failure message=\"seeded\"/></testcase>",
                &case[..case.len() - 2]
            ),
            "absent" => String::new(),
            other => panic!("a seed's run is `failed` or `absent`, not `{other}`"),
        };
        text.replace_range(start..end, &replaced);
    }
    text
}

/// One `b_signals` row of a task: what arm B's checker says once the task is seeded.
#[derive(Debug, Clone)]
pub struct Signal {
    pub defect: String,
    /// The diagnostic code, or `None` for a trace row.
    pub code: Option<String>,
    /// The entity the diagnostic is on; `None` counts it anywhere (a seeded commit, whose id is the seeding's).
    pub entity: Option<String>,
    pub count: usize,
    /// `seed`, or `complete` (after the task's completion edits).
    pub when: String,
    /// Whether the defect's seed changes the test run (`run` in its seed): its signal must read the run, so its file
    /// edits over the recorded run unchanged report none.
    pub reruns: bool,
}

/// The `b_signals` of `task`.
pub fn signals(task: &str) -> Vec<Signal> {
    let tasks = ab_tasks();
    let task = ab_task(&tasks, task);
    let text = |row: &serde_json::Value, key: &str| row[key].as_str().map(str::to_string);
    task["b_signals"]
        .as_array()
        .expect("b_signals")
        .iter()
        .map(|row| {
            let defect = text(row, "defect").expect("a defect");
            Signal {
                reruns: task["seeds"][&defect]["run"].is_object(),
                defect,
                code: text(row, "code"),
                entity: text(row, "entity"),
                count: row["count"].as_u64().expect("a count") as usize,
                when: text(row, "when").expect("when"),
            }
        })
        .collect()
}

/// The task `id` of `tasks.json`.
pub fn ab_task<'a>(tasks: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    tasks["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .find(|t| t["id"] == id)
        .unwrap_or_else(|| panic!("no task {id} in tasks.json"))
}

/// Arm `arm`'s copy of `task`'s fixture, made by `seed.py seed --worktree` (the working tree, so a change under test
/// is seen before it is committed) with `flags` (`--state complete`, `--unseeded`) and the components from the
/// environment; `Err` holds what `seed.py` said when it refused.
pub fn seeded(task: &str, arm: &str, flags: &[&str]) -> Result<Project, String> {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("copy");
    let core = required("GSD_CORE_WASM").display().to_string();
    let mut args = vec![
        "seed",
        "--worktree",
        "--task",
        task,
        "--arm",
        arm,
        "--gsd-core",
        &core,
    ];
    let security = std::env::var("GSD_SECURITY_WASM").unwrap_or_default();
    if !security.is_empty() {
        args.extend(["--gsd-security", &security]);
    }
    let out = root.display().to_string();
    args.extend(["--out", &out]);
    args.extend_from_slice(flags);
    let made = python("seed.py", &args);
    if !made.status.success() {
        return Err(String::from_utf8_lossy(&made.stderr).into_owned());
    }
    let manifest: serde_json::Value = std::fs::read_to_string(root.join("specforge.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let extensions = manifest["extensions"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|e| e.as_str())
                .map(|e| match e.split_once('=') {
                    Some((name, wasm)) => (name.to_string(), Some(PathBuf::from(wasm))),
                    None => (e.to_string(), None),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Project {
        _dir: dir,
        root,
        extensions: RefCell::new(extensions),
        quiet: Vec::new(),
    })
}

impl Project {
    /// Whether `specforge.json` loads `name`.
    pub fn loads(&self, name: &str) -> bool {
        self.extensions.borrow().iter().any(|(n, _)| n == name)
    }

    /// A copy's collects, as a run of the arm would make them where no test runner runs: the fixture's recorded
    /// report (when the copy loads `@code/core`), with `task`'s seeded runs on a `seeded` copy, then its commits.
    pub fn collect_seeded(&self, task: &str, seeded: bool) {
        let tasks = ab_tasks();
        let task = ab_task(&tasks, task);
        let fixture = task["fixture"].as_str().expect("a fixture");
        if self.loads("@code/core")
            && let Some((collectors, runner, golden)) = recorded_report(fixture)
        {
            for collector in collectors {
                succeeds(&self.collect(&["--runner", collector]));
            }
            let runs: Vec<(String, String)> = task["seeded"]
                .as_array()
                .expect("seeded")
                .iter()
                .filter(|_| seeded)
                .filter_map(|d| task["seeds"][d.as_str()?]["run"].as_object())
                .flat_map(|runs| runs.iter())
                .map(|(name, outcome)| (name.clone(), outcome.as_str().unwrap_or("").to_string()))
                .collect();
            let recorded = std::fs::read_to_string(&golden)
                .unwrap_or_else(|e| panic!("{}: {e}", golden.display()));
            let report = self.root.parent().expect("the temp dir").join("report.xml");
            std::fs::write(&report, seeded_report(&recorded, &runs)).expect("the report");
            succeeds(&self.collect(&[
                "--runner",
                runner,
                "--no-run",
                "--report",
                &report.display().to_string(),
            ]));
        }
        succeeds(&self.collect(&["--runner", "gsd-git"]));
    }

    /// What `signal` counts on the copy: diagnostics of its code on its entity (its `data.entity`, or quoted in its
    /// message).
    pub fn signal(&self, signal: &Signal) -> usize {
        let code = signal.code.as_deref().expect("a code");
        let run = self.run(&["check", "--format", "json"]);
        let found: serde_json::Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
        found
            .as_array()
            .expect("a list")
            .iter()
            .filter(|d| d["code"] == code)
            .filter(|d| match &signal.entity {
                None => true,
                Some(entity) => {
                    d["data"]["entity"] == entity.as_str()
                        || d["message"]
                            .as_str()
                            .is_some_and(|m| m.contains(&format!("'{entity}'")))
                }
            })
            .count()
    }
}

fn succeeds(run: &Run) {
    assert_eq!(run.code, 0, "{}", run.all());
}

/// The defects the requirement thread exists to report, whose signal B0 (no test facts) does not have: those
/// `tasks.json#defects` marks `"thread": true`.
fn thread_defects() -> Vec<String> {
    ab_tasks()["defects"]
        .as_array()
        .expect("defects")
        .iter()
        .filter(|d| d["thread"] == true)
        .map(|d| d["id"].as_str().expect("an id").to_string())
        .collect()
}

/// Checks every `b_signals` row of `task` on arms B and B0: seeded and unseeded copies are collected and counted.
/// On B a row counts `count` on the seeded copy and 0 on the unseeded one; a `complete` row counts 0 before the
/// completion edits too; and a row whose seed changes the test run counts 0 on a seeded copy collected over the
/// recorded run unchanged (the file edit alone). On B0 a thread row counts 0 on both copies (B0 has no test facts to
/// report it from), and the others as on B.
pub fn assert_signals(task: &str) {
    let rows = signals(task);
    let thread = thread_defects();
    let mut failures = Vec::new();
    for arm in ["B", "B0"] {
        for state in ["seed", "complete"] {
            let here: Vec<&Signal> = rows
                .iter()
                .filter(|r| state == "seed" || r.when == "complete")
                .collect();
            if here.is_empty() {
                continue;
            }
            let flags: &[&str] = if state == "complete" {
                &["--state", "complete"]
            } else {
                &[]
            };
            let copies = seeded(task, arm, flags).and_then(|s| {
                let mut control = vec!["--unseeded"];
                control.extend_from_slice(flags);
                seeded(task, arm, &control).map(|u| (s, u))
            });
            let (seeded_copy, unseeded_copy) = match copies {
                Ok(copies) => copies,
                Err(why) => {
                    for row in &here {
                        failures.push(format!(
                            "{arm} {state} {}: seeding refused: {why}",
                            row.defect
                        ));
                    }
                    continue;
                }
            };
            seeded_copy.collect_seeded(task, true);
            unseeded_copy.collect_seeded(task, false);
            let reruns: Vec<&Signal> = here.iter().copied().filter(|r| r.reruns).collect();
            if arm == "B" && state == "seed" && !reruns.is_empty() {
                match seeded(task, arm, &[]) {
                    Ok(edited) => {
                        edited.collect_seeded(task, false);
                        for row in reruns {
                            let got = edited.signal(row);
                            if got != 0 {
                                failures.push(format!(
                                    "{arm} {} {:?}: {got} on its file edit alone, over the recorded run",
                                    row.defect,
                                    row.code.as_deref().unwrap_or_default()
                                ));
                            }
                        }
                    }
                    Err(why) => {
                        failures.push(format!("{arm} edit-only copy: seeding refused: {why}"))
                    }
                }
            }
            for row in &here {
                let got = (seeded_copy.signal(row), unseeded_copy.signal(row));
                let wanted = if (arm == "B0" && thread.contains(&row.defect))
                    || (row.when == "complete" && state == "seed")
                {
                    (0, 0)
                } else {
                    (row.count, 0)
                };
                if got != wanted {
                    failures.push(format!(
                        "{arm} {state} {} {:?}: (seeded, unseeded) = {got:?}, want {wanted:?}",
                        row.defect,
                        row.code.as_deref().unwrap_or_default()
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{task}:\n{}", failures.join("\n"));
}

#[cfg(test)]
mod tests {
    use super::seeded_report;

    #[test]
    fn a_seeded_run_fails_or_drops_its_test_case() {
        let report = "<testsuites>\n<testcase name=\"a\" file=\"t.js\"/>\n<testcase name=\"b\" file=\"t.js\"/>\n</testsuites>\n";
        let runs = [
            ("a".to_string(), "failed".to_string()),
            ("b".to_string(), "absent".to_string()),
        ];
        assert_eq!(
            seeded_report(report, &runs),
            "<testsuites>\n<testcase name=\"a\" file=\"t.js\"><failure message=\"seeded\"/></testcase>\n\n</testsuites>\n"
        );
    }
}
