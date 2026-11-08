//! What the phase says of the values their owner state holds back (ADR 0350).

use std::collections::BTreeMap;

use specforge_common::{Diagnostic, codes};
use specforge_effects::Roots;
use specforge_effects::contained::Contained;
use specforge_registry::{Disk, DiskLookup};

use super::collect::Held;
use super::dispatch::{backslashed, refusal};
use crate::Environment;

/// What the written form of each planned path says while its owner is pending (ADR 0350 D5): read in its root without
/// the disk, a refusal is E094 or E095 and a backslash W175, placed as a probe's are. A link, a case difference and
/// existence wait for the owner to complete.
pub(super) fn written_forms<'h, 'a: 'h>(
    env: &Environment,
    disk: &dyn Disk,
    held: impl Iterator<Item = &'h Held<'a>>,
) -> Vec<Diagnostic> {
    let roots = env
        .effects_allowed()
        .with_named(Roots::new(env.root(), env.spec_root()));
    let lookup = DiskLookup(disk);
    let contained = Contained::new(&roots, &lookup);
    held.filter_map(|one| {
        let root = one.root.clone()?;
        match contained.lexical(one.value.text, root) {
            Err(refused) => refusal(&one.value, one.value.text, &refused),
            Ok(read) if read.backslashes => Some(backslashed(&one.value, one.value.text)),
            Ok(_) => None,
        }
    })
    .collect()
}

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
