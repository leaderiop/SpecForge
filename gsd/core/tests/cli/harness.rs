//! Runs the `specforge` binary on a project that loads `@gsd/core` by name, the builtin the binary embeds (ADR 0366
//! D5), and with [`Project::with_extension`] another `@gsd/*` beside it.
//!
//! `SPECFORGE_BIN` is the binary and `GSD_SECURITY_WASM` the `@gsd/security` component; a missing one fails the test
//! naming the command that builds it. An edit to `gsd/core` reaches these tests once it is re-vendored and the binary
//! rebuilt.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "harness_collect.rs"]
mod collect;

#[path = "harness_ab.rs"]
pub mod ab;

#[path = "harness_answers.rs"]
pub mod answer_check;
use answer_check::answers_listed;

#[path = "harness_cites.rs"]
mod cites;

#[path = "harness_judged.rs"]
mod judged;
pub use judged::{Book, is_judged, judged, unjudged};

#[path = "harness_measures.rs"]
mod measures;
pub use measures::{Freshness, Verdict};

pub(crate) const HOW_TO_BUILD: &str = "build it: (cd <repo> && cargo build --profile test -p specforge-cli) and (cd gsd && cargo build -p gsd-security --release --target wasm32-wasip2), then set SPECFORGE_BIN and GSD_SECURITY_WASM to the artifacts";

fn required(var: &str) -> PathBuf {
    match std::env::var_os(var).map(|value| existing_file(var, &value)) {
        Some(Ok(path)) => path,
        Some(Err(why)) => panic!("{why}; {HOW_TO_BUILD}"),
        None => panic!("{var} is not set; {HOW_TO_BUILD}"),
    }
}

/// The compile cache every run shares, whatever its `HOME` (an install's temp home would compile `@gsd/core` and
/// every builtin again): the test process's own, as `specforge_component::user_compile_cache_dir` finds it.
pub fn compile_cache() -> std::ffi::OsString {
    match (
        std::env::var_os("SPECFORGE_WASMTIME_CACHE"),
        std::env::var_os("HOME"),
    ) {
        (Some(set), _) => set,
        (None, Some(home)) => Path::new(&home).join(".cache/specforge/wasmtime").into(),
        (None, None) => "off".into(),
    }
}

/// `value` as the file `var` names. It must be absolute: cargo runs the tests in `gsd/core`, and each test project
/// resolves a component's `<name>=<path>` (`@gsd/security`'s) from its own temp directory, so a relative path means
/// something else to each. `@gsd/core` is the binary's builtin and has no path.
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

/// The work every `specforge` run of these tests holds a guest call to, in wasm instructions (`SPECFORGE_WASM_FUEL`),
/// in place of any clock: a hook's `timeout_ms` and an extension's `max_execution_ms` (held as the work it allows).
/// Whether a call answers then depends on its input, not on how busy the machine is: under the workspace's tests a
/// 2.6 s hook passed its 5000 ms timeout, and a 3.5 s pass its 30 s limit (#157, #159). The most a call of the suite
/// spends (2026-11-03): 5.6 G, the secret guard on 127 closed spans (`hooks_hostile`); about half a hook's timeout
/// at the ~4 G instructions/s measured on a loaded machine.
pub const WORK: u64 = 10_000_000_000;

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

/// A copy of `tests/projects/<name>/` in a temp dir, loading `@gsd/core` by name: the builtin `SPECFORGE_BIN` embeds.
pub struct Project {
    /// The temp dir, removed when the project drops.
    _dir: tempfile::TempDir,
    /// The project's root: the temp dir, or a directory inside it.
    root: PathBuf,
    /// The loaded extensions as `(name, component)`, `@gsd/core` first; a builtin has no component. A cell:
    /// [`Project::node_tests`] loads the test packs on a shared project.
    extensions: RefCell<Vec<(String, Option<PathBuf>)>>,
}

pub fn project(name: &str) -> Project {
    project_at(name, "")
}

/// [`project`] under `inside` of its temp dir: a project in a subdirectory of the repository a test makes there.
pub fn project_at(name: &str, inside: &str) -> Project {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/projects");
    from_dir_at(&fixtures.join(name), inside)
}

/// The builtins the taskboard twin loads beside `@gsd/core`, as the requirement thread configures the project: the
/// code facts, the runner pack that collects its tests and the source pack whose items the thread's code step reads.
pub const TWIN_BUILTINS: [&str; 3] = ["@code/core", "@javascript/node-test", "@javascript/source"];

/// The parity suite's taskboard fixture: its `spec/` twin of the GSD project beside its `.planning/`, loading
/// [`TWIN_BUILTINS`].
pub fn taskboard() -> Project {
    from_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures/taskboard")).twin()
}

/// The security parity suite's vault fixture: its `spec/` twin of the GSD project beside its `.planning/`, loading
/// [`TWIN_BUILTINS`] as the taskboard does.
#[allow(
    dead_code,
    reason = "core's own tests do not use it; @gsd/security's include this harness"
)]
pub fn vault() -> Project {
    from_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures/vault")).twin()
}

/// The collectors whose facts `rust-test` reads before a recorded nextest report: the manifest's (`rust-cargo`) and
/// the items' (`rust-source`).
pub const RUST_TEST_COLLECTORS: [&str; 2] = ["rust-cargo", "rust-source"];

/// The parity suite's ledger fixture: a Rust project whose CI is red, its `spec/` twin beside its `.planning/`,
/// loading beside `@gsd/core` the builtins `evaluation/gsd-ab/tasks.json` names as the ledger's `thread` (the code
/// facts and the Rust packs whose collectors read its manifest, its items and a nextest report).
#[allow(
    dead_code,
    reason = "@gsd/security's tests include this harness and do not use it"
)]
pub fn ledger() -> Project {
    let tasks = ab::ab_tasks();
    let thread: Vec<&str> = tasks["fixtures"]
        .as_array()
        .expect("fixtures")
        .iter()
        .find(|fixture| fixture["id"] == "ledger")
        .expect("the ledger fixture")["thread"]
        .as_array()
        .expect("its thread")
        .iter()
        .map(|builtin| builtin.as_str().expect("a builtin"))
        .collect();
    from_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures/ledger"))
        .with_builtins(&thread)
}

/// What a twin's `truth` and `summary` entities mirror in the `.planning/` at `planning`: each `must_haves` truth of
/// every `*-PLAN.md` as `(plan, text)` and the plan of every `*-SUMMARY.md`, from the directories under `phases/` and
/// `quick/` (when there is one), both sorted.
pub fn planning_truths_and_summaries(planning: &Path) -> (Vec<(String, String)>, Vec<String>) {
    let (mut truths, mut summaries) = (Vec::new(), Vec::new());
    for parent in ["phases", "quick"].map(|parent| planning.join(parent)) {
        if !parent.is_dir() {
            continue;
        }
        for dir in std::fs::read_dir(&parent).expect("a planning dir") {
            for file in std::fs::read_dir(dir.expect("a dir").path()).expect("a dir") {
                let path = file.expect("a file").path();
                let name = path
                    .file_name()
                    .expect("a name")
                    .to_string_lossy()
                    .into_owned();
                if let Some(plan) = name.strip_suffix("-PLAN.md") {
                    let text = std::fs::read_to_string(&path).expect("a planning file");
                    let block = text.split("  truths:\n").nth(1).expect("must_haves truths");
                    for line in block.lines().take_while(|l| l.starts_with("    - ")) {
                        truths.push((plan.to_string(), line[6..].trim_matches('"').to_string()));
                    }
                } else if let Some(plan) = name.strip_suffix("-SUMMARY.md") {
                    summaries.push(plan.to_string());
                }
            }
        }
    }
    truths.sort();
    summaries.sort();
    (truths, summaries)
}

/// [`taskboard`] under `inside` of its temp dir.
pub fn taskboard_at(inside: &str) -> Project {
    from_dir_at(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/fixtures/taskboard"),
        inside,
    )
    .twin()
}

fn from_dir(source: &Path) -> Project {
    from_dir_at(source, "")
}

/// [`from_dir`] with the project `inside` the temp dir (`.claude/worktrees/agent-x`: a subagent's worktree).
fn from_dir_at(source: &Path, inside: &str) -> Project {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join(inside);
    std::fs::create_dir_all(&root).expect("the project's directory");
    copy_dir(source, &root);
    let p = Project {
        _dir: dir,
        root,
        // `@specforge/checks` is a required peer of `@gsd/core`: its `files` fields are read by `@specforge/checks:file`.
        extensions: RefCell::new(vec![
            ("@gsd/core".to_string(), None),
            ("@specforge/checks".to_string(), None),
        ]),
    };
    p.settings(serde_json::json!({}));
    p
}

impl Project {
    /// Rewrites `specforge.json`: `@gsd/core` loaded from the built component, with `settings` under its name.
    pub fn settings(&self, settings: serde_json::Value) {
        self.settings_in("spec", settings);
    }

    /// [`Self::settings`] with the manifest's `spec_root` (`.`: the spec root is the project root).
    pub fn settings_in(&self, spec_root: &str, settings: serde_json::Value) {
        // another extension's block survives a rewrite of core's
        let mut manifest = self.manifest();
        let mut blocks = manifest["settings"].take();
        if !blocks.is_object() {
            blocks = serde_json::json!({});
        }
        blocks["@gsd/core"] = settings;
        let extensions: Vec<String> = self
            .extensions
            .borrow()
            .iter()
            .map(|(name, wasm)| match wasm {
                Some(wasm) => format!("{name}={}", wasm.display()),
                None => name.clone(),
            })
            .collect();
        let manifest = serde_json::json!({
            "name": "gsd-test",
            "version": "0.1.0",
            "spec_root": spec_root,
            "extensions": extensions,
            "settings": blocks,
        });
        self.write("specforge.json", &manifest.to_string());
    }

    /// Loads `name` too, from the component the environment variable `var` names (`@gsd/core` stays first).
    #[allow(
        dead_code,
        reason = "@gsd/security's tests include this file and use it; core's do not"
    )]
    pub fn with_extension(mut self, name: &str, var: &str) -> Self {
        self.extensions
            .get_mut()
            .push((name.to_string(), Some(required(var))));
        self.rewrite_extensions();
        self
    }

    /// The taskboard twin's setup: [`TWIN_BUILTINS`] loaded.
    fn twin(self) -> Self {
        self.with_builtins(&TWIN_BUILTINS)
    }

    /// Loads the builtins `names` too, by name, after `@gsd/core`.
    pub fn with_builtins(mut self, names: &[&str]) -> Self {
        self.extensions
            .get_mut()
            .extend(names.iter().map(|name| (name.to_string(), None)));
        self.rewrite_extensions();
        self
    }

    /// Rewrites `specforge.json`'s extensions, its spec root and `@gsd/core`'s settings kept.
    fn rewrite_extensions(&self) {
        let manifest = self.manifest();
        let spec_root = manifest["spec_root"].as_str().unwrap_or("spec").to_string();
        let core = manifest["settings"]["@gsd/core"].clone();
        self.settings_in(&spec_root, core);
    }

    /// Sets the settings block of `extension` (`@gsd/security`) in `specforge.json`.
    #[allow(
        dead_code,
        reason = "@gsd/security's tests include this file and use it; core's do not"
    )]
    pub fn settings_of(&self, extension: &str, settings: serde_json::Value) {
        let mut manifest = self.manifest();
        manifest["settings"][extension] = settings;
        self.write("specforge.json", &manifest.to_string());
    }

    /// `specforge.json` as written; an empty object before the first write.
    fn manifest(&self) -> serde_json::Value {
        std::fs::read_to_string(self.root.join("specforge.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Runs `specforge <args>` in the project's root.
    pub fn run(&self, args: &[&str]) -> Run {
        self.run_with(args, &[])
    }

    /// Runs `specforge <args>` with these environment variables set.
    pub fn run_with(&self, args: &[&str], env: &[(&str, &str)]) -> Run {
        self.spawn(args, env, "")
    }

    /// `specforge install <host>` in the project, with `HOME` set to a temp dir: an install never reaches the
    /// developer's home.
    pub fn install(&self, host: &str, env: &[(&str, &str)]) -> Run {
        let home = tempfile::tempdir().expect("a temp home");
        let home_path = home.path().display().to_string();
        let mut all = vec![("HOME", home_path.as_str())];
        all.extend_from_slice(env);
        self.run_with(&["install", host], &all)
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
            .env("SPECFORGE_WASMTIME_CACHE", compile_cache())
            .env("SPECFORGE_WASM_FUEL", WORK.to_string())
            .envs(env.iter().copied())
            .current_dir(&self.root)
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
        let run = Run {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        answers_listed(args, &run.stdout);
        run
    }

    /// Runs `specforge <args>` as a person at a terminal would: stdin is the slave side of a pty, so the host sees an
    /// interactive caller (ADR 0301). The master side stays open until the child exits.
    #[cfg(unix)]
    pub fn run_as_person(&self, args: &[&str]) -> Run {
        use rustix::fs::{Mode, OFlags, open};
        use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
        use std::process::Stdio;
        let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).expect("a pty master");
        grantpt(&master).expect("grantpt");
        unlockpt(&master).expect("unlockpt");
        let name = ptsname(&master, Vec::new()).expect("the pty slave's name");
        let slave = open(
            name.as_c_str(),
            OFlags::RDWR | OFlags::NOCTTY,
            Mode::empty(),
        )
        .expect("open the pty slave");
        let output = Command::new(required("SPECFORGE_BIN"))
            .args(args)
            .env("SPECFORGE_WASMTIME_CACHE", compile_cache())
            .env("SPECFORGE_WASM_FUEL", WORK.to_string())
            .current_dir(&self.root)
            .stdin(Stdio::from(slave))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run specforge");
        drop(master);
        Run {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// The project's root, as an absolute path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `(code, message)` of every `gsd/` diagnostic of `specforge check --format json`, sorted, the judged rules'
    /// left out (see [`Project::report`]).
    pub fn findings(&self) -> Vec<(String, String)> {
        self.findings_all()
            .into_iter()
            .filter(|(code, _)| !is_judged(code))
            .collect()
    }

    /// [`Project::findings`] with the judged rules' diagnostics.
    pub fn findings_all(&self) -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = self
            .checked()
            .into_iter()
            .filter(|(code, _)| code.starts_with("gsd/"))
            .collect();
        found.sort();
        found
    }

    /// `(code, message)` of every diagnostic of `specforge check --format json`, in report order, nothing left out.
    pub fn checked(&self) -> Vec<(String, String)> {
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
            .collect()
    }

    /// `(code, message)` of every diagnostic of `specforge check --format json` (kernel and `gsd/`), in report order,
    /// the `I204` note and the `I205` note ("not judged: no test results") left out: `gsd/E501`'s `test(..)` input is
    /// unjudged while a recorded verdict's tests are not collected, and the tests of the rules are not about that.
    /// The `I213` note (values that wait for their owner) is left out for the same reason: every project with a
    /// pending plan says it once.
    /// The judged rules' diagnostics (`no verdict`, `stale`, the outcome) are left out too: every plan, truth and so
    /// on of every fixture has one, and the tests of the other rules are not about them. [`Project::report_all`] has
    /// them, and the clean-report invariants and the judged tests use it.
    pub fn report(&self) -> Vec<(String, String)> {
        self.report_all()
            .into_iter()
            .filter(|(code, _)| !is_judged(code))
            .collect()
    }

    /// [`Project::report`] with the judged rules' diagnostics.
    pub fn report_all(&self) -> Vec<(String, String)> {
        self.checked()
            .into_iter()
            .filter(|(code, _)| !matches!(code.as_str(), "I204" | "I205" | "I213"))
            .collect()
    }

    /// Every code of `specforge check --format json` (kernel and `gsd/`, `I204`, `I205` and `I213` left out), in
    /// report order.
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
        let mut lines: Vec<String> = std::fs::read_to_string(self.root.join(file))
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

    /// Writes `lines` at the end of UAT test `name`'s block in `spec/uat.spec`, before its closing brace.
    pub fn close_uat_test(&self, name: &str, lines: &str) {
        let text = self.read("spec/uat.spec");
        let start = text
            .find(&format!("uat_test {name} "))
            .unwrap_or_else(|| panic!("no uat_test {name} in spec/uat.spec:\n{text}"));
        let end = start + text[start..].find("\n}").expect("the test's closing brace") + 1;
        self.write(
            "spec/uat.spec",
            &format!("{}{lines}{}", &text[..end], &text[end..]),
        );
    }

    /// Whether a path of the project exists.
    pub fn exists(&self, relative: &str) -> bool {
        self.root.join(relative).exists()
    }

    /// A file of the project, as it is now.
    pub fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.root.join(relative))
            .unwrap_or_else(|e| panic!("read {relative}: {e}"))
    }

    /// Writes (or replaces) a file of the project, creating its folders.
    pub fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
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
        let why = existing_file("GSD_SECURITY_WASM", "Cargo.toml".as_ref()).unwrap_err();
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
