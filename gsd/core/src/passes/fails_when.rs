//! The failing-direction pass: `gsd/E507`, each `automated` verify command states what makes it fail
//! (`gate-verify-failure-directions`, `resolveFailingDirection`). The sentinel `MISSING - Wave 0 ...` is not a
//! runnable command, so it has no failure to state. A bare `verify_text` is not a command and is not read.

use specforge_extension_sdk::prelude::*;

use super::plans::{of_kind, rows};
use super::verify_paths::missing_sentinel;

/// A whole statement that says nothing (GSD's list, compared in lower case).
const PLACEHOLDERS: &[&str] = &[
    "tbd", "todo", "n/a", "na", "none", "unknown", "tba", "?", "-",
];

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("fails_when", |p| {
        p.phase("check").run(check);
    });
}

/// Why a statement does not name a failing direction, for one command.
fn problem(command: &str, statement: Option<&str>) -> Option<&'static str> {
    if command.is_empty() || missing_sentinel(command) {
        return None;
    }
    match statement.map(str::trim) {
        None => Some("states no `fails_when`"),
        Some("") => Some("has an empty `fails_when`"),
        Some(s) if PLACEHOLDERS.contains(&s.to_lowercase().as_str()) => {
            Some("has a placeholder `fails_when`")
        }
        Some(_) => None,
    }
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "task")
        .flat_map(|task| {
            rows(task, "verify").into_iter().filter_map(move |row| {
                let command = row.get("automated")?.trim();
                let why = problem(command, row.get("fails_when").map(String::as_str))?;
                Some(
                    PassDiagnostic::new(
                        "E507",
                        PassSeverity::Error,
                        format!("task '{}': the verify command `{command}` {why}", task.id),
                    )
                    .with_suggestion(
                        "say what makes the command fail, e.g. \"the test run exits non-zero\"",
                    )
                    .with_entity(&task.id),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
