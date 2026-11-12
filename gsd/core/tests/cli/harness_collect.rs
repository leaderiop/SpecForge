//! The harness's repository and collects: a project's git history, written with a pinned identity and pinned dates,
//! and `specforge collect` over it.

use std::path::Path;
use std::process::Command;

use specforge_extension_sdk::prelude::TestStatus;

use super::{BASELINE_AT, Project, Run};

impl Project {
    /// Commits the project as it is now: the baseline the history rules (`I`, `+`, append-only) compare against.
    /// Git runs with its identity on the command line, never the global configuration, and its dates are pinned.
    pub fn commit(&self) {
        self.ensure_repository();
        self.git(&["add", "-A"], BASELINE_AT);
        self.git(
            &["commit", "-q", "--allow-empty", "-m", "baseline"],
            BASELINE_AT,
        );
    }

    /// Writes `files` (path, text) and commits just them with `subject`, the author and committer time `at`
    /// (`YYYY-MM-DDTHH:MM:SSZ`). It answers the commit's full hash.
    pub fn git_commit(&self, subject: &str, files: &[(&str, &str)], at: &str) -> String {
        self.ensure_repository();
        for (path, text) in files {
            self.write(path, text);
            self.git(&["add", "--", path], at);
        }
        self.git(&["commit", "-q", "--allow-empty", "-m", subject], at);
        let head = self.git(&["rev-parse", "HEAD"], at);
        head.trim().to_string()
    }

    /// `git init` in the project, with no commit; a repository already there is left as it is.
    pub fn init_repository(&self) {
        self.ensure_repository();
    }

    fn ensure_repository(&self) {
        if !self.root.join(".git").exists() {
            self.git(&["init", "-q", "-b", "main"], BASELINE_AT);
        }
    }

    /// `git <args>` in the project with a pinned identity and pinned dates; it answers standard output.
    pub fn git(&self, args: &[&str], at: &str) -> String {
        let output = Command::new("git")
            .args(["-c", "user.name=gsd", "-c", "user.email=gsd@example.com"])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args([
                "-c",
                "core.excludesFile=/dev/null",
                "-c",
                "tag.gpgSign=false",
            ])
            .args(args)
            // Not the developer's configuration, hooks, signing or repository.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("GIT_AUTHOR_DATE", at)
            // The committer's date differs from the author's, so `%at` is told from `%ct`.
            .env("GIT_COMMITTER_DATE", "2030-01-01T00:00:00Z")
            .current_dir(&self.root)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// `specforge collect <args> --yes --format json`.
    pub fn collect(&self, args: &[&str]) -> Run {
        let mut all = vec!["collect"];
        all.extend_from_slice(args);
        all.extend(["--yes", "--format", "json"]);
        self.run(&all)
    }

    /// `collect --runner node-test --no-run --report` over the twin's recorded JUnit report
    /// (`goldens/taskboard/junit.xml`): the thread's test facts without running node.
    pub fn collect_node_test_golden(&self) -> Run {
        let golden =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/goldens/taskboard/junit.xml");
        self.collect_report(&[], "node-test", &golden)
    }

    /// A recorded test run collected where nothing runs: the `collectors` whose facts `runner` reads first (each must
    /// succeed), then `runner` over `report` with `--no-run`. It answers that last collect.
    pub fn collect_report(&self, collectors: &[&str], runner: &str, report: &Path) -> Run {
        for collector in collectors {
            let run = self.collect(&["--runner", collector]);
            assert_eq!(run.code, 0, "{collector}: {}", run.all());
        }
        self.collect(&[
            "--runner",
            runner,
            "--no-run",
            "--report",
            &report.display().to_string(),
        ])
    }

    /// Collects `cases` (`(file, name, status)`) as `@code/core` test facts, as `@javascript/node-test` reads node's
    /// JUnit report: loads `@code/core` and `@javascript/node-test` when they are not loaded, writes `package.json`
    /// (`{"name":"taskboard"}`) when the project has none, writes the cases as one report at
    /// `.specforge/reports/node-test.xml` and runs `collect --runner node-test --no-run`. A `[gsd:<id>]` tag in a name
    /// puts `<id>` in the fact's `names`. Nothing runs node.
    pub fn node_tests(&self, cases: &[(&str, &str, TestStatus)]) -> Run {
        let missing: Vec<&str> = ["@code/core", "@javascript/node-test"]
            .into_iter()
            .filter(|name| !self.extensions.borrow().iter().any(|(n, _)| n == name))
            .collect();
        if !missing.is_empty() {
            self.extensions
                .borrow_mut()
                .extend(missing.iter().map(|name| (name.to_string(), None)));
            self.rewrite_extensions();
        }
        if !self.exists("package.json") {
            self.write("package.json", r#"{"name":"taskboard"}"#);
        }
        let body: String = cases
            .iter()
            .map(|(file, name, status)| {
                let inner = match status {
                    TestStatus::Passed => "",
                    TestStatus::Failed => "<failure message=\"no\"/>",
                    TestStatus::Skipped => "<skipped/>",
                };
                format!(
                    "<testcase name=\"{}\" classname=\"test\" file=\"{file}\">{inner}</testcase>\n",
                    escape(name)
                )
            })
            .collect();
        self.write(
            ".specforge/reports/node-test.xml",
            &format!("<?xml version=\"1.0\"?>\n<testsuites>\n{body}</testsuites>\n"),
        );
        self.collect(&["--runner", "node-test", "--no-run"])
    }

    /// `(source, target)` of each edge labelled `label` in the graph, as `specforge export --format graph` writes
    /// it, sorted.
    pub fn edges_of(&self, label: &str) -> Vec<(String, String)> {
        let run = self.run(&["export", "--format", "graph"]);
        let graph: serde_json::Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("export prints JSON ({e}): {}", run.all()));
        let mut edges: Vec<(String, String)> = graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .filter(|e| e["label"] == label)
            .map(|e| {
                let end = |key: &str| e[key].as_str().unwrap_or_default().to_string();
                (end("source"), end("target"))
            })
            .collect();
        edges.sort();
        edges
    }

    /// Records one passing test for the `execute` project's deliverable check (`test/tasks.test.js#adds a task`) as
    /// a test fact ([`Project::node_tests`]). Once a test has been collected, a check no test ran is `unknown` and
    /// `gsd/W408` says so; this settles it.
    pub fn passing_tests(&self) {
        let run = self.node_tests(&[("test/tasks.test.js", "adds a task", TestStatus::Passed)]);
        assert_eq!(run.code, 0, "{}", run.all());
    }
}

/// `text` as an XML attribute value.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}
