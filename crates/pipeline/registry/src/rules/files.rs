//! The paths an entity's field names, each resolved in its root through `Contained` (ADR 0335): a path that leaves
//! its root, names an unknown root, differs in case from the disk or uses backslashes, and the contained close
//! sibling of a missing one (ADR 0118). Every `file_exists` rule, E016 among them, reads paths this way.

use specforge_common::SourceSpan;
use specforge_effects::RootName;
use specforge_effects::contained::{Contained, Refused};
use specforge_graph::ValueShape;

use specforge_protocol_types::FileBase;

use crate::disk::Disk;
use crate::entity::{EntityRecord, FieldRecord};

/// What is wrong with one path a field names.
pub(super) enum Problem {
    /// It does not exist (the rule's own code, E016 for a file reference), with a contained close sibling.
    Missing { suggestion: Option<String> },
    /// It leaves its root (E094), and why.
    Leaves(&'static str),
    /// Its `<name>/` prefix names no root (E095).
    UnknownRoot(String),
    /// It exists under another case (W174): the disk's spelling.
    Case(String),
    /// It uses backslashes (W175), read as `/`.
    Backslashes,
}

/// One problem with one path a field names.
pub(super) struct Finding<'r> {
    /// As the entity wrote it: the scalar's text, or the list item.
    pub path: &'r str,
    pub problem: Problem,
    /// Where a block member writes the path; `None` for a field, which the entity's span stands for.
    pub span: Option<SourceSpan>,
}

/// The problems of the paths `record` names in `field`, in order, each resolved by `contained` in `base`'s root (the
/// spec root, or the project root where the field declares that base, ADR 0285) unless its `<root>/` prefix names
/// another. A path is contained in the root it names: `..` above it, an absolute path and a link out are E094. `field`
/// is a declared path: a member (`paths.file`) is read in each item of its block.
pub(super) fn checked<'r>(
    record: &'r EntityRecord,
    field: &str,
    base: FileBase,
    contained: &Contained<'_>,
    disk: &dyn Disk,
) -> Vec<Finding<'r>> {
    let default = match base {
        FileBase::Spec => RootName::Spec,
        FileBase::Project => RootName::Project,
    };
    let mut findings = Vec::new();
    for (path, span) in named(record, field) {
        let mut found = |problem| {
            findings.push(Finding {
                path,
                problem,
                span: span.clone(),
            })
        };
        match contained.resolve(path, default.clone()) {
            Err(Refused::UnknownRoot(name)) => found(Problem::UnknownRoot(name)),
            Err(Refused::Absolute) => found(Problem::Leaves("it is absolute")),
            Err(Refused::Climbs) => found(Problem::Leaves("its `..` climbs above its root")),
            Err(Refused::LinksOut { .. }) => found(Problem::Leaves("a link leads out of its root")),
            Err(Refused::Empty) => found(Problem::Missing { suggestion: None }),
            Ok(resolved) => {
                // Read as written too: a session records the path itself, so a link on the way pointed elsewhere,
                // which no listing shows, re-runs the checks.
                disk.exists(&resolved.at);
                if resolved.backslashes {
                    found(Problem::Backslashes);
                }
                match (&resolved.case, resolved.exists) {
                    (Some(disk), true) => found(Problem::Case(disk.clone())),
                    (None, true) => {}
                    (_, false) => found(Problem::Missing {
                        suggestion: contained.sibling(&resolved),
                    }),
                }
            }
        }
    }
    findings
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
