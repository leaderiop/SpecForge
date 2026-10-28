//! Runs the `specforge` binary on a project that loads the built `@gsd/core`.
//!
//! `SPECFORGE_BIN` is the binary and `GSD_CORE_WASM` the component; a missing one fails the test naming the command
//! that builds it.

use std::path::{Path, PathBuf};
use std::process::Command;

const HOW_TO_BUILD: &str = "build it: (cd <repo> && cargo build -p specforge-cli) and (cd gsd && cargo build -p gsd-core --release --target wasm32-wasip2), then set SPECFORGE_BIN and GSD_CORE_WASM to the two artifacts";

fn required(var: &str) -> PathBuf {
    match std::env::var_os(var) {
        Some(path) if Path::new(&path).is_file() => PathBuf::from(path),
        Some(path) => panic!(
            "{var}={} is not a file; {HOW_TO_BUILD}",
            Path::new(&path).display()
        ),
        None => panic!("{var} is not set; {HOW_TO_BUILD}"),
    }
}

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
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/projects")
        .join(name);
    let wasm = required("GSD_CORE_WASM");
    let dir = tempfile::tempdir().expect("a temp dir");
    copy_dir(&source, dir.path());
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
        let output = Command::new(required("SPECFORGE_BIN"))
            .args(args)
            .envs(env.iter().copied())
            .current_dir(self.dir.path())
            .output()
            .expect("run specforge");
        Run {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
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
