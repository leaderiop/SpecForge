//! The one place a planned change is written (ADR 0289): [`apply`] writes it under the project's write lock and names who
//! asked. A typed edit, a rename and `harness gc` write through it, and the surfaces and tests do not call
//! [`PlannedChange::apply`] themselves. An extension's edit plan is applied by [`apply_plan`]: lowered, checked
//! against the field flags and the revisions it names, planned with the typed edit engine, judged, and written all or
//! nothing with its harness ledger.

mod collector;
mod command;
mod flags;
pub(crate) mod journal;
mod lower;
mod revision;

pub use collector::apply_collected;
pub use command::{CommandRun, PlanOutcome, execute};

use crate::authoring::edit::{EditField, Edited, Step, plan_steps};
use crate::authoring::planned::PlannedChange;
use crate::view::ProjectView;
use crate::{Caller, OpError, OpErrorKind, Refusal, Writes, Wrote};
use specforge_project::CompiledProject;
use specforge_project::harness::LedgerChange;
use specforge_protocol_types::EditPlan;
use std::collections::BTreeSet;

/// A plan's operations are not well formed: the data names the operation's number (from 1).
pub const INVALID_PLAN: &str = "invalid_plan";
/// A plan from a command that does not declare `writes`.
pub const UNDECLARED_WRITE: &str = "undeclared_write";
/// A plan writes a computed value.
pub const COMPUTED_FIELD: &str = "computed_field";
/// A plan, or a typed edit, sets a harness value its origin may not set.
pub const HARNESS_FIELD: &str = "harness_field";
/// A plan names a revision of a file that is not the file's now.
pub const STALE_REVISION: &str = "stale_revision";

/// One operation of a lowered plan: its number in the plan, from 1.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Op {
    pub number: usize,
    pub step: Step,
}

/// `message`, about operation `number` of a plan.
pub(super) fn invalid(number: usize, message: impl std::fmt::Display) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        INVALID_PLAN,
        format!("operation {number}: {message}"),
    )
    .with_data(serde_json::json!({"op": number}))
}

/// The surface a core operation was asked on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Mcp,
    /// `interactive`: stdin and stdout are a terminal, so a person is at the keyboard.
    Cli {
        interactive: bool,
    },
    Lsp,
}

/// Who asked for a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A command of `extension` answered the plan.
    Command { extension: String },
    /// A collector of `extension` answered the plan.
    Collector { extension: String },
    /// A hook of `extension` answered the plan.
    Hook { extension: String },
    /// The host's own operation, asked on `surface`.
    Core { surface: Surface },
}

impl Origin {
    /// A core operation asked on the command line.
    pub fn cli(interactive: bool) -> Self {
        Origin::Core {
            surface: Surface::Cli { interactive },
        }
    }

    /// A core operation asked over MCP.
    pub fn mcp() -> Self {
        Origin::Core {
            surface: Surface::Mcp,
        }
    }
}

/// Apply `plan`, an extension's, over the project `view` reads, for `origin`, unless `dry_run` (ADR 0289). In order: the
/// plan is lowered (`invalid_plan`); its operations are checked against the field flags (`computed_field`,
/// `harness_field`); the revisions it names are compared with the compile (`stale_revision`); it is planned with the
/// typed edit engine, renames included, and judged (E080); then, under the write lock, the revisions are compared
/// with the disk (`stale_revision`), and, when a source other than the plan's has changed since the compile, the
/// plan is judged again over the disk as it is now (E080). Nothing is written unless every file can be.
pub fn apply_plan(
    view: &ProjectView<'_>,
    plan: &EditPlan,
    origin: &Origin,
    dry_run: bool,
    caller: Caller<'_>,
) -> Result<Edited, Refusal<EditField>> {
    let lowered = lower::lower(view, plan)?;
    flags::check(view, origin, &lowered.ops, true)?;
    if let Some(number) = lowered.verdicts.first() {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            NOT_JUDGED,
            format!("operation {number}: set_verdict names a rule, and no extension declares a judged rule"),
        )
        .with_data(serde_json::json!({"op": number}))
        .into());
    }
    revision::against_compile(view.project(), &lowered.files)?;
    let steps: Vec<Step> = lowered.ops.iter().map(|op| op.step.clone()).collect();
    let planned = plan_steps(view, &steps)?;
    let report = planned.report().clone();
    let wrote = Wrote::unless_preview(dry_run, || {
        let change = planned.change().clone().serialized();
        let project = view.project();
        let files = lowered.files.clone();
        let sources = view.env().sources().clone();
        change.apply_checked(caller, || {
            revision::against_disk(&sources, caller, &files)?;
            rejudge(project, &change, &files)
        })
    })?;
    Ok(Edited { report, wrote })
}

/// Check typed `edits` of a core surface against the field flags, for `origin`: a computed value is never written, and
/// a harness value only by a command or collector of its extension.
pub(crate) fn check_edits(
    view: &ProjectView<'_>,
    origin: &Origin,
    edits: &[crate::authoring::edit::Edit],
) -> Result<(), OpError> {
    let ops: Vec<Op> = edits
        .iter()
        .enumerate()
        .map(|(n, edit)| Op {
            number: n + 1,
            step: Step::Edit(edit.clone()),
        })
        .collect();
    flags::check(view, origin, &ops, false)
}

/// A rule named by `set_verdict` is not judged.
pub const NOT_JUDGED: &str = "not_judged";

/// When a source other than the plan's has changed since `project` was compiled, `change` judged over the disk as
/// it is now: E080 when it adds an error the disk does not have.
fn rejudge(
    project: &CompiledProject,
    change: &PlannedChange,
    files: &std::collections::BTreeMap<String, String>,
) -> Result<(), OpError> {
    let own: BTreeSet<String> = files.keys().cloned().collect();
    if !project.sources_moved(&own) {
        return Ok(());
    }
    let before = project.edited_over_disk(&Default::default(), None);
    let ledger = change.ledger_over(&project.root_file(specforge_project::harness::LEDGER_FILE));
    let after = project.edited_over_disk(&change.after(), ledger.as_deref());
    crate::authoring::edit::validate::refuse_new_errors(
        &before.checked_diagnostics(),
        after.diagnostics(),
        project.environment().registries().codes(),
    )
    .map_err(|refusal| refusal.error)
}

/// Write `change` as one project change (ADR 0128) under the write lock, each file resolved through the caller's
/// containment (ADR 0104): checked against the text it was planned on, then replaced or created; on any failure,
/// every file already written is put back. Returns the files it wrote.
pub fn apply(
    change: &PlannedChange,
    _origin: &Origin,
    caller: Caller<'_>,
) -> Result<Writes, OpError> {
    change.apply(caller)
}

/// Write the ledger change of a rename the editor applied to its buffers and has saved (ADR 0289): the ledger as the
/// rename leaves it, `after` being the project the buffers show, under the write lock, over what the ledger holds
/// now. The renamed files are the editor's to save, so this writes nothing else.
pub fn land_ledger(
    before: &CompiledProject,
    after: &CompiledProject,
    change: LedgerChange,
    origin: &Origin,
    caller: Caller<'_>,
) -> Result<Writes, OpError> {
    let planned = PlannedChange::new(
        after.environment().sources().clone(),
        "cannot write the ledger of a rename",
        [],
    )
    .landing_ledger(before, after, change);
    apply(&planned, origin, caller)
}

/// What tests write through (behind the `testing` feature): a core operation on the command line, nobody at a terminal.
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::{Origin, apply as write};
    use crate::authoring::planned::PlannedChange;
    use crate::{Caller, OpError, Writes};

    /// The origin most tests write as.
    pub fn origin() -> Origin {
        Origin::cli(false)
    }

    /// Write `change` as [`origin`]: the one way a test applies a planned change.
    pub fn apply(change: &PlannedChange, caller: Caller<'_>) -> Result<Writes, OpError> {
        write(change, &origin(), caller)
    }
}
