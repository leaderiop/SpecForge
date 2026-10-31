//! The plans writing collectors answered (ADR 0289), applied after the collect wrote its report. ADR 0303: an
//! extension's error refuses no collected fact; a host error does.

use super::{Origin, UNDECLARED_WRITE, apply_plan};
use crate::view::ProjectView;
use crate::{Caller, CollectedPlan, EditDocument, OpError, OpErrorKind};

/// Apply each of `plans`, in collector order, over the project `view` read and checked again over the disk as the
/// collect left it, its report included (ADR 0297: an error the recorded results alone cause refuses no plan; a second
/// plan that names a file the first wrote is `stale_revision`), unless `dry_run`. A plan from a collector that does not declare `writes` is refused
/// (`undeclared_write`). Each document is under its runner's name. The first refusal ends it: the plans before it were
/// written.
pub fn apply_collected(
    view: &ProjectView<'_>,
    plans: Vec<CollectedPlan>,
    dry_run: bool,
    caller: Caller<'_>,
) -> Result<Vec<(String, EditDocument)>, OpError> {
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
