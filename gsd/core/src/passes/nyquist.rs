//! The Nyquist rules, under `workflow.nyquist_validation`: `gsd/E314`, a task of a phase that has a validation has an
//! automated verify or a wave-0 item (D8), and `gsd/W510`, a phase that needs a validation has one, validated before
//! its verification. GSD skips D8 for a phase without a validation, and `validate-phase` is opt-in, so a plain planned
//! phase is not warned.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{children, link, links, of_kind, phase_of, rows};

/// The words a research uses for the section that asks for a validation.
const SECTION: &str = "validation architecture";

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("nyquist", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let validations: Vec<&PassEntity> = of_kind(input, "validation").collect();
    let validated_phases: BTreeSet<&str> = validations
        .iter()
        .filter_map(|v| link(input, v, "phase"))
        .collect();
    let mut found = tasks_without_a_check(input, &validated_phases);
    found.extend(missing_validations(input, &validations, &validated_phases));
    found
}

/// E314: a task of a validated phase with no automated verify and no wave-0 item for its plan's requirements.
fn tasks_without_a_check(input: &PassInput, validated: &BTreeSet<&str>) -> Vec<PassDiagnostic> {
    let wave0: BTreeSet<(&str, &str)> = of_kind(input, "wave0_item")
        .filter_map(|w| {
            let validation = link(input, w, "validation")?;
            let validation = of_kind(input, "validation").find(|v| v.id == validation)?;
            Some((link(input, validation, "phase")?, link(input, w, "for")?))
        })
        .collect();
    let mut found = Vec::new();
    for plan in of_kind(input, "plan") {
        let Some(phase) = phase_of(input, &plan.id).filter(|p| validated.contains(p)) else {
            continue;
        };
        let covered = links(input, plan, "requirements")
            .iter()
            .any(|r| wave0.contains(&(phase, *r)));
        for task in children(input, &plan.id, "task") {
            let automated = rows(task, "verify")
                .iter()
                .any(|row| row.get("automated").is_some_and(|c| !c.trim().is_empty()));
            if !automated
                && !covered
                && task.fields.get("type").map(String::as_str) != Some("manual")
            {
                found.push(
                    PassDiagnostic::new(
                        "E314",
                        PassSeverity::Error,
                        format!(
                            "task '{}' has no automated verify and no wave-0 item for its requirements",
                            task.id
                        ),
                    )
                    .with_suggestion(
                        "add `verify [{ automated \"…\" fails_when \"…\" }]`, or a wave0_item of the phase's validation that names the task's requirement in `for` (Nyquist 8a)",
                    )
                    .with_entity(&task.id),
                );
            }
        }
    }
    found
}

/// W510: a planned phase whose research asks for a validation has none, or a verification exists while the phase's
/// validation is not approved.
fn missing_validations(
    input: &PassInput,
    validations: &[&PassEntity],
    validated: &BTreeSet<&str>,
) -> Vec<PassDiagnostic> {
    let planned: BTreeSet<&str> = of_kind(input, "plan")
        .filter_map(|p| phase_of(input, &p.id))
        .collect();
    let mut found = Vec::new();
    for research in of_kind(input, "research") {
        let Some(phase) = link(input, research, "phase").filter(|p| planned.contains(p)) else {
            continue;
        };
        let asks = research
            .fields
            .values()
            .any(|text| text.to_lowercase().contains(SECTION));
        if asks && !validated.contains(phase) {
            found.push(
                PassDiagnostic::new(
                    "W510",
                    PassSeverity::Warning,
                    format!(
                        "phase '{phase}' is planned and its research has a Validation Architecture section, but it has no validation"
                    ),
                )
                .with_suggestion("seed the phase's validation from the research: /gsd-validate-phase")
                .with_entity(&research.id),
            );
        }
    }
    for verification in of_kind(input, "verification") {
        let phase = link(input, verification, "phase");
        let Some(validation) = validations
            .iter()
            .find(|v| phase.is_some() && link(input, v, "phase") == phase)
        else {
            continue;
        };
        if !validation.fields.contains_key("approval") {
            found.push(
                PassDiagnostic::new(
                    "W510",
                    PassSeverity::Warning,
                    format!(
                        "verification '{}' exists while validation '{}' is still a draft",
                        verification.id, validation.id
                    ),
                )
                .with_suggestion("approve the validation before verifying the phase")
                .with_entity(&verification.id),
            );
        }
    }
    found
}
