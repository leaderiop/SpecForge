//! The compute pass: every `C` field of the model is listed in `computes`, as the kernel needs a compute pass for
//! each computed field it declares (W021 otherwise). It answers the values of execution
//! (`execute`: plan (`draft`, `checked`, `ready` from the plan-check verdicts, then execution) and task status, a summary's duration, actuals and key files, a deliverable check's status). A
//! check's status reads the test facts its `ref` names (`passes::thread`). `progress` answers the requirement, phase,
//! milestone, project, workstream, context and retrospective values; `requirement` a requirement's `proof`, its
//! thread's state (`passes::thread::requirement_proof`, read once per requirement), and its `satisfied_by`.
//!
//! `verify` answers a verification's status, score and re-verification, `uat` a session's totals and status and a
//! gap's resolution, `audit` a validation's coverage, a milestone audit and the windows and proposals, and `ladder` the
//! phase's `complete` and `needs_review`, `completed`, a requirement's `gaps_found` and `project.windows_*`. `estimate`
//! answers a plan's slices under the smart-zone budget. `review` answers a review concern's status, a round's open
//! counts and a revision conflict's status, `batch` a quick batch's status and eligible items, `learning` a
//! lesson's status, `decision` a decision's `in_force` (the superseded set E310 reads), `risk` a risk's rpn, post_rpn,
//! status and blocking (against `specforge.risk_block_on`, read once here), and `invariant` an invariant's status and
//! last_verified (from each phase's status on the ladder and its verification), and `product` a term's `used_in`.
//!
//! Not answered, by design: `phase.frontend` (`@gsd/ui`'s detection) and `retrospective.sessions` (no harness).

mod audit;
mod batch;
mod branch;
mod decision;
mod estimate;
mod execute;
mod halt;
pub mod invariant;
pub mod ladder;
mod learning;
mod product;
mod progress;
mod requirement;
mod review;
pub mod risk;
pub(crate) mod uat;
pub mod verify;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    let computes: Vec<String> = crate::kinds::computed()
        .into_iter()
        .filter(|field| !super::CLAIMED.contains(&field.as_str()))
        .collect();
    c.pass("derive", |p| {
        let names: Vec<&str> = computes.iter().map(String::as_str).collect();
        p.compute()
            .computes(&names)
            .baseline(&[
                "verification",
                "truth",
                "artifact",
                "key_link",
                "verify_gap",
                "override",
            ])
            .input(Input::facts(&[crate::passes::thread::TEST]))
            .run(derive);
    });
}

fn derive(input: &PassInput) -> PassOutput {
    let mut values: Vec<PassValue> = Vec::new();
    let commits = crate::passes::commits::commits(input);
    let statuses = execute::plan_statuses(input, &commits);
    values.extend(execute::values(input, &commits, &statuses));
    values.extend(verify::values(input, &statuses));
    let verified = verify::by_phase(input);
    values.extend(progress::values(input, &commits, &statuses, &verified));
    let phases = progress::phase_statuses(input, &verified);
    let invariants = invariant::statuses(input, &phases, &verified);
    values.extend(invariant::values(input, &invariants));
    values.extend(uat::values(input, &statuses));
    values.extend(learning::values(input, &verified));
    values.extend(batch::values(input, &statuses));
    values.extend(decision::values(input));
    let failing = verified
        .iter()
        .filter(|(_, v)| v.status == "gaps_found")
        .map(|(phase, _)| phase.as_str())
        .collect();
    let requirements = progress::requirement_states(input, &statuses, &failing);
    let proofs = requirement::proofs(input, &commits);
    values.extend(requirement::values(input, &proofs));
    values.extend(audit::values(input, &requirements, &verified));
    values.extend(branch::values(input));
    values.extend(estimate::values(input));
    values.extend(review::values(input));
    let block_on = Settings::current()
        .integer(crate::settings::RISK_BLOCK_ON)
        .unwrap_or(crate::settings::RISK_BLOCK_ON_DEFAULT);
    let (risks, findings) =
        risk::values_and_findings(input, &commits, &statuses, &invariants, block_on);
    values.extend(risks);
    values.extend(product::values(input));
    let mut diagnostics = branch::diagnostics(input);
    diagnostics.extend(findings);
    PassOutput {
        values,
        diagnostics,
        ..Default::default()
    }
}
