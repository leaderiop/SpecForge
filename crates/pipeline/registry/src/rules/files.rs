//! The paths an entity's field names, and the close sibling of a missing one (ADR 0118): the one
//! reading every `file_exists` rule, E016 among them, shares.

use std::path::Path;

use specforge_common::{SourceSpan, find_close_match};
use specforge_graph::ValueShape;

use specforge_protocol_types::FileBase;

use crate::entity::{EntityRecord, FieldRecord, RuleInput};

/// One path a field names that does not exist under its base.
pub(super) struct Missing<'r> {
    /// As the entity wrote it: the scalar's text, or the list item.
    pub path: &'r str,
    /// `did you mean '<dir>/<name>'?`: a close sibling in the missing path's directory.
    pub suggestion: Option<String>,
    /// Where a block member writes the path; `None` for a field, which the entity's span stands for.
    pub span: Option<SourceSpan>,
}

/// The paths `record` names in `field`, in order, that `input.disk` says do not exist under `base`'s root: the spec
/// root, or the project root where the field declares that base (ADR 0285). A relative path starts at that root; an
/// absolute one is checked as written (`join` keeps it). `field` is a declared path: a member (`paths.file`) is read
/// in each item of its block. One `exists` per named path; only a missing one lists its directory.
pub(super) fn missing<'r>(
    record: &'r EntityRecord,
    field: &str,
    base: FileBase,
    input: &RuleInput<'_>,
) -> Vec<Missing<'r>> {
    let root = match base {
        FileBase::Spec => input.spec_root,
        FileBase::Project => input.project_root,
    };
    named(record, field)
        .into_iter()
        .filter(|(path, _)| !input.disk.exists(&root.join(path)))
        .map(|(path, span)| Missing {
            path,
            suggestion: sibling(path, root, input),
            span,
        })
        .collect()
}

/// The paths `record` names in `field`: its last occurrence's text when it is a scalar, or each
/// item when it is a list, never an empty one. A block, an expression group, a variant list and a
/// type union name none; a joined text is never split (ADR 0019). A member path names those of
/// each of its members.
fn named<'r>(record: &'r EntityRecord, field: &str) -> Vec<(&'r str, Option<SourceSpan>)> {
    let member = field.contains('.');
    let written: Vec<&FieldRecord> = if member {
        record
            .fields
            .iter()
            .flat_map(|f| &f.members)
            .filter(|m| m.declared == field)
            .collect()
    } else {
        record
            .fields
            .iter()
            .rev()
            .find(|f| f.key == field)
            .into_iter()
            .collect()
    };
    written
        .into_iter()
        .flat_map(|written| -> Vec<(&'r str, Option<SourceSpan>)> {
            let span = if member {
                written.value_span.clone()
            } else {
                None
            };
            let paths: Vec<&str> = match written.shape {
                ValueShape::String
                | ValueShape::Identifier
                | ValueShape::Date
                | ValueShape::Integer
                | ValueShape::Boolean => vec![written.text.as_str()],
                ValueShape::List => written.items.iter().flatten().map(String::as_str).collect(),
                ValueShape::Block
                | ValueShape::Blocks
                | ValueShape::Expression
                | ValueShape::Variants
                | ValueShape::TypeUnion => Vec::new(),
            };
            paths.into_iter().map(|p| (p, span.clone())).collect()
        })
        .filter(|(name, _)| !name.is_empty())
        .collect()
}

/// The paths `record` names in `field` (a declared path), as [`named`] reads them for every `file_exists` rule: what a
/// pass's or command's `field` read hands to the host (ADR 0293).
pub fn paths_named<'r>(record: &'r EntityRecord, field: &str) -> Vec<&'r str> {
    named(record, field)
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}

/// The close sibling of `path` in its directory, as the help a missing path carries.
fn sibling(path: &str, root: &Path, input: &RuleInput<'_>) -> Option<String> {
    let full_path = root.join(path);
    let parent = full_path.parent()?;
    let siblings = input.disk.list(parent)?;
    let file_name = full_path.file_name()?.to_str()?;

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
