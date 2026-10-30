//! The compute pass: every `C` field of the model is listed in `computes`, as the kernel needs a compute pass for
//! each computed field it declares (W021 otherwise). It answers `context.digest` and the values of execution
//! (`execute`: plan (`draft`, `checked`, `ready` from the plan-check verdicts, then execution) and task status, a summary's duration, actuals and key files, a deliverable check's status). It
//! declares `reads_tests`, to read the report for the checks only, so a compile without `specforge-report.json` says
//! I205 once. `progress` answers the requirement, phase, milestone, project, workstream, context and retrospective values.
//!
//! `verify` answers a verification's status, score and re-verification, `uat` a session's totals and status and a
//! gap's resolution, `audit` a validation's coverage, a milestone audit and the windows and proposals, and `ladder` the
//! phase's `complete` and `needs_review`, `completed`, a requirement's `gaps_found` and `project.windows_*`.
//!
//! Not answered, by design: `phase.frontend` (`@gsd/ui`'s detection) and `retrospective.sessions` (no harness).

mod audit;
mod execute;
mod halt;
pub mod ladder;
mod progress;
mod uat;
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
            .reads_tests()
            .baseline(&[
                "verification",
                "truth",
                "artifact",
                "key_link",
                "verify_gap",
            ])
            .run(derive);
    });
}

fn derive(input: &PassInput) -> PassOutput {
    let mut values: Vec<PassValue> = input
        .entities
        .iter()
        .filter(|e| e.kind == "context")
        .filter_map(|e| {
            Some(PassValue {
                entity: e.id.clone(),
                field: "digest".into(),
                value: crate::digest::context(input, &e.id)?.into(),
            })
        })
        .collect();
    let commits = crate::passes::commits::commits(input);
    let statuses = execute::plan_statuses(input, &commits);
    values.extend(execute::values(input, &commits, &statuses));
    values.extend(verify::values(input, &statuses));
    let verified = verify::by_phase(input);
    values.extend(progress::values(input, &commits, &statuses, &verified));
    values.extend(uat::values(input, &statuses));
    let failing = verified
        .iter()
        .filter(|(_, v)| v.status == "gaps_found")
        .map(|(phase, _)| phase.as_str())
        .collect();
    let requirements = progress::requirement_states(input, &statuses, &failing);
    values.extend(audit::values(input, &requirements, &verified));
    PassOutput {
        values,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_derive_pass_gives_each_context_its_digest() {
        let entity = |id: &str, kind: &str| PassEntity {
            id: id.into(),
            kind: kind.into(),
            ..Default::default()
        };
        let input = PassInput {
            entities: vec![
                entity("v1.0/01/01-CONTEXT", "context"),
                entity("v1.0/02/02-CONTEXT", "context"),
                entity("v1.0/02/02-RESEARCH", "research"),
            ],
            ..Default::default()
        };
        let output = derive(&input);
        let given: Vec<(&str, &str)> = output
            .values
            .iter()
            .filter(|v| v.field == "digest")
            .map(|v| (v.entity.as_str(), v.field.as_str()))
            .collect();
        assert_eq!(
            given,
            [
                ("v1.0/01/01-CONTEXT", "digest"),
                ("v1.0/02/02-CONTEXT", "digest")
            ]
        );
        assert_ne!(output.values[0].value, output.values[1].value);
    }
}
