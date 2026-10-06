use specforge_formatter::unified_diff;
use specforge_ops::format::{self, Mode, Request};
use std::io::{self, Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};

/// Run the `specforge format` command.
///
/// Returns the process exit code:
/// - 0: all files already formatted (or successfully formatted)
/// - 1: in `--check` mode, some files would change; or a file couldn't be
///   written (the others still are)
pub fn run(path: &Path, check: bool, diff: bool, stdin: bool, explicit_paths: &[String]) -> i32 {
    let project_root = format::project_root(path);
    let explicit: Vec<PathBuf> = explicit_paths.iter().map(Into::into).collect();
    let request = Request {
        root: &project_root,
        paths: &explicit,
        mode: if check || diff {
            Mode::Check
        } else {
            Mode::Write
        },
    };

    if stdin {
        return run_stdin(&project_root, path);
    }
    let outcome = format::run(&request);
    print_config_warnings(&outcome.config_diagnostics);
    if outcome.found_nothing() {
        eprintln!("No .spec files found");
        return 0;
    }
    for (file, error) in &outcome.unreadable {
        eprintln!("error: failed to read {}: {error}", file.display());
    }
    for (file, d) in &outcome.file_diagnostics {
        eprintln!("{}: {}", file.display(), d.message);
    }

    for change in &outcome.changes {
        let shown = change.path.display().to_string();
        if diff {
            print!(
                "{}",
                unified_diff(&shown, &change.before, &change.after).diff_text
            );
        } else if let Some(error) = &change.write_error {
            eprintln!("error: failed to write {shown}: {error}");
        } else {
            println!("{shown}");
        }
    }

    if !check && !diff {
        eprintln!(
            "Formatted {} file(s), {} changed",
            outcome.checked,
            outcome.changes.len()
        );
    }

    let failed = outcome.write_failures().next().is_some();
    if failed || (check && !outcome.changes.is_empty()) {
        1
    } else {
        0
    }
}

fn print_config_warnings(diagnostics: &[specforge_common::Diagnostic]) {
    for d in diagnostics {
        eprintln!("warning: {}", d.message);
    }
}

/// Format stdin, text of the project at `root` in `dir`, and write it to
/// stdout: it gets the configuration a file in `dir` would.
fn run_stdin(root: &Path, dir: &Path) -> i32 {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("error: failed to read stdin: {e}");
        return 1;
    }

    let place = format::Place::InProject { root, dir };
    let result = format::document(place, &input, None, None);

    for d in &result.diagnostics {
        if d.span.is_some() {
            eprintln!("{}", d.message);
        } else {
            eprintln!("warning: {}", d.message);
        }
    }

    print!("{}", result.formatted);
    io::stdout().flush().ok();

    0
}
