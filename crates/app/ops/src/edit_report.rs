//! What an edit plan will write or wrote, in the document both surfaces print (ADR 0162, ADR 0289): a base type, so a
//! group that applies no plan (collect) can carry the result of one.

use serde::Serialize;
use specforge_common::DiagnosticList;
use specforge_common::shape::Shape;
use specforge_protocol_types::EditPlan;

/// What a plan will write.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct EditReport {
    /// The entities whose block the edits, taken together, create, change or delete, sorted
    /// (`PlannedChange::entities`).
    pub entities: Vec<String>,
    /// The files the edits change, relative to the project root, sorted.
    pub affected_files: Vec<String>,
    /// A unified diff of every affected file (project-root-relative headers), in file order. Empty when nothing changes.
    pub diff: String,
    /// The warnings and infos the edited project reports that the project did not, in report order.
    pub added: DiagnosticList,
}

/// What `specforge set|unset|link|unlink|delete|new --format json` and `specforge.edit` print (`McpEditResult`):
/// the report, and whether the run previewed (ADR 0052 D1, ADR 0162).
#[derive(Debug, Clone, Serialize, Shape)]
pub struct EditDocument {
    #[serde(flatten)]
    pub report: EditReport,
    /// The edits were only planned: nothing was written.
    pub dry_run: bool,
}

/// A plan a collector answered, not yet applied: the host applies it after the report is written (ADR 0289).
#[derive(Debug, Clone)]
pub struct CollectedPlan {
    /// The collector's name.
    pub runner: String,
    /// The extension that declares it.
    pub extension: String,
    /// Whether the collector declares `writes`.
    pub writes: bool,
    pub plan: EditPlan,
}
