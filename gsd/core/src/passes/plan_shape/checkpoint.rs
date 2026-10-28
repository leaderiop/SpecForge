//! `gsd/E307`, the part a declarative rule cannot read: `auto_select` and a recorded `answer.option` name one of the
//! checkpoint's options, and option names are unique (`src/verify.cts:1217`). The required fields of each checkpoint
//! kind are declarative (`rules.rs`).

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

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
            if let Some(selected) = checkpoint.fields.get("auto_select") {
                if selected.trim().is_empty() {
                    found.push(invalid(format!(
                        "checkpoint '{id}' auto_select is empty — name an option"
                    )));
                } else if !names.contains(selected) {
                    found.push(invalid(format!(
                        "checkpoint '{id}' auto_select=\"{selected}\" does not match any option (available: {available})"
                    )));
                }
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
