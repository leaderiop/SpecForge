//! What the phase says of the values their owner state holds back (ADR 0350).

use std::collections::BTreeMap;

use specforge_common::{Diagnostic, codes};

/// I213, once per compile: how many values wait for their owner to complete, by `kind.path` (ADR 0350). `None` when
/// none does: a closed value waits for nothing.
pub(super) fn waiting(waiting: &BTreeMap<String, usize>) -> Option<Diagnostic> {
    let total: usize = waiting.values().sum();
    if total == 0 {
        return None;
    }
    let paths: Vec<String> = waiting
        .iter()
        .map(|(path, n)| format!("{path} {n}"))
        .collect();
    let (values, wait, they) = if total == 1 {
        ("value", "waits", "it is")
    } else {
        ("values", "wait", "they are")
    };
    Some(Diagnostic::new(
        codes::I213,
        format!(
            "{total} {values} {wait} for their owner to complete ({}): {they} checked once it is",
            paths.join(", ")
        ),
    ))
}
