//! What a list of typed edits did (ADR 0162): [`run`] plans it and writes it unless it previews; [`EditDocument`]
//! is what both surfaces print.

use super::{Edit, EditDocument, EditField, EditReport, plan};
use crate::authoring::apply::{self, Origin};
use crate::view::ProjectView;
use crate::{Caller, Refusal, Wrote};

/// What a list of typed edits did: what it plans (or planned) and whether it wrote.
#[derive(Debug, Clone)]
pub struct Edited {
    pub report: EditReport,
    pub wrote: Wrote,
}

/// Plan `edits` over `view` ([`plan`]) and write them as one project change ([`apply::apply`], for `origin`) unless `dry_run`. An empty
/// plan is applied too: it stages no file and answers no writes. Refused as [`plan`] refuses, or with [`apply::apply`]'s
/// error (its writes: the files it could not put back).
pub fn run(
    view: &ProjectView<'_>,
    edits: &[Edit],
    dry_run: bool,
    origin: &Origin,
    caller: Caller<'_>,
) -> Result<Edited, Refusal<EditField>> {
    apply::check_edits(view, origin, edits)?;
    let planned = plan(view, edits)?;
    let wrote = Wrote::unless_preview(dry_run, || apply::apply(planned.change(), origin, caller))?;
    Ok(Edited {
        report: planned.report().clone(),
        wrote,
    })
}

impl Edited {
    /// The document both surfaces print.
    pub fn document(&self) -> EditDocument {
        EditDocument {
            report: self.report.clone(),
            dry_run: self.wrote.previewed(),
        }
    }
}
