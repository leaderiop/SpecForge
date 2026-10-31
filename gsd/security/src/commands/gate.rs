//! `security_gate <phase>`: the ship gate's verdict, `check predicate` plus `ship.md`'s bespoke security branch
//! (`SECURITY_SHIP_GATE_NO_REVIEW`, `SECURITY_SHIP_GATE_OPEN_THREATS`). Fail closed, as upstream's: no audit blocks, and
//! so does a `threats_open` that is not exactly 0. Parity+ (REVIEW-security-part2 m3): an audit that never ran (`status`
//! is not `verified`) blocks too, and so does a security model that does not load or check clean
//! (`SECURITY_SHIP_GATE_UNREADABLE`, the reasons in `details.problems`). The workflow runs it at `ship:pre`; it is the
//! gate, a verb the workflow calls (decision U1), not a refusal of `/gsd-ship` by the kernel.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, name_of, node, required, run, target};
use crate::kinds::{CATEGORY, DISPOSITION, SEVERITY};

/// The verdict of a phase: `(block, message, code, exists, status, threats_open)`.
pub struct Verdict {
    pub block: bool,
    pub message: String,
    pub code: Option<&'static str>,
    pub exists: bool,
    pub status: Option<String>,
    pub threats_open: Option<i64>,
    pub problems: Vec<String>,
}

/// The verdict for phase `number` given its audit's `(status, threats_open)`, if it has one.
pub fn verdict(
    number: &str,
    enforcement: bool,
    audit: Option<(Option<String>, Option<i64>)>,
    problems: Vec<String>,
) -> Verdict {
    let exists = audit.is_some();
    let (status, threats_open) = audit.unwrap_or_default();
    let (block, code, message) = if !enforcement {
        (false, None, String::new())
    } else if !problems.is_empty() {
        (
            true,
            Some("SECURITY_SHIP_GATE_UNREADABLE"),
            format!(
                "Security ship gate: the security model of phase {number} does not load or check clean ({}).\nRun \
                 `specforge check`, fix it, then re-run /gsd-secure-phase {number} before shipping.",
                problems.join("; ")
            ),
        )
    } else if !exists {
        (
            true,
            Some("SECURITY_SHIP_GATE_NO_REVIEW"),
            format!(
                "Security enforcement is enabled but no SECURITY.md exists for this phase.\nRun /gsd-secure-phase {number} \
                 and resolve findings before shipping."
            ),
        )
    } else if threats_open == Some(0) && status.as_deref() == Some("verified") {
        (false, None, String::new())
    } else {
        // a draft audit never ran (or a threat is open): `found` says which
        let found = match (threats_open, status.as_deref()) {
            (Some(0), Some(status)) => status.to_string(),
            (Some(n), _) => n.to_string(),
            (None, _) => "unset".to_string(),
        };
        (
            true,
            Some("SECURITY_SHIP_GATE_OPEN_THREATS"),
            format!(
                "Security ship gate: SECURITY.md does not assert threats_open == 0 (found: {found}).\nResolve open threats \
                 (or re-run /gsd-secure-phase {number}) before shipping."
            ),
        )
    };
    Verdict {
        block,
        message,
        code,
        exists,
        status,
        threats_open,
        problems,
    }
}

/// Why the security entities of `phase` cannot be trusted, from what the graph holds: the host leaves a reference it
/// cannot resolve out of the graph, and an entity of a block that does not parse is not there at all. Fail closed:
/// any reason blocks.
// shortcut: a `.spec` file that does not parse is seen only through what it takes with it (a dangling reference, or
// fewer threats than the last audit run counted); `specforge check` reports the parse error itself. Upgrade when a
// command is handed the compile's diagnostics.
fn problems(graph: &CommandGraph, phase: &GraphNode, audit: Option<&GraphNode>) -> Vec<String> {
    let mut found = Vec::new();
    let mut bad = |what: String| {
        if !found.contains(&what) {
            found.push(what);
        }
    };
    let in_phase = |plan: &str| target(graph, plan, "phase") == Some(phase.id.as_str());
    let mut threats = 0_i64;
    for t in graph.nodes_of_kind("threat") {
        let Some(plan) = target(graph, &t.id, "plan") else {
            // it could be of any phase
            bad(format!("threat {} names no plan that exists", t.id));
            continue;
        };
        if !in_phase(plan) {
            continue;
        }
        threats += 1;
        for (field, allowed) in [
            ("severity", SEVERITY),
            ("category", CATEGORY),
            ("disposition", DISPOSITION),
        ] {
            if !t.text(field).is_some_and(|v| allowed.contains(&v.trim())) {
                bad(format!("threat {} has no valid {field}", name_of(&t.id)));
            }
        }
        if t.text("status")
            .is_some_and(|s| !["open", "closed"].contains(&s))
        {
            bad(format!("threat {} has an invalid status", name_of(&t.id)));
        }
    }
    for a in graph.nodes_of_kind("security_audit") {
        if target(graph, &a.id, "phase").is_none() {
            bad(format!("audit {} names no phase that exists", a.id));
        }
    }
    for r in graph.nodes_of_kind("accepted_risk") {
        let (audit_of, threat_of) = (
            target(graph, &r.id, "audit"),
            target(graph, &r.id, "threat"),
        );
        if audit_of.is_none() || threat_of.is_none() {
            bad(format!(
                "accepted risk {} names an audit or a threat that does not exist",
                r.id
            ));
        }
    }
    // the last run counted the register as it was: fewer threats now means some did not load
    let counted = audit.and_then(|a| {
        graph
            .nodes_of_kind("audit_run")
            .filter(|r| target(graph, &r.id, "audit") == Some(a.id.as_str()))
            .max_by_key(|r| super::support::natural(name_of(&r.id)))
            .and_then(|r| {
                r.fields
                    .get("total")
                    .and_then(|n| n.as_i64().or_else(|| n.as_str()?.trim().parse().ok()))
            })
    });
    if let Some(counted) = counted.filter(|c| *c > threats) {
        bad(format!(
            "the last audit run counted {counted} threats and {threats} load"
        ));
    }
    found
}

fn security_gate(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let phase = node(call, "phase", required(call, "phase")?)?;
    let number = phase.id.rsplit('/').next().unwrap_or(&phase.id);
    let graph = call.graph();
    let audit = graph.nodes_of_kind("security_audit").find(|a| {
        graph
            .edges_from(&a.id)
            .iter()
            .any(|e| e.label == "phase" && e.target == phase.id)
    });
    let enforcement = Settings::current()
        .bool("workflow.security_enforcement")
        .unwrap_or(true);
    let found = problems(graph, phase, audit);
    let v = verdict(
        number,
        enforcement,
        audit.map(|a| {
            (
                a.text("status").map(str::to_string),
                a.fields
                    .get("threats_open")
                    .and_then(|n| n.as_i64().or_else(|| n.as_str()?.trim().parse().ok())),
            )
        }),
        found,
    );
    let payload = json!({
        "block": v.block,
        "message": v.message,
        "details": {
            "code": v.code,
            "exists": v.exists,
            "status": v.status,
            "threats_open": v.threats_open,
            "problems": v.problems,
        },
    });
    let human = if v.block {
        format!("{}\n", v.message)
    } else {
        format!("phase {number}: the security gate passes\n")
    };
    Ok(answer(call, payload, human))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("security_gate", |cmd| {
        cmd.description(
            "The ship gate's verdict for a phase: blocks when it has no security audit, the audit never ran, threats are open or the security model does not load",
        )
        .arg("phase", |a| {
            a.required().description("the phase (its id, or its number)");
        })
        .handler(|call| run(call, security_gate));
    });
}

#[cfg(test)]
mod tests {
    use super::verdict;

    #[test]
    fn no_audit_blocks_open_threats_block_zero_passes() {
        let none = verdict("01", true, None, vec![]);
        assert!(none.block && none.code == Some("SECURITY_SHIP_GATE_NO_REVIEW") && !none.exists);
        let open = verdict("01", true, Some((Some("draft".into()), Some(2))), vec![]);
        assert!(open.block && open.code == Some("SECURITY_SHIP_GATE_OPEN_THREATS"));
        assert!(open.message.contains("(found: 2)"), "{}", open.message);
        let unset = verdict("01", true, Some((None, None)), vec![]);
        assert!(unset.block && unset.message.contains("(found: unset)"));
        let clean = verdict("01", true, Some((Some("verified".into()), Some(0))), vec![]);
        let draft = verdict("01", true, Some((Some("draft".into()), Some(0))), vec![]);
        assert!(draft.block && draft.message.contains("(found: draft)"));
        let unreadable = verdict(
            "01",
            true,
            Some((Some("verified".into()), Some(0))),
            vec!["x".into()],
        );
        assert!(unreadable.block && unreadable.code == Some("SECURITY_SHIP_GATE_UNREADABLE"));
        assert!(!clean.block && clean.code.is_none());
    }

    #[test]
    fn enforcement_off_never_blocks() {
        assert!(!verdict("01", false, None, vec![]).block);
        assert!(!verdict("01", false, Some((None, Some(3))), vec![]).block);
    }
}
