//! `gsd/E307`, the part a declarative rule cannot read: `auto_select` and a recorded `answer.option` name one of the
//! checkpoint's options, and option names are unique (`src/verify.cts:1217`). The required fields of each checkpoint
//! kind are declarative (`rules.rs`). GSD checks `auto_select` only in a `decision` checkpoint that has options. A
//! checkpoint's `after` names a task of its own plan.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use crate::digest::scope_of;
use crate::passes::plans::{block, of_kind, rows};

pub fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "checkpoint")
        .flat_map(|checkpoint| {
            let names: Vec<String> = rows(checkpoint, "options")
                .into_iter()
                .filter_map(|mut option| option.remove("name"))
                .collect();
            let available = if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            };
            let id = &checkpoint.id;
            let invalid = |message: String| {
                PassDiagnostic::new("E307", PassSeverity::Error, message)
                    .with_suggestion("name one of the checkpoint's options")
                    .with_entity(id)
            };
            let mut found = Vec::new();
            let mut seen = BTreeSet::new();
            for name in &names {
                if !seen.insert(name) {
                    found.push(invalid(format!(
                        "checkpoint '{id}' has two options named '{name}'"
                    )));
                }
            }
            let decision = checkpoint.fields.get("kind").map(String::as_str) == Some("decision");
            let selected = checkpoint.fields.get("auto_select").filter(|_| decision);
            if let Some(selected) = selected {
                if selected.trim().is_empty() {
                    found.push(invalid(format!(
                        "checkpoint '{id}' auto_select is empty — name an option"
                    )));
                } else if !names.is_empty() && !names.contains(selected) {
                    found.push(invalid(format!(
                        "checkpoint '{id}' auto_select=\"{selected}\" does not match any option (available: {available})"
                    )));
                }
            }
            if let Some(after) = input
                .edges
                .iter()
                .find(|e| e.source == *id && e.label == "after")
                .filter(|e| scope_of(&e.target) != scope_of(id))
            {
                found.push(invalid(format!(
                    "checkpoint '{id}' is after '{}', a task of another plan",
                    after.target
                )));
            }
            let answer = block(checkpoint, "answer");
            let chosen = answer.get("option").filter(|c| !c.is_empty());
            if let Some(chosen) = chosen.filter(|c| !names.contains(c)) {
                found.push(invalid(format!(
                    "checkpoint '{id}' answer.option=\"{chosen}\" does not match any option (available: {available})"
                )));
            }
            found
        })
        .collect()
}
