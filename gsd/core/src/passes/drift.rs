//! The drift pass: `gsd/W406`, a research, phase spec or validation recorded against a context that has changed
//! since. GSD compares file times (`src/gate-context-drift.cts`); a pass sees no history, so the artifact carries the
//! `context_digest` it was recorded with and the compute pass gives the context its current `digest`.
//!
//! As upstream: no context in the artifact's phase, or an artifact that records no digest, is not judged, and the
//! project's research (no phase) has no context to drift from.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use crate::digest::scope_of;

/// The kinds that record `context_digest` (`validation` is declared in round r39).
const RECORDS_DIGEST: &[&str] = &["research", "phase_spec", "validation"];

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("drift", |p| {
        p.phase("check").run(check);
    });
}

/// What GSD tells the user to do about the stale artifact (`gate-context-drift`'s messages).
fn suggestion(kind: &str, phase: &str) -> String {
    let number = phase.rsplit('/').next().unwrap_or(phase);
    let how = match kind {
        "research" => format!("regenerate the research: /gsd-plan-phase {number} --research"),
        "phase_spec" => {
            "regenerate or reconcile the spec against the current decisions".to_string()
        }
        _ => "regenerate or reconcile the validation against the current decisions".to_string(),
    };
    format!("{how}, then record it again; do not hand-inject the newer decisions into a prompt")
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    // The first context of each scope by id (E206 reports a second).
    let mut contexts: BTreeMap<&str, &PassEntity> = BTreeMap::new();
    for context in input.entities.iter().filter(|e| e.kind == "context") {
        let slot = contexts.entry(scope_of(&context.id)).or_insert(context);
        if context.id < slot.id {
            *slot = context;
        }
    }
    input
        .entities
        .iter()
        .filter(|e| RECORDS_DIGEST.contains(&e.kind.as_str()))
        .filter_map(|artifact| {
            let recorded = artifact.fields.get("context_digest")?;
            let context = contexts.get(scope_of(&artifact.id))?;
            let current = context.fields.get("digest")?;
            (recorded != current).then(|| {
                PassDiagnostic::new(
                    "W406",
                    PassSeverity::Warning,
                    format!(
                        "{} '{}' was recorded against an older context: '{}' changed since",
                        artifact.kind.replace('_', " "),
                        artifact.id,
                        context.id
                    ),
                )
                .with_suggestion(suggestion(&artifact.kind, scope_of(&artifact.id)))
                .with_entity(&artifact.id)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
