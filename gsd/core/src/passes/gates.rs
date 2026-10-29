//! The gate guards: `gsd/E106`, `E511`, `E512`, `W318`, `W404` and `E516` (D-r39-12). A command's or prompt's gate
//! runs the guard passes that report its codes; a gate fails only on an Error.
//!
//! A guard also runs on every judged write, with no trigger. Every guard here answers nothing unless its trigger is a
//! gate (its trigger kind), so a write is never refused by one: an agent's plan, and `gsd-git`'s, stay free
//! of E512 on a project with an open window. Black-box, a gate can be evaluated only at a command's or prompt's entry,
//! and gsd has none until r41/r42: the guards are tested on pass inputs (`gates/tests.rs`), and `declaration_verify`
//! runs them through the SDK's MockHost.
//!
//! The scope of a gate is its subject (a phase, or a milestone and so its phases) or, with none, the whole project.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::derive::{ladder::window_status, verify::by_phase};
use super::plans::{link, of_kind, phase_of, text};
use crate::digest::scope_of;

type Check = fn(&PassInput) -> Vec<PassDiagnostic>;

const GUARDS: &[(&str, &str, Check)] = &[
    ("gate_e106", "E106", e106),
    ("gate_e511", "E511", e511),
    ("gate_e512", "E512", e512),
    ("gate_w318", "W318", w318),
    ("gate_w404", "W404", w404),
    ("gate_e516", "E516", e516),
];

pub fn declare(c: &mut ContributionsBuilder) {
    for (name, code, check) in GUARDS {
        c.pass(name, |p| {
            p.guard().reports(&[code]).run(*check);
        });
    }
}

/// The subject of a gate; `None` outside a gate, where every guard answers nothing.
fn gate(input: &PassInput) -> Option<Option<&str>> {
    // The protocol's discriminator is `kind`. The SDK does not export `TriggerKind`, so it is read by name.
    let trigger = input
        .trigger
        .as_ref()
        .filter(|t| format!("{:?}", t.kind) == "Gate")?;
    Some(trigger.subject.as_deref())
}

/// The phases in the gate's scope: the subject phase, a subject milestone's phases, or every phase.
fn phases<'a>(input: &'a PassInput, subject: Option<&str>) -> Vec<&'a PassEntity> {
    of_kind(input, "phase")
        .filter(|p| match subject {
            None => true,
            Some(s) => {
                p.id == s || crate::passes::commits::target(input, &p.id, "milestone") == Some(s)
            }
        })
        .filter(|p| !crate::passes::plans::name_of(&p.id).starts_with("999."))
        .collect()
}

fn refuse(
    code: &str,
    severity: PassSeverity,
    entity: &str,
    message: String,
    how: &str,
) -> PassDiagnostic {
    PassDiagnostic::new(code, severity, message)
        .with_suggestion(how)
        .with_entity(entity)
}

/// An id ordered as a person reads it: `A9` before `A10`.
fn natural(id: &str) -> (String, u64) {
    let digits = id.chars().rev().take_while(char::is_ascii_digit).count();
    let (head, number) = id.split_at(id.len() - digits);
    (head.to_string(), number.parse().unwrap_or(0))
}

/// E106: the latest audit of the subject milestone is `passed`, or `tech_debt` a person accepted.
fn e106(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input).flatten() else {
        return Vec::new();
    };
    // Only a milestone is audited: at a phase-scoped gate there is nothing to ask.
    if !of_kind(input, "milestone").any(|m| m.id == subject) {
        return Vec::new();
    }
    let mut audits: Vec<&PassEntity> = of_kind(input, "milestone_audit")
        .filter(|a| link(input, a, "milestone") == Some(subject))
        .collect();
    audits.sort_by_key(|a| natural(&a.id));
    let Some(latest) = audits.last() else {
        return vec![refuse(
            "E106",
            PassSeverity::Error,
            subject,
            format!("milestone '{subject}' has no audit"),
            "run /gsd-audit-milestone first",
        )];
    };
    let status = text(latest, "status");
    let accepted = latest.fields.contains_key("debt_accepted");
    if status == Some("passed") || (status == Some("tech_debt") && accepted) {
        return Vec::new();
    }
    let why = match status {
        Some("tech_debt") => "its tech debt is not accepted".to_string(),
        other => format!("its status is {}", other.unwrap_or("not computed")),
    };
    vec![refuse(
        "E106",
        PassSeverity::Error,
        &latest.id,
        format!(
            "the latest audit '{}' of milestone '{subject}' does not pass: {why}",
            latest.id
        ),
        "close the audit's gaps, or have a person accept its tech debt (`debt_accepted`)",
    )]
}

/// E511: every phase in scope has a `passed` verification (`ship.md:59-74`, `phase.cts:4000-4001`).
fn e511(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input) else {
        return Vec::new();
    };
    let verified = by_phase(input);
    phases(input, subject)
        .into_iter()
        .filter(|p| verified.get(&p.id).map(|v| v.status) != Some("passed"))
        .map(|p| {
            let status = verified.get(&p.id).map_or("none", |v| v.status);
            refuse(
                "E511",
                PassSeverity::Error,
                &p.id,
                format!(
                    "phase '{}' is not verified: its verification is {status}",
                    p.id
                ),
                "run /gsd-verify-work until the verification passes",
            )
        })
        .collect()
}

/// E512: no window in the project is open (the `windows_enforce` gate). A window is project-scoped, and the host
/// keeps only the errors of a subject-scoped gate that name an entity within the subject, so at such a gate the
/// diagnostic names the subject: an open window anywhere refuses the gate, as upstream's project-wide `ship:pre`.
fn e512(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input) else {
        return Vec::new();
    };
    of_kind(input, "broken_window")
        .filter(|w| window_status(input, w) == "open")
        .map(|w| {
            refuse(
                "E512",
                PassSeverity::Error,
                subject.unwrap_or(&w.id),
                format!("broken window '{}' is open", w.id),
                "fix the window, or waive it with a reason",
            )
        })
        .collect()
}

/// The plan a plan or a summary subject names: a plan is its own, a summary's is the plan that scopes it.
fn subject_plan<'a>(input: &'a PassInput, subject: &str) -> Option<&'a PassEntity> {
    let plan = if of_kind(input, "summary").any(|s| s.id == subject) {
        scope_of(subject)
    } else {
        subject
    };
    of_kind(input, "plan").find(|p| p.id == plan)
}

/// The plans in the gate's scope: the plan (or the summary's plan) the subject names, else the phases' plans.
fn plans_in<'a>(input: &'a PassInput, subject: Option<&str>) -> Vec<&'a PassEntity> {
    if let Some(plan) = subject.and_then(|s| subject_plan(input, s)) {
        return vec![plan];
    }
    let phases: BTreeSet<&str> = phases(input, subject)
        .into_iter()
        .map(|p| p.id.as_str())
        .collect();
    of_kind(input, "plan")
        .filter(|p| phase_of(input, &p.id).is_some_and(|ph| phases.contains(ph)))
        .collect()
}

/// Whether `entity` has a stale judgement that counts at a gate. A plan-check verdict (W311, W313, W315, W316) of a
/// plan whose summary is `completed` is history: the plan check judges a plan before it runs, and answering a checkpoint or
/// confirming a setup mid-execution changes the digest it was recorded over (U33). The judgement itself stays stale.
fn stale(input: &PassInput, entity: &PassEntity) -> bool {
    // Executed means a summary with `completed` (as E517): a summary merely created before the run is not one.
    let executed = entity.kind == "plan"
        && of_kind(input, "summary")
            .any(|s| scope_of(&s.id) == entity.id && s.fields.contains_key("completed"));
    entity.judgements.iter().any(|j| {
        let plan_check = j
            .rule
            .strip_prefix("gsd/")
            .and_then(crate::judged::get)
            .is_some_and(|r| r.judge == "gsd-plan-checker");
        j.state == "stale" && !(executed && plan_check)
    })
}

/// W318: a plan's plan-check verdict is stale.
fn w318(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input) else {
        return Vec::new();
    };
    plans_in(input, subject)
        .into_iter()
        .filter(|p| stale(input, p))
        .map(|p| {
            refuse(
                "W318",
                PassSeverity::Warning,
                &p.id,
                format!("the plan-check verdict on plan '{}' is stale", p.id),
                "check the plan again before executing it",
            )
        })
        .collect()
}

/// W404: a `tdd` plan with a summary has an answered `tdd_review` checkpoint.
fn w404(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input) else {
        return Vec::new();
    };
    plans_in(input, subject)
        .into_iter()
        .filter(|p| text(p, "type") == Some("tdd"))
        .filter(|p| of_kind(input, "summary").any(|s| scope_of(&s.id) == p.id))
        .filter(|p| {
            !of_kind(input, "checkpoint").any(|c| {
                scope_of(&c.id) == p.id
                    && text(c, "kind") == Some("tdd_review")
                    && c.fields.contains_key("answer")
            })
        })
        .map(|p| {
            // The host keeps a diagnostic only when it names an entity within the subject's scope: at a summary,
            // the summary (as E512 names its subject), since its plan is outside the summary's scope.
            let entity = subject
                .filter(|s| of_kind(input, "summary").any(|e| e.id == *s))
                .unwrap_or(&p.id);
            refuse(
                "W404",
                PassSeverity::Warning,
                entity,
                format!(
                    "tdd plan '{}' has a summary and no answered TDD review checkpoint",
                    p.id
                ),
                "run the TDD review and record its answer",
            )
        })
        .collect()
}

/// E516: no stale verdict is in scope.
fn e516(input: &PassInput) -> Vec<PassDiagnostic> {
    if gate(input).is_none() {
        return Vec::new();
    }
    input
        .entities
        .iter()
        .filter(|e| stale(input, e))
        .map(|e| {
            refuse(
                "E516",
                PassSeverity::Error,
                &e.id,
                format!("the verdict on '{}' is stale", e.id),
                "record the verdict again over the current inputs",
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
