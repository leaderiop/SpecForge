//! The assets' shell lines, run under bash with a `PATH` that holds no jq and no node: `--pick` reads, the batch list
//! as tab-separated lines, and the checks once written with jq.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod answers;
mod batch;
mod pick;
mod relics;
mod review;
mod scripts;
mod shapes;

/// The tools the `--pick` lines call besides `specforge`: no jq.
const PICK_TOOLS: &[&str] = &[
    "git", "cat", "grep", "head", "tail", "sed", "awk", "cut", "tr", "wc", "sort", "paste", "env",
    "date", "mktemp", "mv", "rm",
];

/// `bash` with a `PATH` of the named tools and `specforge` alone.
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
    /// `bash` with a `PATH` of [`PICK_TOOLS`] and `specforge` (`SPECFORGE_BIN`), and no jq: a line that
    /// still ran jq would print nothing. `None` (and a skip note) when bash is absent.
    fn without_jq() -> Option<Shell> {
        Shell::only(PICK_TOOLS)
    }

    /// `bash` with a `PATH` of `tools` and `specforge` (`SPECFORGE_BIN`) alone. `None` (and a skip note) when bash is
    /// absent.
    fn only(tools: &[&str]) -> Option<Shell> {
        let Some(bash) = which("bash") else {
            eprintln!("skipped: bash is absent");
            return None;
        };
        let bin = tempfile::tempdir().expect("a bin dir");
        for tool in tools {
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

/// The first line of `text` that starts (after its indent) with `start`.
fn line<'a>(text: &'a str, start: &str) -> &'a str {
    text.lines()
        .map(str::trim_start)
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("no line starts with {start}"))
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
