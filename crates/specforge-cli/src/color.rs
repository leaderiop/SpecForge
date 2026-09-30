//! Whether human-readable diagnostics are colour-coded.
//!
//! The rule, first match wins:
//! 1. `NO_COLOR` set to a non-empty value: never colour (no-color.org).
//! 2. `CLICOLOR_FORCE` set to a non-empty value other than `0`: colour,
//!    even into a pipe or a file.
//! 3. Otherwise: colour only when the stream is a terminal.
//!
//! Machine-readable (JSON) output never asks, so it is never coloured.

use std::io::IsTerminal;

/// Whether diagnostics printed to stderr are coloured.
pub fn stderr() -> bool {
    decide(std::io::stderr().is_terminal())
}

/// Whether diagnostics printed to stdout are coloured.
pub fn stdout() -> bool {
    decide(std::io::stdout().is_terminal())
}

fn decide(is_terminal: bool) -> bool {
    let set = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    if set("NO_COLOR").is_some() {
        return false;
    }
    if set("CLICOLOR_FORCE").is_some_and(|v| v != "0") {
        return true;
    }
    is_terminal
}
