//! The edge types of a registry build, resolved once (ADR 0060): each one
//! with every field that writes it, its ends, and which edge type each
//! `(kind, field)` writes. The rule set, the entity snapshot, the schema and
//! trace read the answer; none derives it again.

use std::collections::{BTreeMap, HashMap};

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::{EdgeTypeDescriptor, ExtensionDeclaration, FieldDescriptor};

/// A field or member that names an edge: its extension's load index, kind, declared path, label, extension and
/// descriptor.
type Candidate<'a> = (
    usize,
    &'a str,
    String,
    &'a str,
    &'a str,
    &'a FieldDescriptor,
);

use crate::{EdgeRegistry, EdgeRegistryEntry, EdgeWriter, FieldRegistry};

/// Every edge type of `declarations` (in load order), resolved once:
/// `declared` (the edge types the declarations declare, each with the
/// extension declaring it; the first in load order won and populate's W018
/// reported the rest), then every field of `fields` that names an `edge` as
/// that edge type's writer. A writer of a declared edge type outside one of
/// its declared ends is W021 and writes nothing; a label no extension
/// declares is an edge type of its writers.
///
/// Writers are ordered by extension in load order, then kind, then field,
/// whatever order the field registry holds them in.
pub(super) fn resolve(
    declarations: &[ExtensionDeclaration],
    declared: Vec<(String, EdgeTypeDescriptor)>,
    fields: &FieldRegistry,
) -> (EdgeRegistry, Vec<Diagnostic>) {
    let declared: BTreeMap<String, (String, EdgeTypeDescriptor)> = declared
        .into_iter()
        .map(|(extension, edge)| (edge.label.clone(), (extension, edge)))
        .collect();
    let load_index = |extension: &str| {
        declarations
            .iter()
            .position(|d| d.name() == extension)
            .unwrap_or(usize::MAX)
    };

    // Every field and every member of a typed block that names an `edge` (ADR 0285): a member writes its edge under
    // its declared member path (`requires.phase`).
    let mut candidates: Vec<Candidate<'_>> = Vec::new();
    for (kind, field, entry) in fields.iter() {
        let extension = load_index(entry.source_extension());
        entry.declared().for_each_path(field, &mut |path, d| {
            if let Some(label) = d.edge.as_deref() {
                candidates.push((extension, kind, path, label, entry.source_extension(), d));
            }
        });
    }
    candidates.sort_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));

    let mut diagnostics = Vec::new();
    let mut writers: BTreeMap<&str, Vec<EdgeWriter>> = BTreeMap::new();
    let mut written = HashMap::new();
    for (_, kind, field, label, source_extension, descriptor) in candidates {
        let field = field.as_str();
        let target_kind = descriptor.target_kind.clone();
        if let Some((_, edge)) = declared.get(label)
            && let Some(problem) = outside_an_end(edge, kind, target_kind.as_deref())
        {
            diagnostics.push(Diagnostic::new(
                codes::W021,
                format!(
                    "extension '{}': field '{field}' on kind '{kind}' writes edge type '{label}', whose declared {problem} — its edges are not of that type",
                    source_extension
                ),
            ));
            continue;
        }
        written.insert((kind.to_string(), field.to_string()), label.to_string());
        writers.entry(label).or_default().push(EdgeWriter {
            extension: source_extension.to_string(),
            kind: kind.to_string(),
            field: field.to_string(),
            target_kind,
        });
    }

    let mut entries = BTreeMap::new();
    for (label, (extension, edge)) in &declared {
        let writers = writers.remove(label.as_str()).unwrap_or_default();
        entries.insert(
            label.clone(),
            EdgeRegistryEntry::resolved(label, extension, Some(edge.clone()), writers),
        );
    }
    // A label no extension declares is an edge type of its writers, owned
    // by the first one's extension.
    for (label, writers) in writers {
        let owner = writers[0].extension.clone();
        entries.insert(
            label.to_string(),
            EdgeRegistryEntry::resolved(label, &owner, None, writers),
        );
    }
    (EdgeRegistry::resolved(entries, written), diagnostics)
}

/// Why a field of `kind` writing `edge` towards `target` is outside the
/// ends `edge` declares, when it is: the declared end it breaks, as the
/// W021 words it.
fn outside_an_end(edge: &EdgeTypeDescriptor, kind: &str, target: Option<&str>) -> Option<String> {
    if let Some(source) = edge.source_kind.as_deref()
        && source != kind
    {
        return Some(format!("source_kind is '{source}'"));
    }
    if let Some(declared) = edge.target_kind.as_deref()
        && let Some(target) = target
        && declared != target
    {
        return Some(format!("target_kind is '{declared}', not '{target}'"));
    }
    None
}
