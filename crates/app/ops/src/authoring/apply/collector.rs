//! The facts collectors answered (ADR 0327) and the plans writing collectors answered (ADR 0289), written after the
//! collect wrote its report: the facts first, so a refused plan loses none. ADR 0303: an extension's error refuses no
//! collected fact; a host error does.

use specforge_project::facts::Store;
use specforge_project::harness::{LedgerChange, Line};

use super::{Origin, UNDECLARED_WRITE, apply_plan};
use crate::authoring::planned::PlannedChange;
use crate::view::ProjectView;
use crate::{Caller, CollectedFacts, CollectedPlan, EditDocument, OpError, OpErrorKind};

/// Store `facts` (nothing on a `dry_run`), then apply each of `plans`, in collector order, over the project `view` read and checked again over the disk as the
/// collect left it, its report included (ADR 0297: an error the recorded results alone cause refuses no plan; a second
/// plan that names a file the first wrote is `stale_revision`), unless `dry_run`. A plan from a collector that does not declare `writes` is refused
/// (`undeclared_write`). Each document is under its runner's name. The first refusal ends it: the plans before it were
/// written.
pub fn apply_collected(
    view: &ProjectView<'_>,
    plans: Vec<CollectedPlan>,
    facts: CollectedFacts,
    dry_run: bool,
    caller: Caller<'_>,
) -> Result<Vec<(String, EditDocument)>, OpError> {
    if !dry_run {
        store(view, facts, caller)?;
    }
    // shortcut: one re-check per collect that answered a plan; pass the post-report project from `collect` if it shows.
    let reread = (!plans.is_empty()).then(|| {
        view.project()
            .edited_over_disk(&std::collections::BTreeMap::new(), None)
    });
    // The re-read keeps the call's containment (ADR 0183): MCP's root bounds what it reads.
    let fresh = reread
        .as_ref()
        .map(|project| ProjectView::of(project).bounded_by(view.containment()));
    let view = fresh.as_ref().unwrap_or(view);
    let mut applied = Vec::new();
    for CollectedPlan {
        runner,
        extension,
        writes,
        plan,
    } in plans
    {
        if !writes {
            return Err(OpError::new(
                OpErrorKind::PermissionDenied,
                UNDECLARED_WRITE,
                format!(
                    "collector '{runner}' answered a plan but does not declare `writes`; nothing was written"
                ),
            ));
        }
        let edited = apply_plan(
            view,
            &plan,
            &Origin::Collector { extension },
            dry_run,
            caller,
        )
        .map_err(|refusal| refusal.into_error())?;
        applied.push((runner, edited.document()));
    }
    Ok(applied)
}

/// Store each collector's facts under their key, replacing its stores under any other key and its `facts:` line, in
/// one change under the write lock (ADR 0327 D2, D4). The host writes it: no extension's plan, so no extension is its
/// origin. Nothing when none is stored.
fn store(view: &ProjectView<'_>, facts: CollectedFacts, caller: Caller<'_>) -> Result<(), OpError> {
    let CollectedFacts { key, stores } = facts;
    if stores.is_empty() {
        return Ok(());
    }
    let (mut files, mut lines) = (Vec::new(), Vec::new());
    for (collector, extension, facts) in stores {
        let bytes = Store::new(&collector, &extension, &key, facts).bytes();
        lines.push(Line::facts(&collector, &key, &bytes));
        files.push((collector, bytes));
    }
    PlannedChange::new(
        view.env().sources().clone(),
        "cannot store the collected facts",
        [],
    )
    .with_fact_stores(&key, files)
    .keeping_ledger(
        view.project(),
        LedgerChange {
            facts: lines,
            ..Default::default()
        },
    )
    .apply(caller)?;
    Ok(())
}
