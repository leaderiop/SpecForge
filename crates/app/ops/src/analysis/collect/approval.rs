//! Whether each collector's command may run (ADR 0335 D2, ADR 0125 D5): a collector's command is its extension's
//! process capability, so it runs only through that extension's grant. Every grant is decided and settled for every
//! selected collector before the first command runs: the approvals the runs need are asked for once, then each
//! command is held to its grant. A command its grant refuses is E090 (the project's effects ceiling, or a command
//! its capability does not declare); one nobody approved is E059.

use super::evidence::Source;
use super::{Collector, fail};
use crate::analysis::grants::grants_of;
use crate::asking::Asking;
use crate::settle::{Unsettled, settle};
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};
use specforge_common::codes;
use specforge_common::remedy::printable;
use specforge_effects::{Consent, Denied, EffectRequest, Grant, Need, ProcessRequest, Uses};
use specforge_protocol_types::Operation;

/// Settle every approval the commands of `planned` need, asking `asking` and storing in `consent`, then hold each
/// command to its extension's grant. Approvals given but not stored are added to `unsaved`.
pub(super) fn settle_runs(
    view: &ProjectView,
    planned: &[(&Collector, Source<'_>)],
    asking: Asking<'_>,
    consent: &dyn Consent,
    unsaved: &mut Vec<String>,
) -> Result<(), OpError> {
    let mut granting: Vec<(Grant, Uses)> = grants_of(view, consent)?
        .into_iter()
        .map(|grant| (grant, Uses::new()))
        .collect();
    for (collector, source) in planned {
        let (Source::Run { .. }, Some(scope)) = (source, &collector.capability) else {
            continue;
        };
        if let Some((_, uses)) = granting
            .iter_mut()
            .find(|(grant, _)| grant.extension() == collector.extension)
        {
            uses.insert((Operation::Collect, scope.clone()));
        }
    }
    granting.retain(|(_, uses)| !uses.is_empty());
    for (grant, uses) in &mut granting {
        match settle(grant, uses, asking, consent) {
            Ok(settled) => unsaved.extend(settled.unsaved),
            Err(unsettled) => {
                return Err(unapproved(planned, grant.extension(), consent, &unsettled));
            }
        }
    }
    for (collector, source) in planned {
        let Source::Run { argv, .. } = source else {
            continue;
        };
        let Some((grant, _)) = granting
            .iter()
            .find(|(grant, _)| grant.extension() == collector.extension)
        else {
            continue;
        };
        let request = EffectRequest::Process(ProcessRequest {
            argv: argv.clone(),
            ..ProcessRequest::default()
        });
        if let Err(denial) = grant.permits(Operation::Collect, &request) {
            return Err(match denial.why {
                Denied::NotApproved | Denied::Root => {
                    e059(argv, &collector.extension, &consent.store(), None)
                }
                _ => OpError::of(OpErrorKind::PermissionDenied, denial.diagnostic()),
            });
        }
    }
    Ok(())
}

/// E059 for the first command of `extension` whose scope `unsettled` lists.
fn unapproved(
    planned: &[(&Collector, Source<'_>)],
    extension: &str,
    consent: &dyn Consent,
    unsettled: &Unsettled,
) -> OpError {
    let scopes: Vec<&str> = unsettled
        .needs
        .iter()
        .filter_map(|need| match need {
            Need::Capability { approval, .. } => Some(approval.scope.as_str()),
            Need::Root(_) => None,
        })
        .collect();
    let argv = planned
        .iter()
        .find_map(|(collector, source)| match source {
            Source::Run { argv, .. }
                if collector.extension == extension
                    && collector
                        .capability
                        .as_deref()
                        .is_some_and(|scope| scopes.contains(&scope)) =>
            {
                Some(argv.as_slice())
            }
            _ => None,
        })
        .unwrap_or_default();
    e059(
        argv,
        extension,
        &consent.store(),
        unsettled.store.as_ref().map(|d| d.message.as_str()),
    )
}

/// E059: running `argv` needs this machine's approval; `store` says why the consent store could not be used.
fn e059(argv: &[String], extension: &str, kept: &str, store: Option<&str>) -> OpError {
    let mut message = format!(
        "running `{}` needs approval for this project",
        argv.join(" ")
    );
    if let Some(store) = store {
        message.push_str(&format!(" ({store})"));
    }
    let kept = printable(kept);
    fail(
        OpErrorKind::PermissionDenied,
        codes::E059,
        printable(&message).into_owned(),
    )
    .with_suggestion(format!(
        "approve it with `{}` (kept in {kept}), once with `specforge collect` in a terminal, \
         or run `specforge collect --yes`; reading the report the runner already wrote needs no approval",
        specforge_common::remedy::command("specforge grants approve {}", &[extension])
    ))
}
