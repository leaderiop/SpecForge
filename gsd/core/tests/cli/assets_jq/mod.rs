//! Plan 03 (F9): the shell lines that read JSON with `jq` where upstream's run node, run under bash with a `PATH` that
//! holds no node. Where node is installed, upstream's line runs on the same input and gives the same answer.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod pick;
mod readers;
mod scripts;

/// The tools the lines below call; nothing else is on the `PATH`.
const TOOLS: &[&str] = &[
    "jq", "git", "cat", "mv", "rm", "mktemp", "grep", "head", "sed", "awk", "cut", "tr", "env",
];

/// The tools the `--pick` lines call besides `specforge`: no jq.
const PICK_TOOLS: &[&str] = &[
    "git", "cat", "grep", "head", "tail", "sed", "awk", "cut", "tr", "wc", "sort", "paste", "env",
];

/// `bash` with a `PATH` of [`TOOLS`] alone, or `None` (and a skip note) when bash or jq is absent.
struct Shell {
    bash: PathBuf,
    bin: tempfile::TempDir,
}

fn which(tool: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .args(["-c", &format!("command -v {tool}")])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && path.starts_with('/')).then(|| PathBuf::from(path))
}

impl Shell {
    fn new() -> Option<Shell> {
        let (Some(bash), Some(_)) = (which("bash"), which("jq")) else {
            eprintln!("skipped: bash or jq is absent");
            return None;
        };
        let bin = tempfile::tempdir().expect("a bin dir");
        for tool in TOOLS {
            if let Some(path) = which(tool) {
                std::os::unix::fs::symlink(path, bin.path().join(tool)).expect("a link");
            }
        }
        Some(Shell { bash, bin })
    }

    /// G4 (plan 04): `bash` with a `PATH` of [`PICK_TOOLS`] and `specforge` (`SPECFORGE_BIN`), and no jq: a line that
    /// still ran jq would print nothing. `None` (and a skip note) when bash is absent.
    fn without_jq() -> Option<Shell> {
        let Some(bash) = which("bash") else {
            eprintln!("skipped: bash is absent");
            return None;
        };
        let bin = tempfile::tempdir().expect("a bin dir");
        for tool in PICK_TOOLS {
            if let Some(path) = which(tool) {
                std::os::unix::fs::symlink(path, bin.path().join(tool)).expect("a link");
            }
        }
        let specforge = std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN (see harness.rs)");
        std::os::unix::fs::symlink(specforge, bin.path().join("specforge")).expect("a link");
        Some(Shell { bash, bin })
    }

    /// `script` under `bash -c` in `dir`: `(stdout, stderr, exit code)`.
    fn run_in(&self, dir: &Path, script: &str) -> (String, String, i32) {
        let out = Command::new(&self.bash)
            .current_dir(dir)
            .env("PATH", self.bin.path())
            .args(["-c", script])
            .stdin(Stdio::null())
            .output()
            .expect("bash runs");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.code().unwrap_or(-1),
        )
    }

    fn run(&self, script: &str) -> String {
        self.run_in(self.bin.path(), script).0
    }
}

/// The asset `path` (under `assets/`) as text.
fn asset(path: &str) -> String {
    std::fs::read_to_string(format!("{}/assets/{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Upstream's copy of `path` in the vendored snapshot: its own name, or `<name>.txt`.
fn upstream(path: &str) -> String {
    let plain = format!("{}/{path}", gsd_kit::snapshot!());
    std::fs::read_to_string(&plain)
        .or_else(|_| std::fs::read_to_string(format!("{plain}.txt")))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The first line of `text` that starts (after its indent) with `start`.
fn line<'a>(text: &'a str, start: &str) -> &'a str {
    text.lines()
        .map(str::trim_start)
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("no line starts with {start}"))
}

fn has_node() -> bool {
    which("node").is_some()
}

/// Upstream's `script` with node on the `PATH` too.
fn run_with_node(script: &str) -> String {
    let out = Command::new("bash")
        .args(["-c", script])
        .stdin(Stdio::null())
        .output()
        .expect("bash runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Single-quotes `s` for bash.
fn q(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// The lines of `text` from the first that starts (after its indent) with `start` through the first that contains
/// `end`.
fn block(text: &str, start: &str, end: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let a = lines
        .iter()
        .position(|l| l.trim_start().starts_with(start))
        .unwrap_or_else(|| panic!("no line starts with {start}"));
    let b = a + lines[a..]
        .iter()
        .position(|l| l.contains(end))
        .unwrap_or_else(|| panic!("no line after {start} contains {end}"));
    lines[a..=b].join("\n")
}

/// The line after the first that starts (after its indent) with `start`.
fn after<'a>(text: &'a str, start: &str) -> &'a str {
    let mut lines = text
        .lines()
        .skip_while(|l| !l.trim_start().starts_with(start));
    lines.next();
    lines
        .next()
        .unwrap_or_else(|| panic!("nothing after {start}"))
}

/// `setup`, then our `code` (no node on the `PATH`), then `show`, prints `want`; so does upstream's `theirs` (node on
/// the `PATH`) where node is installed. A trailing newline does not count: the lines' callers read `$(…)`.
fn both(sh: &Shell, setup: &str, ours: &str, theirs: Option<&str>, show: &str, want: &str) {
    let script = |code: &str| format!("{setup}\n{code}\n{show}");
    let trim = |s: String| s.trim_end_matches('\n').to_string();
    assert_eq!(trim(sh.run(&script(ours))), want, "ours after {setup}");
    if let Some(theirs) = theirs.filter(|_| has_node()) {
        assert_eq!(
            trim(run_with_node(&script(theirs))),
            want,
            "upstream's after {setup}"
        );
    }
}
