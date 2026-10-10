//! The terminal's answer to a publisher key change (`add` and `update` without `--yes`).

use specforge_registry_client::KeyChange;
use std::io::{IsTerminal, Write};

/// Ask the human to accept `change`. Refusal is the default, and the answer
/// when there is no terminal: blocking on a pipe nobody answers would hang.
pub(crate) fn ask_key_change(change: &KeyChange) -> bool {
    if !std::io::stdin().is_terminal() {
        return false;
    }
    eprintln!(
        "KEY CHANGE for '{}': pinned '{}' but new package is signed '{}'",
        change.package, change.pinned, change.offered
    );
    eprint!("trust the new key and re-pin? [y/N] ");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}
