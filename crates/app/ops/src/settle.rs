//! Settling a run's approvals (ADR 0335 D2.4, ADR 0125 D5): every approval a run needs is decided before its first
//! effect. `Yes` approves them for this run and stores nothing; an asker is asked once per scope or root and a yes
//! is stored; nobody to ask (MCP, `--format json`) means stored approvals only.

use specforge_common::Diagnostic;
use specforge_effects::{Approval, ApprovedAt, Consent, Grant, Need, Uses};
use specforge_protocol_types::PortScope;

use crate::asking::{Asking, Question};
use crate::consent::unusable_store;

/// What settling decided: the run may go on, and the grant permits what it approved.
#[derive(Debug, Default)]
pub struct Settled {
    /// Approved for this run only, never stored: `Yes`, or a yes the store could not keep.
    pub for_this_run: Vec<Need>,
    /// Asked, approved and stored.
    pub stored: Vec<Need>,
    /// Why a yes was not stored (the run still goes on).
    pub unsaved: Vec<String>,
}

/// The approvals nobody gave: nothing ran.
#[derive(Debug)]
pub struct Unsettled {
    pub needs: Vec<Need>,
    /// W186 when the store could not be used: nothing in it counted, and nothing was asked that it would lose.
    pub store: Option<Box<Diagnostic>>,
}

/// Settle the approvals of `grant` that `uses` needs: ask `asking`, store each yes in `consent`, and mark each
/// approval given approved in `grant`.
pub fn settle(
    grant: &mut Grant,
    uses: &Uses,
    asking: Asking<'_>,
    consent: &dyn Consent,
) -> Result<Settled, Unsettled> {
    let needs = grant.needs(uses);
    let mut settled = Settled::default();
    if needs.is_empty() {
        return Ok(settled);
    }
    if let Asking::Yes = asking {
        for need in &needs {
            grant.allow(need);
        }
        settled.for_this_run = needs;
        return Ok(settled);
    }
    // An answer an unusable store would lose is not asked for.
    if let Some(error) = grant.store_problem() {
        return Err(Unsettled {
            needs,
            store: Some(Box::new(unusable_store(error))),
        });
    }
    let mut refused = Vec::new();
    for need in needs {
        let question = match &need {
            Need::Capability {
                approval,
                why,
                summary,
            } => Question::Capability {
                extension: &approval.extension,
                scope: &approval.scope,
                summary,
                why,
                project: &approval.project,
                command: match &approval.granted {
                    PortScope::Process(process) => Some(&process.argv),
                    _ => None,
                },
            },
            Need::Root(root) => Question::Root {
                name: &root.root,
                target: &root.target,
                project: &root.project,
            },
        };
        if !asking.answers(&question) {
            refused.push(need);
            continue;
        }
        grant.allow(&need);
        match consent.approve(stamped(need.approval())) {
            Ok(()) => settled.stored.push(need),
            Err(error) => {
                settled
                    .unsaved
                    .push(specforge_common::remedy::printable(&error.to_string()).into_owned());
                settled.for_this_run.push(need);
            }
        }
    }
    if refused.is_empty() {
        Ok(settled)
    } else {
        Err(Unsettled {
            needs: refused,
            store: None,
        })
    }
}

/// `approval`, dated now.
pub(crate) fn stamped(approval: Approval) -> Approval {
    let now = ApprovedAt(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    match approval {
        Approval::Capability(mut a) => {
            a.approved_at = now;
            Approval::Capability(a)
        }
        Approval::Root(mut a) => {
            a.approved_at = now;
            Approval::Root(a)
        }
    }
}

#[cfg(test)]
mod tests;
