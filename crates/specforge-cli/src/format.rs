use crate::outcome::{Exit, Refusal};
use specforge_common::Diagnostic;
use specforge_formatter::unified_diff;
use specforge_ops::format::{self, Mode, Request};
use specforge_ops::{OpError, OpErrorKind};
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
pub fn run(path: &Path, check: bool, diff: bool, stdin: bool, explicit_paths: &[String]) -> Exit {
    let project_root = specforge_common::project_root_of(path);
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
        return Exit::Passed;
    }
    for d in &outcome.diagnostics {
        print_diagnostic(d);
    }
    // A failed file is reported as every operation's failure is
    // (`error[CODE]: …`), its code and kind the OS-given ones.
    // The run's exit is its verdict (`outcome.ok()`), not this report's.
    for failure in &outcome.failures {
        let _ = Refusal::of(crate::OutputFormat::Human).report(&failure.to_op_error());
    }

    for change in &outcome.changes {
        let shown = change.path.display().to_string();
        if diff {
            print!(
                "{}",
                unified_diff(&shown, &change.before, &change.after).diff_text
            );
        } else if change.written || !mode.writes() {
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

    Exit::of_verdict(outcome.ok())
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
fn run_stdin(root: &Path, dir: &Path) -> Exit {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        return Refusal::of(crate::OutputFormat::Human).report(&OpError::new(
            OpErrorKind::of_io(&e),
            "file_unreadable",
            format!("failed to read stdin: {e}"),
        ));
    }

    let place = format::Place::InProject { root, dir };
    let result = format::document(place, &input, None, None);

    for d in &result.diagnostics {
        print_diagnostic(d);
    }

    print!("{}", result.formatted);
    io::stdout().flush().ok();

    Exit::of_verdict(result.complete())
}
