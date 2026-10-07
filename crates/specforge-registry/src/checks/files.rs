//! E016: a path a `file_reference` field names that does not exist, and the
//! one derivation of those paths the session's check inputs share.

use std::collections::BTreeSet;
use std::path::Path;

use specforge_common::{Diagnostic, codes, find_close_match};

use crate::FieldRegistry;
use crate::entity::{EntityRecord, ValueShape};

/// The names of the fields a path is read from, sorted and unique.
pub(super) fn reference_fields(fields: &FieldRegistry) -> BTreeSet<&str> {
    fields.file_reference_fields()
}

/// The paths `record` names in its `file_reference` fields (those in
/// `names`), in field-name order, each field's last occurrence: the items
/// of a list of strings.
pub(super) fn paths<'a>(
    record: &'a EntityRecord,
    names: &'a BTreeSet<&str>,
) -> impl Iterator<Item = &'a str> {
    names.iter().flat_map(move |name| {
        record
            .fields
            .iter()
            .rev()
            .find(|field| field.key == *name)
            .filter(|field| field.shape == ValueShape::Strings)
            .and_then(|field| field.items.as_deref())
            .into_iter()
            .flatten()
            .map(String::as_str)
    })
}

/// E016 for every path a `file_reference` field names that does not exist
/// under `spec_root`, with a close sibling as the suggestion.
pub(super) fn missing(
    entities: &[EntityRecord],
    fields: &FieldRegistry,
    spec_root: &Path,
) -> Vec<Diagnostic> {
    let names = reference_fields(fields);
    let mut diagnostics = Vec::new();
    for record in entities {
        for path in paths(record, &names) {
            if spec_root.join(path).exists() {
                continue;
            }
            let mut diagnostic = Diagnostic::new(
                codes::E016,
                format!(
                    "file reference '{}' in entity '{}' does not exist",
                    path, record.id
                ),
            )
            .with_span(record.span.clone());
            if let Some(suggestion) = suggest_similar_file(path, spec_root) {
                diagnostic = diagnostic.with_suggestion(suggestion);
            }
            diagnostics.push(diagnostic);
        }
    }
    diagnostics
}

fn suggest_similar_file(path: &str, spec_root: &Path) -> Option<String> {
    let full_path = spec_root.join(path);
    let parent = full_path.parent()?;
    if !parent.is_dir() {
        return None;
    }

    let file_name = full_path.file_name()?.to_str()?;
    let siblings: Vec<String> = std::fs::read_dir(parent)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let match_name = find_close_match(file_name, siblings.iter().map(|s| s.as_str()))?;

    // Reconstruct the relative path with the suggested filename
    let path_obj = Path::new(path);
    let suggested = if let Some(dir) = path_obj.parent().filter(|p| !p.as_os_str().is_empty()) {
        format!("{}/{}", dir.display(), match_name)
    } else {
        match_name.to_string()
    };

    Some(format!("did you mean '{}'?", suggested))
}
