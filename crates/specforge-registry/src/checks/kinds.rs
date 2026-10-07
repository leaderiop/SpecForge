//! E024: an entity of a kind no loaded extension declares.

use specforge_common::{Diagnostic, codes, structural};

use super::index::KeywordExtensionIndex;
use crate::KindRegistry;
use crate::entity::EntityRecord;

/// E024 for every entity whose kind is neither structural (`spec`, `ref`,
/// `use`, `define`) nor registered, with the builtin extension that would
/// declare it as the suggestion when the bundled index knows one.
pub(super) fn unknown(entities: &[EntityRecord], kinds: &KindRegistry) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for entity in entities {
        let (keyword, id, span) = (entity.kind.as_str(), entity.id.as_str(), &entity.span);
        if structural::is_structural(keyword) || kinds.contains(keyword) {
            continue;
        }

        // The bundled index loads only once an unknown keyword turns up.
        let suggestion = Some(match KeywordExtensionIndex::bundled().lookup(keyword) {
            Some(ext) => format!("install it with: specforge add {ext}"),
            None => {
                format!("search for an extension that provides it with: specforge search {keyword}")
            }
        });

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
