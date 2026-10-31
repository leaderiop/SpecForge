//! The passes of `@gsd/security`: the compute pass for the model's computed fields (plan 16 T11).
//!
//! `threat.blocking` is `status != closed`, no `accepted_risk` naming the threat, and a severity at or above
//! `workflow.security_block_on` (`none` blocks nothing). `security_audit.threats_open` counts the blocking threats of
//! the audit's phase, `status` is `verified` once the audit has a run and nothing blocks, and `register_planned` says a
//! threat of the phase was authored at plan time. Upstream froze `threats_open` in SECURITY.md's frontmatter when the
//! audit ran; computing it makes a changed `block_on` or an accepted risk take effect at once
//! (`[threats-open-live]`).

use std::collections::BTreeSet;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use crate::commands::support::name_of;
use crate::commands::threat_ids::{claims, duplicates};

/// Severities, lowest first; a threshold of `none` is not in the list and blocks nothing.
const ORDER: &[&str] = &["low", "medium", "high", "critical"];

/// Whether an open threat of `severity` blocks under `block_on`.
pub fn blocks(severity: &str, block_on: &str) -> bool {
    let rank = |s: &str| ORDER.iter().position(|o| *o == s);
    matches!((rank(severity), rank(block_on)), (Some(s), Some(b)) if s >= b)
}

pub fn declare(c: &mut ContributionsBuilder) {
    let computes = crate::kinds::computed();
    c.pass("security", |p| {
        let names: Vec<&str> = computes.iter().map(String::as_str).collect();
        p.compute().computes(&names).run(compute);
    });
    c.pass("threat_ids", |p| {
        p.phase("check").run(threat_ids);
    });
    c.pass("audit_advisory", |p| {
        p.phase("check").run(advisory);
    });
}

/// `gsd.security/W301` on a phase whose status is `complete` while its audit is missing, still a draft, or has threats
/// open: upstream's close-time advisory (#4972), here on every `check` (`[audit-advisory-every-check]`: core has no
/// close verb, the phase status is computed). `workflow.security_enforcement` gates it off.
fn advisory(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "phase")
        .filter(|phase| phase.fields.get("status").map(String::as_str) == Some("complete"))
        .filter_map(|phase| {
            let number = name_of(&phase.id);
            let audit = of_kind(input, "security_audit")
                .find(|a| link(input, &a.id, "phase") == Some(phase.id.as_str()));
            let said = match audit {
                None => format!("{number}-SECURITY is missing"),
                Some(a) => {
                    let open: u64 = a
                        .fields
                        .get("threats_open")
                        .and_then(|n| n.trim().parse().ok())
                        .unwrap_or(0);
                    if open > 0 {
                        format!("{number}-SECURITY has {open} threat(s) open")
                    } else if a.fields.get("status").map(String::as_str) == Some("verified") {
                        return None;
                    } else {
                        format!("{number}-SECURITY is still draft")
                    }
                }
            };
            Some(
                PassDiagnostic::new(
                    "W301",
                    PassSeverity::Warning,
                    format!("{said}: run /gsd-secure-phase {number}"),
                )
                .with_entity(audit.map_or(&phase.id, |a| &a.id)),
            )
        })
        .collect()
}

/// `gsd.security/E101` on each threat that claims an ID another plan of its phase claims.
fn threat_ids(input: &PassInput) -> Vec<PassDiagnostic> {
    let name = |id: &str| name_of(id).to_string();
    let claimed: Vec<(String, &PassEntity, String)> = of_kind(input, "threat")
        .filter(|t| claims(&name(&t.id)))
        .filter_map(|t| {
            let plan = link(input, &t.id, "plan")?;
            Some((name(link(input, plan, "phase")?), t, name(plan)))
        })
        .collect();
    let duplicated = duplicates(
        claimed
            .iter()
            .map(|(p, t, plan)| (p.as_str(), name_of(&t.id), plan.as_str())),
    );
    let highest = |phase: &str| {
        claimed
            .iter()
            .filter(|(p, _, _)| p == phase)
            .filter_map(|(_, t, _)| name(&t.id).rsplit('-').next()?.parse::<u64>().ok())
            .max()
            .unwrap_or(0)
    };
    claimed
        .iter()
        .filter_map(|(phase, threat, _)| {
            let (_, id, plans) = duplicated
                .iter()
                .find(|(p, id, _)| *p == phase && *id == name_of(&threat.id))?;
            Some(
                PassDiagnostic::new(
                    "E101",
                    PassSeverity::Error,
                    format!(
                        "threat ID {id} is claimed by more than one plan of phase {phase}: {}",
                        plans.join(", ")
                    ),
                )
                .with_suggestion(format!(
                    "renumber it after the phase's highest, T-{phase}-{:02}",
                    highest(phase) + 1
                ))
                .with_entity(&threat.id),
            )
        })
        .collect()
}

fn of_kind<'a>(input: &'a PassInput, kind: &'a str) -> impl Iterator<Item = &'a PassEntity> {
    input.entities.iter().filter(move |e| e.kind == kind)
}

/// The entity `field` resolves to (its edge), `None` when unwritten or unresolved.
fn link<'a>(input: &'a PassInput, entity: &str, field: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == entity && e.label == field)
        .map(|e| e.target.as_str())
}

fn value(entity: &str, field: &str, value: impl Into<serde_json::Value>) -> PassValue {
    PassValue {
        entity: entity.to_string(),
        field: field.to_string(),
        value: value.into(),
    }
}

/// The phase a threat is in: its plan's phase. `None` for a quick plan, which has none.
fn phase_of_threat<'a>(input: &'a PassInput, threat: &PassEntity) -> Option<&'a str> {
    let plan = link(input, &threat.id, "plan")?;
    link(input, plan, "phase")
}

fn compute(input: &PassInput) -> PassOutput {
    let settings = Settings::current();
    let block_on = settings
        .string("workflow.security_block_on")
        .unwrap_or("high");
    // only `accept_risk` sets `at`, so a risk without it is an agent's stub and accepts nothing
    let accepted: BTreeSet<&str> = of_kind(input, "accepted_risk")
        .filter(|r| r.fields.contains_key("at"))
        .filter_map(|r| link(input, &r.id, "threat"))
        .collect();
    let mut values = Vec::new();
    // (phase, blocking, planned) per threat
    let mut by_phase: Vec<(&str, bool, bool)> = Vec::new();
    for threat in of_kind(input, "threat") {
        let open = threat.fields.get("status").map(String::as_str) != Some("closed");
        let severity = threat.fields.get("severity").map_or("", |s| s.trim());
        let blocking = open && !accepted.contains(threat.id.as_str()) && blocks(severity, block_on);
        values.push(value(&threat.id, "blocking", blocking));
        let retroactive = threat.fields.get("retroactive").map(String::as_str) == Some("true");
        if let Some(phase) = phase_of_threat(input, threat) {
            by_phase.push((phase, blocking, !retroactive));
        }
    }
    for audit in of_kind(input, "security_audit") {
        let Some(phase) = link(input, &audit.id, "phase") else {
            continue;
        };
        let here = by_phase.iter().filter(|(p, _, _)| *p == phase);
        let threats_open = here.clone().filter(|(_, blocking, _)| *blocking).count();
        let planned = here.clone().any(|(_, _, planned)| *planned);
        let has_run = input
            .edges
            .iter()
            .any(|e| e.label == "audit" && e.target == audit.id)
            && of_kind(input, "audit_run")
                .any(|r| link(input, &r.id, "audit") == Some(audit.id.as_str()));
        let status = if has_run && threats_open == 0 {
            "verified"
        } else {
            "draft"
        };
        values.push(value(&audit.id, "threats_open", json!(threats_open)));
        values.push(value(&audit.id, "status", status));
        values.push(value(&audit.id, "register_planned", planned));
    }
    PassOutput {
        values,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::blocks;

    #[test]
    fn severity_orders_critical_over_low_and_none_blocks_nothing() {
        assert!(blocks("critical", "high"));
        assert!(blocks("high", "high"));
        assert!(!blocks("medium", "high"));
        assert!(blocks("low", "low"));
        assert!(!blocks("critical", "none"));
        assert!(!blocks("", "high"));
    }
}
