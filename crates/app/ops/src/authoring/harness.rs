//! `specforge harness gc` (ADR 0287): write the harness ledger the way every host write would, dropping the lines of
//! entities and fields the project no longer holds and putting the rest in order. It never runs from `check` (I203
//! only says it would change something).

use crate::authoring::planned::PlannedChange;
use crate::view::ProjectView;
use crate::{Caller, OpError, Wrote};
use serde::Serialize;

/// What a `harness gc` did: the change it planned, and whether it wrote.
#[derive(Debug, Clone)]
pub struct Collected {
    pub change: PlannedChange,
    pub wrote: Wrote,
}

/// What `harness gc --format json` answers.
#[derive(Debug, Clone, Serialize)]
pub struct CollectedDocument {
    pub affected_files: Vec<String>,
    pub diff: String,
    pub dry_run: bool,
}

impl Collected {
    pub fn document(&self) -> CollectedDocument {
        CollectedDocument {
            affected_files: self.change.affected_files().map(String::from).collect(),
            diff: self.change.diff(),
            dry_run: self.wrote.previewed(),
        }
    }
}

/// Plan the ledger's canonical rewrite over `view`'s project; write it unless `dry_run`.
pub fn run(
    view: &ProjectView<'_>,
    dry_run: bool,
    caller: Caller<'_>,
) -> Result<Collected, OpError> {
    view.project_root()?;
    let change = PlannedChange::new(
        view.env().sources().clone(),
        "cannot rewrite specforge-harness",
        [],
    )
    .rewriting_ledger(view.project());
    let wrote = Wrote::unless_preview(dry_run, || change.apply(caller))?;
    Ok(Collected { change, wrote })
}
