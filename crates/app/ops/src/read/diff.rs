//! What changed between two snapshots of the project (ADR 0376 D5): each node added, removed or modified, by kind and
//! id with the digests of its content, and each edge added or removed. `specforge diff` renders it. A revision is
//! resolved by the git port in process; the working tree is the project as compiled.

use std::collections::BTreeMap;

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_project::snapshot::{
    AtError, Comparison, NoteAbout, Side, Snapshot, SnapshotKey, compare,
};
use specforge_protocol_types::{DeltaAspect, DeltaBase, DeltaChange, DeltaPayload};

use crate::view::ProjectView;
use crate::{GIT_FAILED, NOT_A_REPOSITORY, OpError, OpErrorKind, Refusal, UNKNOWN_REVISION};

/// A range of the symmetric form `<a>...<b>`, which names a merge-base and hides which side changed.
pub const SYMMETRIC_RANGE: &str = "symmetric_range";
/// A side of `<a>..<b>` names no revision.
pub const EMPTY_REVISION: &str = "empty_revision";

/// What the working tree is called in an answer.
const WORKING_TREE: &str = "working tree";

/// The two snapshots a diff compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRange {
    /// No range: `HEAD` against the working tree.
    HeadToWorkingTree,
    /// `<rev>`: that revision against the working tree.
    ToWorkingTree(String),
    /// `<a>..<b>`: two revisions.
    Between(String, String),
}

impl DiffRange {
    /// The range `text` names, the one parser of both surfaces: none is `HEAD` against the working tree, `<rev>` that
    /// revision against it, `<a>..<b>` two revisions. `<a>...<b>` is refused ([`SYMMETRIC_RANGE`]), and so is a side
    /// of `..` that names nothing ([`EMPTY_REVISION`]).
    pub fn parse(text: Option<&str>) -> Result<DiffRange, OpError> {
        let Some(text) = text else {
            return Ok(DiffRange::HeadToWorkingTree);
        };
        if text.contains("...") {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                SYMMETRIC_RANGE,
                format!("'{text}' is a symmetric range: compare two revisions as <a>..<b>"),
            ));
        }
        match text.split_once("..") {
            None if text.is_empty() => Err(empty(text)),
            None => Ok(DiffRange::ToWorkingTree(text.to_string())),
            Some((from, to)) if from.is_empty() || to.is_empty() => Err(empty(text)),
            Some((from, to)) => Ok(DiffRange::Between(from.to_string(), to.to_string())),
        }
    }
}

/// The refusal of `text`, a range with a side that names no revision.
fn empty(text: &str) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        EMPTY_REVISION,
        format!("'{text}' names no revision on one side: write <rev> or <a>..<b>"),
    )
}

/// A diff: the range, as written, and the kinds it keeps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffRequest {
    /// `<rev>` or `<a>..<b>`; none is `HEAD` against the working tree.
    pub range: Option<String>,
    /// The kinds kept: their nodes and the edges with an end of one, as a pass reading the delta is sent. None keeps
    /// every change.
    pub kinds: Vec<String>,
}

/// The field of a diff request a refusal can be about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffField {
    Range,
}

/// What changed between two snapshots (`McpDiffResult`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct DiffAnswer {
    pub from: DiffSide,
    pub to: DiffSide,
    /// Each changed node, sorted by kind then id.
    pub nodes: Vec<DiffNode>,
    /// Each changed edge, the added then the removed, each sorted by source, target then label.
    pub edges: Vec<DiffEdge>,
    /// What one side could not compare: a `specforge.json` loading another extension set (both sides are compiled
    /// with the current one), a collector whose facts it lacks, left out of both sides.
    pub notes: Vec<DiffNote>,
    /// How many of `nodes` and `edges` each change counts.
    pub summary: DiffSummary,
}

/// The changes a diff shows, counted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Shape)]
pub struct DiffSummary {
    pub nodes_added: usize,
    pub nodes_removed: usize,
    pub nodes_modified: usize,
    pub edges_added: usize,
    pub edges_removed: usize,
    /// The node counts of each kind that changed, sorted by kind as bytes.
    pub by_kind: Vec<KindCount>,
}

/// The node changes of one kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Shape)]
pub struct KindCount {
    pub kind: String,
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
}

impl DiffSummary {
    /// The counts of `payload`'s changes.
    fn of(payload: &DeltaPayload) -> DiffSummary {
        let mut by_kind: BTreeMap<&str, KindCount> = BTreeMap::new();
        let mut summary = DiffSummary {
            edges_added: payload.added_edges.len(),
            edges_removed: payload.removed_edges.len(),
            ..DiffSummary::default()
        };
        for node in &payload.nodes {
            let count = by_kind.entry(&node.kind).or_insert_with(|| KindCount {
                kind: node.kind.clone(),
                ..KindCount::default()
            });
            let (total, of_kind) = match node.change {
                DeltaChange::Added => (&mut summary.nodes_added, &mut count.added),
                DeltaChange::Removed => (&mut summary.nodes_removed, &mut count.removed),
                DeltaChange::Modified => (&mut summary.nodes_modified, &mut count.modified),
            };
            *total += 1;
            *of_kind += 1;
        }
        summary.by_kind = by_kind.into_values().collect();
        summary
    }
}

/// One side of a diff: a named shape, since an answer holds two.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
#[shape(define)]
pub struct DiffSide {
    /// The revision as written, or `working tree`.
    pub name: String,
    /// The commit's sha, or the key the working tree's facts are filed by.
    pub key: String,
}

/// One changed node.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct DiffNode {
    pub kind: String,
    pub id: String,
    /// `added`, `removed` or `modified`.
    pub change: String,
    /// A modified node's changed fields.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// What else about a modified node changed: `title`, `methods`, `obligations`, `verdicts`, `edges`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aspects: Vec<String>,
    /// A modified node's content digest on the `from` side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// An added or modified node's content digest on the `to` side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// One changed edge.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct DiffEdge {
    pub source: String,
    pub target: String,
    pub label: String,
    /// `added` or `removed`.
    pub change: String,
}

/// What one side of a diff could not compare.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct DiffNote {
    /// `from` or `to`.
    pub side: String,
    /// `extensions_differ`, `config_unreadable`, `facts_not_collected` or `facts_unrecorded`.
    pub about: String,
    /// What it is about: the extensions that differ, comma-separated, `specforge.json`, or a collector's name.
    pub subject: String,
    pub message: String,
}

/// What changed across `request`'s range, the revisions read through the view's git port and its containment's disk.
/// An unknown revision is `unknown_revision`, a project in no repository `not_a_repository`, a git that cannot
/// answer `git_failed`.
pub fn diff(
    view: &ProjectView<'_>,
    request: &DiffRequest,
) -> Result<DiffAnswer, Refusal<DiffField>> {
    let range = DiffRange::parse(request.range.as_deref())
        .map_err(|error| Refusal::about(DiffField::Range, error))?;
    let disk = view.containment().disk();
    let at = |revision: &str| {
        Snapshot::at_revision(view.env(), &disk, revision)
            .map_err(|error| Refusal::about(DiffField::Range, refused(view, error)))
    };
    let working = || Snapshot::of_project(view.project());
    let (from, to) = match &range {
        DiffRange::HeadToWorkingTree => (at("HEAD")?, working()),
        DiffRange::ToWorkingTree(revision) => (at(revision)?, working()),
        DiffRange::Between(from, to) => (at(from)?, at(to)?),
    };
    let label = match &from.key {
        SnapshotKey::Revision { sha, .. } => DeltaBase::Revision {
            id: sha.as_str().to_string(),
        },
        SnapshotKey::WorkingTree { .. } => DeltaBase::None,
    };
    let Comparison { delta, notes } = compare(&from, &to, view.registries().field_forms(), label);
    let payload = if request.kinds.is_empty() {
        delta.payload().clone()
    } else {
        delta.restricted_to(&request.kinds)
    };
    let edges = |edges: &[specforge_protocol_types::DeltaEdge], change: &str| {
        edges
            .iter()
            .map(|e| DiffEdge {
                source: e.source.clone(),
                target: e.target.clone(),
                label: e.label.clone(),
                change: change.to_string(),
            })
            .collect::<Vec<_>>()
    };
    let mut changed = edges(&payload.added_edges, "added");
    changed.extend(edges(&payload.removed_edges, "removed"));
    Ok(DiffAnswer {
        from: side(&from),
        to: side(&to),
        nodes: payload
            .nodes
            .iter()
            .map(|n| DiffNode {
                kind: n.kind.clone(),
                id: n.id.clone(),
                change: change_name(n.change).to_string(),
                fields: n.fields.clone(),
                aspects: n
                    .aspects
                    .iter()
                    .map(|a| aspect_name(*a).to_string())
                    .collect(),
                before: n.before.clone(),
                after: n.after.clone(),
            })
            .collect(),
        summary: DiffSummary::of(&payload),
        edges: changed,
        notes: notes
            .into_iter()
            .map(|note| DiffNote {
                side: match note.side {
                    Side::From => "from",
                    Side::To => "to",
                }
                .to_string(),
                about: match note.about {
                    NoteAbout::ExtensionsDiffer => "extensions_differ",
                    NoteAbout::ConfigUnreadable => "config_unreadable",
                    NoteAbout::FactsNotCollected => "facts_not_collected",
                    NoteAbout::FactsUnrecorded => "facts_unrecorded",
                }
                .to_string(),
                subject: note.subject,
                message: note.message,
            })
            .collect(),
    })
}

/// How `snapshot` is named in an answer.
fn side(snapshot: &Snapshot) -> DiffSide {
    match &snapshot.key {
        SnapshotKey::Revision { name, sha } => DiffSide {
            name: name.clone(),
            key: sha.as_str().to_string(),
        },
        SnapshotKey::WorkingTree { key } => DiffSide {
            name: WORKING_TREE.to_string(),
            key: key.clone(),
        },
    }
}

/// The wire's name of a change.
fn change_name(change: DeltaChange) -> &'static str {
    match change {
        DeltaChange::Added => "added",
        DeltaChange::Removed => "removed",
        DeltaChange::Modified => "modified",
    }
}

/// The wire's name of an aspect.
fn aspect_name(aspect: DeltaAspect) -> &'static str {
    match aspect {
        DeltaAspect::Title => "title",
        DeltaAspect::Methods => "methods",
        DeltaAspect::Obligations => "obligations",
        DeltaAspect::Verdicts => "verdicts",
        DeltaAspect::Edges => "edges",
    }
}

/// Why no snapshot was taken at a revision, as the read refuses it.
fn refused(view: &ProjectView<'_>, error: AtError) -> OpError {
    match error {
        AtError::UnknownRevision(revision) => OpError::new(
            OpErrorKind::InvalidInput,
            UNKNOWN_REVISION,
            format!("unknown revision '{revision}'"),
        ),
        AtError::NotARepository => OpError::new(
            OpErrorKind::PreconditionFailed,
            NOT_A_REPOSITORY,
            format!("{} is in no git repository", view.env().root().display()),
        ),
        AtError::Unreadable(why) => OpError::new(OpErrorKind::Internal, GIT_FAILED, why),
    }
}
