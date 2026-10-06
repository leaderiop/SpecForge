use specforge_common::Diagnostic;
use specforge_formatter::unified_diff;
use specforge_ops::format::{self, Mode, Request};
use std::io::{self, Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};

/// Run the `specforge format` command.
///
/// Returns the process exit code, one rule in every mode (write, `--check`,
/// `--diff`, `--stdin`):
/// - 1 when a file could not be read or written, or has a region left
///   unformatted (W142); the other files, and the formatted parts, are still
///   written or printed;
/// - 1 under `--check` when a file would change;
/// - 0 otherwise.
pub fn run(path: &Path, check: bool, diff: bool, stdin: bool, explicit_paths: &[String]) -> i32 {
    let project_root = format::project_root(path);
    if stdin {
        return run_stdin(&project_root, path);
    }
    let explicit: Vec<PathBuf> = explicit_paths.iter().map(Into::into).collect();
    let mode = Mode::of_flags(check, diff, None);
    let outcome = format::run(&Request {
        root: &project_root,
        paths: &explicit,
        mode,
    });
    if outcome.found_nothing() {
        eprintln!("No .spec files found");
        return 0;
    }
    for d in &outcome.diagnostics {
        print_diagnostic(d);
    }
    for failure in &outcome.failures {
        eprintln!("error: {failure}");
    }

    for change in &outcome.changes {
        let shown = change.path.display().to_string();
        if diff {
            print!(
                "{}",
                unified_diff(&shown, &change.before, &change.after).diff_text
            );
        } else if change.written || mode == Mode::Check {
            println!("{shown}");
        }
    }

    if mode == Mode::Write {
        eprintln!(
            "Formatted {} file(s), {} changed",
            outcome.checked,
            outcome.changes.len()
        );
    }

    if !outcome.succeeded() || !outcome.complete() || (check && !outcome.changes.is_empty()) {
        1
    } else {
        0
    }
}

/// A diagnostic on stderr: `<file>: <message>` when it names a file, the
/// bare message when it is spanned in unnamed text (stdin), else
/// `warning: <message>` (a configuration file's W141 names its file itself).
fn print_diagnostic(d: &Diagnostic) {
    match &d.span {
        Some(span) if !span.file.as_str().is_empty() => {
            eprintln!("{}: {}", span.file.as_str(), d.message);
        }
        Some(_) => eprintln!("{}", d.message),
        None => eprintln!("warning: {}", d.message),
    }
}

/// Format stdin, text of the project at `root` in `dir`, and write it to
/// stdout: it gets the configuration a file in `dir` would. The formatted
/// text is printed even when a region was left unformatted; the exit code
/// is then 1.
fn run_stdin(root: &Path, dir: &Path) -> i32 {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("error: failed to read stdin: {e}");
        return 1;
    }

    let place = format::Place::InProject { root, dir };
    let result = format::document(place, &input, None, None);

    for d in &result.diagnostics {
        print_diagnostic(d);
    }

    print!("{}", result.formatted);
    io::stdout().flush().ok();

    if result.complete() { 0 } else { 1 }
}
