//! The computed values of validation, the milestone audit, broken windows and refactor proposals (`spec/verify.md`,
//! D-r39-8).
//!
//! A validation is `validated` once `approval` is set. A row's `wave` is its task's plan's wave and its `status` is
//! `green` or `red` from the task's recorded tests (`flaky` is never derived). `nyquist_compliant` holds when every row
//! is green and every task of the phase has a row or a manual check; `wave_0_complete` when every wave-0 item is done.
//! A milestone audit lists the milestone's requirements that are not complete, counts the Nyquist state of its phases
//! and is `gaps_found`, `tech_debt` or `passed`; `integration` joins in r40.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::execute::value;
use super::ladder::window_status;
use super::verify::Verified;
use crate::passes::commits::target;
use crate::passes::plans::{children, of_kind, phase_of, strings, text};

/// A task's recorded tests: `red` when one failed, `green` when some passed and none failed, else `pending`.
fn row_status(input: &PassInput, task: &str) -> &'static str {
    let Some(report) = input.test_results.as_ref() else {
        return "pending";
    };
    let tests = report.of(task);
    if tests.iter().any(|t| t.status == TestStatus::Failed) {
        "red"
    } else if tests.iter().any(|t| t.status == TestStatus::Passed) {
        "green"
    } else {
        "pending"
    }
}

/// What a validation computes: `(nyquist_compliant, wave_0_complete)`.
struct Coverage {
    compliant: bool,
    wave0: bool,
}

fn rows_of<'a>(input: &'a PassInput, validation: &PassEntity) -> Vec<&'a PassEntity> {
    of_kind(input, "validation_row")
        .filter(|r| text(r, "validation") == Some(validation.id.as_str()))
        .collect()
}

fn coverage(input: &PassInput, validation: &PassEntity) -> Coverage {
    let rows = rows_of(input, validation);
    let phase = text(validation, "phase").unwrap_or_default();
    let manual: BTreeSet<&str> = of_kind(input, "manual_check")
        .filter(|m| text(m, "validation") == Some(validation.id.as_str()))
        .filter_map(|m| text(m, "requirement"))
        .collect();
    let named: BTreeSet<&str> = rows.iter().filter_map(|r| text(r, "task")).collect();
    let covered = of_kind(input, "plan")
        .filter(|p| phase_of(input, &p.id) == Some(phase))
        .all(|plan| {
            let by_manual = strings(plan, "requirements")
                .iter()
                .any(|r| manual.contains(r.as_str()));
            children(input, &plan.id, "task")
                .iter()
                .all(|t| named.contains(t.id.as_str()) || by_manual)
        });
    let green = rows
        .iter()
        .all(|r| text(r, "task").is_some_and(|t| row_status(input, t) == "green"));
    let wave0 = of_kind(input, "wave0_item")
        .filter(|w| text(w, "validation") == Some(validation.id.as_str()))
        .all(|w| text(w, "done") == Some("true"));
    Coverage {
        compliant: green && covered && !(rows.is_empty() && manual.is_empty()),
        wave0,
    }
}

fn validation_values(input: &PassInput) -> Vec<PassValue> {
    let mut out = Vec::new();
    for validation in of_kind(input, "validation") {
        let covered = coverage(input, validation);
        let status = if validation.fields.contains_key("approval") {
            "validated"
        } else {
            "draft"
        };
        out.push(value(&validation.id, "status", status));
        out.push(value(
            &validation.id,
            "nyquist_compliant",
            covered.compliant,
        ));
        out.push(value(&validation.id, "wave_0_complete", covered.wave0));
        for row in rows_of(input, validation) {
            let Some(task) = text(row, "task") else {
                continue;
            };
            out.push(value(&row.id, "status", row_status(input, task)));
            let wave = of_kind(input, "plan")
                .find(|p| p.id == crate::digest::scope_of(task))
                .and_then(|p| text(p, "wave"))
                .and_then(|w| w.parse::<i64>().ok());
            if let Some(wave) = wave {
                out.push(value(&row.id, "wave", wave));
            }
        }
    }
    out
}

/// The Nyquist state of a phase: `compliant`, `partial` (validated, not compliant), `not_validated` (a draft) or
/// `missing` (`audit-milestone.md:176-185`).
fn nyquist_state(input: &PassInput, phase: &str) -> &'static str {
    let Some(validation) = of_kind(input, "validation").find(|v| text(v, "phase") == Some(phase))
    else {
        return "missing";
    };
    if !validation.fields.contains_key("approval") {
        "not_validated"
    } else if coverage(input, validation).compliant {
        "compliant"
    } else {
        "partial"
    }
}

/// `orphaned` (no plan names the requirement), `partial` (in progress or with gaps) or `unsatisfied`.
fn gap_status(input: &PassInput, requirement: &str, state: &str) -> &'static str {
    let named = input.edges.iter().any(|e| {
        e.label == "requirements"
            && e.target == requirement
            && of_kind(input, "plan").any(|p| p.id == e.source)
    });
    if !named {
        "orphaned"
    } else if matches!(state, "in_progress" | "gaps_found") {
        "partial"
    } else {
        "unsatisfied"
    }
}

fn audit_values(
    input: &PassInput,
    requirement_states: &BTreeMap<&str, &'static str>,
    verified: &BTreeMap<String, Verified>,
) -> Vec<PassValue> {
    let mut out = Vec::new();
    for audit in of_kind(input, "milestone_audit") {
        let Some(milestone) = text(audit, "milestone") else {
            continue;
        };
        let phases: Vec<&PassEntity> = of_kind(input, "phase")
            .filter(|p| target(input, &p.id, "milestone") == Some(milestone))
            .filter(|p| !crate::passes::plans::name_of(&p.id).starts_with("999."))
            .collect();
        let requirements: BTreeSet<&str> = phases
            .iter()
            .flat_map(|p| strings(p, "requirements"))
            .filter_map(|r| {
                requirement_states
                    .keys()
                    .find(|k| **k == r.as_str())
                    .copied()
            })
            .collect();
        let gaps: Vec<(&str, &str)> = requirements
            .iter()
            .filter(|r| requirement_states.get(**r) != Some(&"complete"))
            .map(|r| (*r, gap_status(input, r, requirement_states[*r])))
            .collect();
        if !gaps.is_empty() {
            let rows: Vec<Value> = gaps
                .iter()
                .map(
                    |(requirement, status)| json!({ "requirement": requirement, "status": status }),
                )
                .collect();
            out.push(value(&audit.id, "gaps", rows));
        }
        let mut counts = BTreeMap::from([
            ("compliant", 0),
            ("partial", 0),
            ("not_validated", 0),
            ("missing", 0),
        ]);
        for phase in &phases {
            *counts.entry(nyquist_state(input, &phase.id)).or_insert(0) += 1;
        }
        out.push(value(&audit.id, "nyquist", json!(counts)));
        let unverified = phases
            .iter()
            .any(|p| verified.get(&p.id).map(|v| v.status) != Some("passed"));
        let debt = audit
            .values
            .get("tech_debt")
            .and_then(Value::as_array)
            .is_some_and(|d| !d.is_empty());
        let status = if gaps.iter().any(|(_, s)| *s != "partial") || unverified {
            "gaps_found"
        } else if debt {
            "tech_debt"
        } else {
            "passed"
        };
        out.push(value(&audit.id, "status", status));
    }
    out
}

fn window_values(input: &PassInput) -> Vec<PassValue> {
    let mut out = Vec::new();
    for window in of_kind(input, "broken_window") {
        out.push(value(&window.id, "status", window_status(input, window)));
        if let Some(milestone) = text(window, "phase").and_then(|p| target(input, p, "milestone")) {
            out.push(value(&window.id, "milestone", milestone));
        }
    }
    for proposal in of_kind(input, "refactor_proposal") {
        let outcome = proposal
            .values
            .get("disposition")
            .and_then(|d| d.get("outcome"))
            .and_then(Value::as_str)
            .unwrap_or("proposed");
        out.push(value(&proposal.id, "status", outcome.to_string()));
    }
    out
}

pub fn values(
    input: &PassInput,
    requirement_states: &BTreeMap<&str, &'static str>,
    verified: &BTreeMap<String, Verified>,
) -> Vec<PassValue> {
    let mut out = validation_values(input);
    out.extend(audit_values(input, requirement_states, verified));
    out.extend(window_values(input));
    out
}

#[cfg(test)]
mod tests;
