//! `security_gate <phase>`: the ship gate's verdict, `check predicate` plus `ship.md`'s bespoke security branch
//! (`SECURITY_SHIP_GATE_NO_REVIEW`, `SECURITY_SHIP_GATE_OPEN_THREATS`). Fail closed, as upstream's: no audit blocks, and
//! so does a `threats_open` that is not exactly 0. The workflow runs it at `ship:pre`; it is the gate, a verb the
//! workflow calls (decision U1), not a refusal of `/gsd-ship` by the kernel.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, required, run};

/// The verdict of a phase: `(block, message, code, exists, status, threats_open)`.
pub struct Verdict {
    pub block: bool,
    pub message: String,
    pub code: Option<&'static str>,
    pub exists: bool,
    pub status: Option<String>,
    pub threats_open: Option<i64>,
}

/// The verdict for phase `number` given its audit's `(status, threats_open)`, if it has one.
pub fn verdict(
    number: &str,
    enforcement: bool,
    audit: Option<(Option<String>, Option<i64>)>,
) -> Verdict {
    let exists = audit.is_some();
    let (status, threats_open) = audit.unwrap_or_default();
    let (block, code, message) = if !enforcement {
        (false, None, String::new())
    } else if !exists {
        (
            true,
            Some("SECURITY_SHIP_GATE_NO_REVIEW"),
            format!(
                "Security enforcement is enabled but no SECURITY.md exists for this phase.\nRun /gsd-secure-phase {number} \
                 and resolve findings before shipping."
            ),
        )
    } else if threats_open == Some(0) {
        (false, None, String::new())
    } else {
        let found = threats_open.map_or_else(|| "unset".to_string(), |n| n.to_string());
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
    }
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
    );
    let payload = json!({
        "block": v.block,
        "message": v.message,
        "details": {
            "code": v.code,
            "exists": v.exists,
            "status": v.status,
            "threats_open": v.threats_open,
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
            "The ship gate's verdict for a phase: blocks when it has no security audit or threats are open",
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
        let none = verdict("01", true, None);
        assert!(none.block && none.code == Some("SECURITY_SHIP_GATE_NO_REVIEW") && !none.exists);
        let open = verdict("01", true, Some((Some("draft".into()), Some(2))));
        assert!(open.block && open.code == Some("SECURITY_SHIP_GATE_OPEN_THREATS"));
        assert!(open.message.contains("(found: 2)"), "{}", open.message);
        let unset = verdict("01", true, Some((None, None)));
        assert!(unset.block && unset.message.contains("(found: unset)"));
        let clean = verdict("01", true, Some((Some("verified".into()), Some(0))));
        assert!(!clean.block && clean.code.is_none());
    }

    #[test]
    fn enforcement_off_never_blocks() {
        assert!(!verdict("01", false, None).block);
        assert!(!verdict("01", false, Some((None, Some(3)))).block);
    }
}
