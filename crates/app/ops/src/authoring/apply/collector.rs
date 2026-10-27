//! The plans writing collectors answered (ADR 0289), applied after the collect wrote its report.

use super::{Origin, UNDECLARED_WRITE, apply_plan};
use crate::view::ProjectView;
use crate::{Caller, CollectedPlan, EditDocument, OpError, OpErrorKind};

/// Apply each of `plans`, in collector order, over the project `view` read (a second plan that names a file the first
/// wrote is `stale_revision`), unless `dry_run`. A plan from a collector that does not declare `writes` is refused
/// (`undeclared_write`). Each document is under its runner's name. The first refusal ends it: the plans before it were
/// written.
pub fn apply_collected(
    view: &ProjectView<'_>,
    plans: Vec<CollectedPlan>,
    dry_run: bool,
    caller: Caller<'_>,
) -> Result<Vec<(String, EditDocument)>, OpError> {
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
