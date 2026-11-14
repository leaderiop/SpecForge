//! The probe-rows pass: `gsd/E523`, an `edge` or `spec_prohibition` of a phase spec whose resolution breaks its
//! lifecycle. A row carries what its status allows (the probe engine's `validate_resolution`, with the edge or the
//! prohibition vocabulary), its requirement is a locked requirement of its own spec, and a spec holds one edge per
//! requirement and category (upstream's duplicate rejection). The check descriptor of a prohibition (`check_*`) is not
//! judged: a partial one fails closed where it is read.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::commits::target;
use crate::probe::edge::EDGE_VALIDATORS;
use crate::probe::{Resolution, validate_prohibition_resolution, validate_resolution};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("probe_rows", |p| {
        p.phase("check").run(check);
    });
}

fn diagnostic(id: &str, message: String) -> PassDiagnostic {
    PassDiagnostic::new("E523", PassSeverity::Error, message)
        .with_suggestion(
            "write the row's resolution in one edit: `specforge gsd resolve-edge` or `specforge gsd resolve-prohibition`",
        )
        .with_entity(id)
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut rows: Vec<&PassEntity> = input
        .entities
        .iter()
        .filter(|e| e.kind == "edge" || e.kind == "spec_prohibition")
        .collect();
    rows.sort_by_key(|e| crate::commands::support::natural(&e.id));
    let mut out = Vec::new();
    let mut pairs = BTreeSet::new();
    for row in rows {
        let text = |field: &str| row.fields.get(field).cloned();
        let resolution = Resolution {
            requirement_id: row.id.clone(),
            category: String::new(),
            status: text("status").unwrap_or_default(),
            verification: text("verification"),
            resolution: text("resolution"),
            reason: text("reason"),
        };
        let judged = match row.kind.as_str() {
            "edge" => validate_resolution(&resolution, &EDGE_VALIDATORS),
            _ => validate_prohibition_resolution(&resolution),
        };
        if let Err(why) = judged {
            // upstream keys a row `<requirement>::<category>`; the row's own id names it here
            out.push(diagnostic(
                &row.id,
                why.replace(&format!("{}::", row.id), &row.id),
            ));
        }
        let spec = target(input, &row.id, "spec");
        let requirement = target(input, &row.id, "requirement");
        if let Some(req) = requirement
            && target(input, req, "spec") != spec
        {
            out.push(diagnostic(
                &row.id,
                format!(
                    "{} '{}' names requirement '{req}', which is not a locked requirement of its spec '{}'",
                    row.kind,
                    row.id,
                    spec.unwrap_or_default()
                ),
            ));
        }
        if row.kind == "edge"
            && let (Some(req), Some(category)) = (requirement, row.fields.get("category"))
            && !pairs.insert((spec, req, category.as_str()))
        {
            out.push(diagnostic(
                &row.id,
                format!(
                    "duplicate resolution for {req}::{category} (edge '{}')",
                    row.id
                ),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests;
