//! The harness's repository and collects: a project's git history, written with a pinned identity and pinned dates,
//! and `specforge collect` over it.

use std::path::Path;
use std::process::Command;

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

    /// `collect --runner node-test --no-run` over the twin's recorded JUnit report (`goldens/taskboard/junit.xml`),
    /// put where `@javascript/node-test` reads it: the thread's test facts without running node.
    pub fn collect_node_test_golden(&self) -> Run {
        let golden =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/goldens/taskboard/junit.xml");
        let report = std::fs::read_to_string(&golden).expect("the twin's recorded JUnit report");
        self.write(".specforge/reports/node-test.xml", &report);
        self.collect(&["--runner", "node-test", "--no-run"])
    }

    /// Records one passing test for the `execute` project's deliverable check (`test/tasks.test.js#adds a task`), as
    /// `gsd-junit` reads a JUnit report. Once a test runner has recorded a result, a check no test ran is `unknown`
    /// and `gsd/W408` says so; this settles it.
    pub fn passing_tests(&self) {
        let file = self
            .dir
            .path()
            .canonicalize()
            .expect("root")
            .join("test/tasks.test.js");
        self.write(
            "junit.xml",
            &format!(
                "<?xml version=\"1.0\"?>\n<testsuites>\n<testsuite name=\"s\">\n<testcase name=\"adds a task\" classname=\"test\" file=\"{}\"/>\n</testsuite>\n</testsuites>\n",
                file.display()
            ),
        );
        let run = self.collect(&["--runner", "gsd-junit", "--no-run"]);
        assert_eq!(run.code, 0, "{}", run.all());
    }
}
