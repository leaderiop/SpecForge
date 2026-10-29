//! Runs the `specforge` binary on a project that loads the built `@gsd/core`.
//!
//! `SPECFORGE_BIN` is the binary and `GSD_CORE_WASM` the component; a missing one fails the test naming the command
//! that builds it.

use std::path::{Path, PathBuf};
use std::process::Command;

const HOW_TO_BUILD: &str = "build it: (cd <repo> && cargo build -p specforge-cli) and (cd gsd && cargo build -p gsd-core --release --target wasm32-wasip2), then set SPECFORGE_BIN and GSD_CORE_WASM to the two artifacts";

fn required(var: &str) -> PathBuf {
    match std::env::var_os(var).map(|value| existing_file(var, &value)) {
        Some(Ok(path)) => path,
        Some(Err(why)) => panic!("{why}; {HOW_TO_BUILD}"),
        None => panic!("{var} is not set; {HOW_TO_BUILD}"),
    }
}

/// `value` as the file `var` names. It must be absolute: cargo runs the tests in `gsd/core`, and each test project
/// resolves `@gsd/core=<path>` from its own temp directory, so a relative path means something else to each.
fn existing_file(var: &str, value: &std::ffi::OsStr) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if !path.is_absolute() {
        Err(format!(
            "{var}={} is a relative path; use an absolute one (e.g. $PWD/target/...)",
            path.display()
        ))
    } else if !path.is_file() {
        Err(format!("{var}={} is not a file", path.display()))
    } else {
        Ok(path.to_path_buf())
    }
}

/// The pinned time of the baseline commit.
const BASELINE_AT: &str = "2026-10-28T08:00:00Z";

/// What one `specforge` run answered.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// Everything it printed: the diagnostics go to stderr, the answers to stdout.
    pub fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// A copy of `tests/projects/<name>/` in a temp dir, with `@gsd/core` loaded from `GSD_CORE_WASM`.
pub struct Project {
    dir: tempfile::TempDir,
    wasm: PathBuf,
}

pub fn project(name: &str) -> Project {
    from_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/projects")
            .join(name),
    )
}

/// The parity suite's taskboard fixture: its `spec/` twin of the GSD project beside its `.planning/`.
pub fn taskboard() -> Project {
    from_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures/taskboard"))
}

fn from_dir(source: &Path) -> Project {
    let wasm = required("GSD_CORE_WASM");
    let dir = tempfile::tempdir().expect("a temp dir");
    copy_dir(source, dir.path());
    let p = Project { dir, wasm };
    p.settings(serde_json::json!({}));
    p
}

impl Project {
    /// Rewrites `specforge.json`: `@gsd/core` loaded from the built component, with `settings` under its name.
    pub fn settings(&self, settings: serde_json::Value) {
        let manifest = serde_json::json!({
            "name": "gsd-test",
            "version": "0.1.0",
            "spec_root": "spec",
            "extensions": [format!("@gsd/core={}", self.wasm.display())],
            "settings": { "@gsd/core": settings },
        });
        self.write("specforge.json", &manifest.to_string());
    }

    /// Runs `specforge <args>` in the project's root.
    pub fn run(&self, args: &[&str]) -> Run {
        self.run_with(args, &[])
    }

    /// Runs `specforge <args>` with these environment variables set.
    pub fn run_with(&self, args: &[&str], env: &[(&str, &str)]) -> Run {
        self.spawn(args, env, "")
    }

    /// Runs `specforge <args>` with `stdin` on its standard input (a host's hook event).
    pub fn run_stdin(&self, args: &[&str], stdin: &str) -> Run {
        self.spawn(args, &[], stdin)
    }

    fn spawn(&self, args: &[&str], env: &[(&str, &str)], stdin: &str) -> Run {
        use std::io::Write;
        use std::process::Stdio;
        let mut child = Command::new(required("SPECFORGE_BIN"))
            .args(args)
            .envs(env.iter().copied())
            .current_dir(self.dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run specforge");
        child
            .stdin
            .take()
            .expect("a piped stdin")
            .write_all(stdin.as_bytes())
            .expect("write stdin");
        let output = child.wait_with_output().expect("wait for specforge");
        Run {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// The project's root, as an absolute path.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// `(code, message)` of every `gsd/` diagnostic of `specforge check --format json`, sorted.
    pub fn findings(&self) -> Vec<(String, String)> {
        let run = self.run(&["check", "--format", "json"]);
        let diagnostics: serde_json::Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
        let mut found: Vec<(String, String)> = diagnostics
            .as_array()
            .expect("a list")
            .iter()
            .filter(|d| d["code"].as_str().is_some_and(|c| c.starts_with("gsd/")))
            .map(|d| {
                (
                    d["code"].as_str().unwrap_or_default().to_string(),
                    d["message"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        found.sort();
        found
    }

    /// `(code, message)` of every diagnostic of `specforge check --format json` (kernel and `gsd/`), in report order,
    /// the `I204` note and the `I205` note ("no test report") left out: the compute pass reads the report, so every
    /// project that has none says I205 once, and the tests of the rules are not about that.
    pub fn report(&self) -> Vec<(String, String)> {
        let run = self.run(&["check", "--format", "json"]);
        let found: serde_json::Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
        found
            .as_array()
            .expect("a list")
            .iter()
            .filter_map(|d| {
                let text = |key: &str| d[key].as_str().unwrap_or_default().to_string();
                Some((d["code"].as_str()?.to_string(), text("message")))
            })
            .filter(|(code, _)| code != "I204" && code != "I205")
            .collect()
    }

    /// Every code of `specforge check --format json` (kernel and `gsd/`, `I204` and `I205` left out), in report order.
    pub fn codes(&self) -> Vec<String> {
        self.report().into_iter().map(|(code, _)| code).collect()
    }

    /// The fields of one entity, as `specforge query <id> --format json` answers them.
    pub fn fields_of(&self, id: &str) -> serde_json::Value {
        let query = self.run(&["query", id, "--depth", "0", "--format", "json"]);
        let graph: serde_json::Value = serde_json::from_str(&query.stdout)
            .unwrap_or_else(|e| panic!("query is JSON ({e}): {}", query.all()));
        graph["nodes"][0]["fields"].clone()
    }

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
        if !self.dir.path().join(".git").exists() {
            self.git(&["init", "-q", "-b", "main"], BASELINE_AT);
        }
    }

    /// `git <args>` in the project with a pinned identity and pinned dates; it answers standard output.
    fn git(&self, args: &[&str], at: &str) -> String {
        let output = Command::new("git")
            .args(["-c", "user.name=gsd", "-c", "user.email=gsd@example.com"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .env("GIT_AUTHOR_DATE", at)
            .env("GIT_COMMITTER_DATE", at)
            .current_dir(self.dir.path())
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

    /// The nodes of `kind` in the graph, as `specforge export --format graph` writes them.
    pub fn nodes_of(&self, kind: &str) -> Vec<serde_json::Value> {
        let run = self.run(&["export", "--format", "graph"]);
        let graph: serde_json::Value = serde_json::from_str(&run.stdout)
            .unwrap_or_else(|e| panic!("export prints JSON ({e}): {}", run.all()));
        graph["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .filter(|n| n["kind"] == kind)
            .cloned()
            .collect()
    }

    /// Records `value_json` (compact JSON) as the harness value `field` of `entity` in `specforge-harness`, as the
    /// host's ledger writes it: `path key sha256(path NUL key NUL value)`, sorted, under the header.
    pub fn record_harness(&self, entity: &str, field: &str, value_json: &str) {
        use sha2::{Digest, Sha256};
        let file = "specforge-harness";
        let header = "# specforge-harness 1";
        let mut lines: Vec<String> = std::fs::read_to_string(self.dir.path().join(file))
            .map(|text| text.lines().skip(1).map(String::from).collect())
            .unwrap_or_default();
        let digest = Sha256::digest(format!("{entity}\0{field}\0{value_json}").as_bytes());
        lines.push(format!("{entity} {field} {digest:x}"));
        lines.sort();
        lines.dedup();
        self.write(file, &format!("{header}\n{}\n", lines.join("\n")));
    }

    /// Replaces `from` by `to` in a file of the project. The anchor must be there: a fixture that was reformatted
    /// must fail the test, not silently leave it editing nothing.
    pub fn edit(&self, relative: &str, from: &str, to: &str) {
        let text = self.read(relative);
        assert!(
            text.contains(from),
            "`{from}` is not in {relative}:\n{text}"
        );
        self.write(relative, &text.replacen(from, to, 1));
    }

    /// Whether a path of the project exists.
    pub fn exists(&self, relative: &str) -> bool {
        self.dir.path().join(relative).exists()
    }

    /// A file of the project, as it is now.
    pub fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(relative))
            .unwrap_or_else(|e| panic!("read {relative}: {e}"))
    }

    /// Writes (or replaces) a file of the project, creating its folders.
    pub fn write(&self, relative: &str, text: &str) {
        let path = self.dir.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("create dirs");
        std::fs::write(path, text).expect("write file");
    }
}

fn copy_dir(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap_or_else(|e| panic!("read {}: {e}", from.display()))
    {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            std::fs::create_dir_all(&target).expect("create dir");
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("copy file");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::existing_file;

    #[test]
    fn a_relative_path_is_refused_whether_or_not_the_file_exists() {
        let why = existing_file("GSD_CORE_WASM", "Cargo.toml".as_ref()).unwrap_err();
        assert!(
            why.contains("relative path") && why.contains("absolute"),
            "{why}"
        );
        let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        assert!(existing_file("X", manifest.as_ref()).is_ok());
        let missing = concat!(env!("CARGO_MANIFEST_DIR"), "/nothing");
        assert!(
            existing_file("X", missing.as_ref())
                .unwrap_err()
                .contains("not a file")
        );
    }
}
