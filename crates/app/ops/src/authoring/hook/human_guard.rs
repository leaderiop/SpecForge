//! `core.human_guard` (ADR 0301 Limits, K20c): the host's own deny-policy hook on a host's shell tool. A command line
//! that runs a human-attested command (`specforge <short> <command>`, `CommandDescriptor.human`) is refused, because
//! the check that stdin is a terminal passes where the agent's own shell is a pty (Cursor's terminal, Codex's pty
//! exec). The commands are read from the routed descriptors, not from a list, and the reason names the person's route.
//!
//! shortcut: a command line is matched on its words, and a word that holds spaces (`sh -c "specforge …"`) is split
//! once more; a longer indirection (a script, `eval`, a variable) is not seen, and `script(1)` typed by a person on
//! the agent's behalf is the bound ADR 0301 names. Upgrade when a host can say who typed.

use specforge_protocol_types::Decision;

use super::Outcome;
use crate::host::{HookAnswer, HostHookEvent, shell};
use crate::view::ProjectView;

pub use crate::host::render::HUMAN_GUARD;

const WRAPPERS: [&str; 7] = ["env", "command", "exec", "sudo", "nohup", "time", "nice"];

/// The `specforge <short> <command>` a command's words run, when it is a human-attested one.
fn human_in(view: &ProjectView<'_>, words: &[String]) -> Option<String> {
    let routes = view.registries().commands().routed();
    // The executable: after `NAME=value` words and a wrapper, so `echo specforge …` is no call.
    let start = words
        .iter()
        .position(|w| !w.contains('=') && !WRAPPERS.contains(&w.as_str()))?;
    if words[start].rsplit('/').next() != Some("specforge") {
        return None;
    }
    words[start..].windows(3).find_map(|w| {
        routes
            .iter()
            .find(|r| r.short() == w[1] && r.cli_name() == w[2] && r.declaration().human)
            .map(|r| format!("specforge {} {}", r.short(), r.cli_name()))
    })
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
    // A line that cannot be split is judged as one command of its words as they stand.
    let commands = shell::commands(line).unwrap_or_default();
    let inner = commands
        .iter()
        .flatten()
        .filter(|w| w.contains(char::is_whitespace))
        .filter_map(|w| shell::commands(w).ok())
        .flatten();
    let Some(found) = commands
        .iter()
        .cloned()
        .chain(inner)
        .find_map(|c| human_in(view, &c))
    else {
        return Outcome::default();
    };
    Outcome {
        answer: HookAnswer {
            decision: Some(Decision::Deny),
            reason: Some(format!(
                "`{found}` records what a person says, so it runs only for a person at their own terminal \
                 (ADR 0301). Ask the person to run it themselves; do not run it from a shell tool."
            )),
            ..HookAnswer::default()
        },
        ..Outcome::default()
    }
}
