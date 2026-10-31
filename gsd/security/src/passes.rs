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
    let accepted: BTreeSet<&str> = of_kind(input, "accepted_risk")
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
