//! The glossary pass: `gsd/W538` over the terms (`spec/product.md`), beyond GSD (ADR 0357). A term's `avoid` lists
//! the words not to write for it, its `name` and `aliases` the words to write: one word on both sides is a glossary
//! that contradicts itself.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::plans::strings;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("glossary", |p| {
        p.phase("check").run(check);
    });
}

/// A word as the glossary compares it: each run of whitespace one space, then Unicode lowercase (`str::to_lowercase`,
/// not full case folding).
fn folded(word: &str) -> String {
    word.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// W538: a term whose `avoid` lists a word the glossary accepts, its own `name` or one of its `aliases` or another
/// term's, compared case-insensitively. One warning per term and avoided word, on the term, naming every term and
/// field that accepts the word.
fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut terms: Vec<&PassEntity> = input.entities.iter().filter(|e| e.kind == "term").collect();
    terms.sort_by(|a, b| a.id.cmp(&b.id));
    // A folded word → where the glossary accepts it, as (term, "name" or "alias"), in term order.
    let mut accepted: BTreeMap<String, Vec<(&str, &str)>> = BTreeMap::new();
    for term in &terms {
        let name = term.fields.get("name").map(|n| folded(n));
        for (word, how) in name.into_iter().map(|n| (n, "name")).chain(
            strings(term, "aliases")
                .iter()
                .map(|a| (folded(a), "alias")),
        ) {
            let places = accepted.entry(word).or_default();
            if !places.contains(&(term.id.as_str(), how)) {
                places.push((term.id.as_str(), how));
            }
        }
    }
    let mut found = Vec::new();
    for term in terms {
        let mut avoided: Vec<(String, String)> = Vec::new();
        for word in strings(term, "avoid") {
            let key = folded(&word);
            if accepted.contains_key(&key) && !avoided.iter().any(|(k, _)| *k == key) {
                avoided.push((key, word));
            }
        }
        for (key, word) in avoided {
            let places: Vec<String> = accepted[&key]
                .iter()
                .map(|&(id, how)| match (id == term.id, how) {
                    (true, "name") => "its own name".to_string(),
                    (true, _) => "its own alias".to_string(),
                    (false, "name") => format!("the name of term '{id}'"),
                    (false, _) => format!("an alias of term '{id}'"),
                })
                .collect();
            found.push(
                PassDiagnostic::new(
                    "W538",
                    PassSeverity::Warning,
                    format!(
                        "term '{}' avoids \"{word}\", which is {}",
                        term.id,
                        places.join(", ")
                    ),
                )
                .with_suggestion(format!(
                    "keep \"{word}\" on one side: remove it from `avoid`, or from the `name` or `aliases` that \
                     accept it"
                ))
                .with_entity(term.id.as_str()),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests;
