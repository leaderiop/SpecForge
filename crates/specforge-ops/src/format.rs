//! `specforge format` and the MCP `specforge.format` tool: one run over a
//! project's `.spec` files.
//!
//! A file that can't be read or written is recorded and the run goes on, so
//! a failure on one file never leaves the others half-done. The surfaces
//! present what happened: the CLI prints it, MCP returns it.

use specforge_common::Diagnostic;
use specforge_formatter::{FormatConfig, discover_targets, format_source, load_config};
use std::path::{Path, PathBuf};

/// What the run does with a file whose formatting would change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Rewrite it.
    Write,
    /// Report it, without writing.
    Check,
}

pub struct Request<'a> {
    /// The project root (where `specforge.json` lives). `.specforgefmt.toml`
    /// discovery stops here.
    pub root: &'a Path,
    /// Where `.specforgefmt.toml` discovery starts, walking up to `root`.
    pub config_dir: &'a Path,
    /// Files or directories to format; empty means every `.spec` file under
    /// the project's `spec/` directory (or the root, without one).
    pub paths: &'a [PathBuf],
    pub mode: Mode,
}

/// A file whose formatting differs from its content.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub before: String,
    pub after: String,
    /// Why writing it failed, in write mode.
    pub write_error: Option<String>,
}

impl FileChange {
    /// Whether the formatted text is on disk now.
    pub fn written(&self, mode: Mode) -> bool {
        mode == Mode::Write && self.write_error.is_none()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Files read and formatted.
    pub checked: usize,
    /// Files whose formatting differs, in discovery order.
    pub changes: Vec<FileChange>,
    /// Files that couldn't be read, with why.
    pub unreadable: Vec<(PathBuf, String)>,
    /// What loading `.specforgefmt.toml` reported.
    pub config_diagnostics: Vec<Diagnostic>,
    /// What formatting each file reported (parse errors kept verbatim).
    pub file_diagnostics: Vec<(PathBuf, Diagnostic)>,
}

impl Outcome {
    /// The changed files whose write failed.
    pub fn write_failures(&self) -> impl Iterator<Item = &FileChange> {
        self.changes.iter().filter(|c| c.write_error.is_some())
    }
}

/// The project root `path` is in, else `path` itself.
pub fn project_root(path: &Path) -> PathBuf {
    specforge_common::find_project_root(path).unwrap_or_else(|| path.to_path_buf())
}

/// The format configuration for `request`, with what loading it reported.
pub fn config(request: &Request) -> (FormatConfig, Vec<Diagnostic>) {
    load_config(request.config_dir, request.root)
}

/// The files `request` formats.
pub fn targets(request: &Request) -> Vec<PathBuf> {
    let spec_dir = request.root.join("spec");
    let search_root = if spec_dir.exists() {
        spec_dir
    } else {
        request.root.to_path_buf()
    };
    discover_targets(&search_root, request.paths, &[])
}

/// Format every target of `request`.
pub fn run(request: &Request) -> Outcome {
    let (config, config_diagnostics) = config(request);
    let mut outcome = Outcome {
        config_diagnostics,
        ..Outcome::default()
    };
    for target in targets(request) {
        let source = match std::fs::read_to_string(&target) {
            Ok(source) => source,
            Err(e) => {
                outcome.unreadable.push((target, e.to_string()));
                continue;
            }
        };
        outcome.checked += 1;
        let result = format_source(&source, &config);
        outcome
            .file_diagnostics
            .extend(result.diagnostics.into_iter().map(|d| (target.clone(), d)));
        if result.formatted == source {
            continue;
        }
        let write_error = match request.mode {
            Mode::Write => std::fs::write(&target, &result.formatted)
                .err()
                .map(|e| e.to_string()),
            Mode::Check => None,
        };
        outcome.changes.push(FileChange {
            path: target,
            before: source,
            after: result.formatted,
            write_error,
        });
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESSY: &str = "behavior messy \"Messy\" {\ncontract \"The system MUST work\"\n}\n";

    fn project() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("specforge.json"), "{}").unwrap();
        std::fs::create_dir(dir.path().join("spec")).unwrap();
        std::fs::write(dir.path().join("spec/a.spec"), MESSY).unwrap();
        std::fs::write(dir.path().join("spec/b.spec"), MESSY.replace("messy", "b")).unwrap();
        dir
    }

    fn request(root: &Path, mode: Mode) -> Request<'_> {
        Request {
            root,
            config_dir: root,
            paths: &[],
            mode,
        }
    }

    #[test]
    fn check_mode_reports_every_change_and_writes_nothing() {
        let dir = project();

        let outcome = run(&request(dir.path(), Mode::Check));

        assert_eq!(outcome.checked, 2);
        assert_eq!(outcome.changes.len(), 2);
        assert!(outcome.changes.iter().all(|c| !c.written(Mode::Check)));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("spec/a.spec")).unwrap(),
            MESSY
        );
    }

    #[test]
    fn write_mode_rewrites_and_a_second_run_is_clean() {
        let dir = project();

        let first = run(&request(dir.path(), Mode::Write));
        let second = run(&request(dir.path(), Mode::Write));

        assert!(first.changes.iter().all(|c| c.written(Mode::Write)));
        assert_eq!(second.checked, 2);
        assert!(second.changes.is_empty());
    }
}
