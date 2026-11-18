//! E024: an entity of a kind no loaded extension declares; and W151, the
//! notice that nothing checks the kinds at all.

use std::collections::BTreeSet;

use specforge_common::{Diagnostic, codes, structural};

use crate::entity::EntityRecord;
use crate::{CandidateVocabulary, KindRegistry};

/// E024 for every entity whose kind is neither structural (`spec`, `ref`,
/// `use`, `define`) nor registered, with the extensions that declare it as
/// the suggestion when the candidate vocabulary knows one: those that declare
/// no kind `kinds` holds, else all of them, saying so (ADR 0373 D4).
pub(super) fn unknown(
    entities: &[EntityRecord],
    kinds: &KindRegistry,
    candidates: &CandidateVocabulary,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for entity in entities {
        let (keyword, id, span) = (entity.kind.as_str(), entity.id.as_str(), &entity.span);
        if structural::is_structural(keyword) || kinds.contains(keyword) {
            continue;
        }

        let suggestion = Some(candidates.kind_remedy(keyword, |kind| kinds.contains(kind)));

        let mut diagnostic = Diagnostic::new(
            codes::E024,
            format!(
                "unknown entity kind '{}' for entity '{}' at {}",
                keyword, id, span.file
            ),
        )
        .with_span(span.clone());
        diagnostic.suggestion = suggestion;
        diagnostics.push(diagnostic);
    }

    diagnostics
}

/// The most kinds W151 names before it says there are more.
const KINDS_NAMED: usize = 5;

/// W151: no loaded extension declares an entity kind (`loaded`: some
/// extension is loaded, so I002 does not say so), yet the project writes
/// entities that are not structural; the kind, field and identifier checks
/// do not run on them. One diagnostic names how many and of which kinds
/// (sorted, unique, at most five). Its suggestion installs an extension that
/// declares the first kind, naming every one the candidate vocabulary knows,
/// each with its command ([`CandidateVocabulary::kind_install`], ADR 0369 D2),
/// else searches for one. It has no span: it is about the project, like I002.
/// The suggestion reads the rule E024 reads (ADR 0373 D4) with no kind
/// registered, since W151 runs only when the kind registry is empty: no
/// declarer collides and none is left out.
pub(super) fn unchecked(
    entities: &[EntityRecord],
    loaded: bool,
    candidates: &CandidateVocabulary,
) -> Vec<Diagnostic> {
    if !loaded {
        return Vec::new();
    }
    let unchecked: Vec<&EntityRecord> = entities
        .iter()
        .filter(|record| !structural::is_structural(&record.kind))
        .collect();
    let written: BTreeSet<&str> = unchecked
        .iter()
        .map(|record| record.kind.as_str())
        .collect();
    let Some(first) = written.iter().next().copied() else {
        return Vec::new();
    };
    let named: Vec<&str> = written.iter().take(KINDS_NAMED).copied().collect();
    let mut listed = named.join(", ");
    if written.len() > KINDS_NAMED {
        listed.push_str(", …");
    }
    let (count, noun, verb) = match unchecked.len() {
        1 => (1, "entity", "is"),
        n => (n, "entities", "are"),
    };
    let suggestion = candidates
        .kind_install(first, |_| false)
        .unwrap_or_else(|| {
            format!(
                "enable the extension that declares them; search with: {}",
                specforge_common::remedy::command("specforge extensions search {}", &[first])
            )
        });
    vec![
        Diagnostic::new(
            codes::W151,
            format!(
                "the loaded extensions declare no entity kind: {count} {noun} (kinds: {listed}) {verb} not checked against kinds, fields or identifiers"
            ),
        )
        .with_suggestion(suggestion),
    ]
}
