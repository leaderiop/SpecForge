//! `core.human_guard` (ADR 0301 Limits, K20c): the host's own deny-policy hook on a host's shell tool. A command line
//! that runs a human-attested command (`specforge <short> <command>`, `CommandDescriptor.human`) is refused, because
//! the check that stdin is a terminal passes where the agent's own shell is a pty (Cursor's terminal, Codex's pty
//! exec). The commands are read from the routed descriptors, not from a list, and the reason names the person's route.
//!
//! The line is matched as a whole, not at the executable's position: any word that is `specforge` (any case, a path,
//! `.exe`, even inside `$(…)`) followed anywhere by `<short> <command>` or `<short>:<command>` of a human route is
//! refused, so a wrapper with options, a shell keyword or a brace cannot hide it. A `-c` argument of a shell and the
//! argument of `eval` are read the same way. A line that names specforge and cannot be read is refused too. A false
//! positive (`echo specforge t approve`) is the safe direction: the reason tells the agent to ask the person.
//!
//! shortcut: a script file, a variable, `script(1)` typed by a person on the agent's behalf and a host with no shell
//! hook are not seen; they are the bound ADR 0301 names. Upgrade when a host can say who typed.

use specforge_protocol_types::Decision;

use super::Outcome;
use crate::host::{HookAnswer, HostHookEvent, shell};
use crate::view::ProjectView;

pub use crate::host::render::HUMAN_GUARD;

const SHELLS: [&str; 5] = ["sh", "bash", "zsh", "dash", "ksh"];

/// What the guard found in a line.
enum Found {
    Command(String),
    Unreadable,
}

/// The word as a program name: its last `/` segment, lowercased, without `.exe` and without the quoting or
/// substitution characters glued to it (`specforge)`, `` specforge` ``).
fn program(word: &str) -> String {
    let name = word
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(word)
        .to_lowercase();
    let name = name.trim_matches(|c: char| !c.is_alphanumeric());
    name.strip_suffix(".exe").unwrap_or(name).to_string()
}

/// The human `specforge <short> <command>` that `line` runs, or that it hides from reading.
fn human_in(view: &ProjectView<'_>, line: &str, depth: u8) -> Option<Found> {
    let words = match shell::words(line) {
        Ok(words) => words,
        Err(_) if line.to_lowercase().contains("specforge") => return Some(Found::Unreadable),
        Err(_) => return None,
    };
    let lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
    let routes = view.registries().commands().routed();
    for (i, word) in words.iter().enumerate() {
        if program(word) == "specforge" {
            let hit = routes.iter().filter(|r| r.declaration().human).find(|r| {
                let (short, cli) = (r.short().to_lowercase(), r.cli_name().to_lowercase());
                let joined = format!("{short}:{cli}");
                (i + 1..lower.len()).any(|j| {
                    lower[j] == joined || (lower[j] == short && lower.get(j + 1) == Some(&cli))
                })
            });
            if let Some(r) = hit {
                return Some(Found::Command(format!(
                    "specforge {} {}",
                    r.short(),
                    r.cli_name()
                )));
            }
        }
        // The text a shell is handed with `-c`, and `eval`'s: read once more.
        let runs_text = i > 0
            && depth < 3
            && (program(&words[i - 1]) == "eval"
                || (words[i - 1].starts_with('-')
                    && !words[i - 1].starts_with("--")
                    && words[i - 1].contains('c')
                    && words[..i - 1]
                        .iter()
                        .any(|w| SHELLS.contains(&program(w).as_str()))));
        if runs_text {
            if let Some(found) = human_in(view, word, depth + 1) {
                return Some(found);
            }
        }
    }
    None
}

pub(super) fn run(view: &ProjectView<'_>, event: &HostHookEvent) -> Outcome {
    let Some(line) = event
        .tool
        .as_ref()
        .filter(|t| t.class == "shell")
        .and_then(|t| t.input.get("command")?.as_str())
    else {
        return Outcome::default();
    };
    let Some(found) = human_in(view, line, 0) else {
        return Outcome::default();
    };
    let reason = match found {
        Found::Command(found) => format!(
            "`{found}` records what a person says, so it runs only for a person at their own terminal \
             (ADR 0301). Ask the person to run it themselves; do not run it from a shell tool."
        ),
        Found::Unreadable => "this command line names specforge and cannot be read, so the human-attested commands \
             it might run cannot be ruled out (ADR 0301). Write it plainly, or ask the person to run it."
            .to_string(),
    };
    Outcome {
        answer: HookAnswer {
            decision: Some(Decision::Deny),
            reason: Some(reason),
            ..HookAnswer::default()
        },
        ..Outcome::default()
    }
}
