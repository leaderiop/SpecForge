//! `record_security_audit <phase> --verdict '<json>' [--run-by <who>]`: workflow step 6 and `verification.append-audit`
//! in one command, the single writer of the audit (gsd-core #2119: the auditor returns its verdict, it writes nothing).
//!
//! The verdict is `{"threats": [{"id", "status": "open"|"closed", "evidence"?}, ...]}`. A threat of the phase's plans
//! gets its `status` and `evidence`; one an `accepted_risk` names is left as it is ("Accepted risks do not resurface").
//! A threat the plans do not hold is recorded only when the verdict says `"retroactive": true` and names the `plan` of
//! the phase to put it in, with the fields of the kind (`[retro-threat-in-plan]`: upstream kept the retroactive
//! register in SECURITY.md only). The audit is created on the first call at the current ASVS level, and an `audit_run`
//! is appended only when `(total, closed, open)` differ from the last run's, as `append-audit` does. Everything is
//! checked before anything is written.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, name_of, next_id, node, now, required, run, target};
use crate::kinds::{CATEGORY, DISPOSITION, SEVERITY};

const DEFAULT_RUN_BY: &str = "gsd-security-auditor";

/// A threat of the phase as the graph holds it.
struct Held<'a> {
    node: &'a GraphNode,
    accepted: bool,
}

fn text<'a>(item: &'a Value, key: &str) -> Option<&'a str> {
    item.get(key)?
        .as_str()
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

/// A required enum field of a retroactive threat.
fn choice<'a>(item: &'a Value, key: &str, allowed: &[&str]) -> Result<&'a str, String> {
    let id = text(item, "id").unwrap_or("?");
    match text(item, key) {
        Some(v) if allowed.contains(&v) => Ok(v),
        Some(v) => Err(format!(
            "threat {id}: {key} '{v}' is not one of {}",
            allowed.join(", ")
        )),
        None => Err(format!("threat {id}: a retroactive threat needs '{key}'")),
    }
}

/// The file the phase's audit and its runs live in: the `security.spec` beside the phase's first plan, else that plan's
/// file. A new file would not round-trip (the host leaves an emptied file behind when its last entity is deleted).
pub(super) fn audit_file(graph: &CommandGraph, phase: &GraphNode, plans: &[&GraphNode]) -> String {
    let existing = graph
        .nodes_of_kind("security_audit")
        .filter_map(|a| a.file.as_deref())
        .next();
    let anchor = plans
        .first()
        .and_then(|p| p.file.as_deref())
        .or(phase.file.as_deref())
        .unwrap_or("spec/security.spec");
    let beside = match anchor.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/security.spec"),
        None => "security.spec".to_string(),
    };
    match existing {
        Some(file) => file.to_string(),
        None if graph
            .nodes()
            .iter()
            .any(|n| n.file.as_deref() == Some(&beside)) =>
        {
            beside
        }
        None => anchor.to_string(),
    }
}

fn record(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let phase = node(call, "phase", required(call, "phase")?)?;
    let number = name_of(&phase.id);
    let verdict: Value = serde_json::from_str(required(call, "verdict")?)
        .map_err(|e| format!("verdict: not the JSON it takes: {e}"))?;
    let named = verdict["threats"]
        .as_array()
        .ok_or("verdict: needs a \"threats\" list")?;
    let run_by = call
        .str("run_by")
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or(DEFAULT_RUN_BY);
    let at = now(call)?;
    let graph = call.graph();

    let plans: Vec<&GraphNode> = graph
        .nodes_of_kind("plan")
        .filter(|p| target(graph, &p.id, "phase") == Some(phase.id.as_str()))
        .collect();
    let accepted: BTreeSet<&str> = graph
        .nodes_of_kind("accepted_risk")
        .filter(|r| r.fields.contains_key("at"))
        .filter_map(|r| target(graph, &r.id, "threat"))
        .collect();
    let held: Vec<Held> = graph
        .nodes_of_kind("threat")
        .filter(|t| {
            target(graph, &t.id, "plan").is_some_and(|p| plans.iter().any(|plan| plan.id == p))
        })
        .map(|node| Held {
            accepted: accepted.contains(node.id.as_str()),
            node,
        })
        .collect();

    // every change, validated before the first op is built: (threat id, new status, evidence) and the new threats
    let mut status: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
    let mut created: Vec<(String, &GraphNode, Value)> = Vec::new();
    for item in named {
        let id = text(item, "id").ok_or("verdict: every threat needs an \"id\"")?;
        let new_status = match text(item, "status") {
            Some(s @ ("open" | "closed")) => s,
            _ => {
                return Err(format!(
                    "threat {id}: status must be \"open\" or \"closed\""
                ));
            }
        };
        let evidence = text(item, "evidence").map(str::to_string);
        let matches: Vec<&Held> = held
            .iter()
            .filter(|h| name_of(&h.node.id) == id || h.node.id == id)
            .collect();
        match matches.as_slice() {
            [one] if !one.accepted => {
                status.insert(one.node.id.clone(), (new_status.to_string(), evidence));
            }
            [_] => {}
            [] if item["retroactive"] == true => {
                let plan_name = text(item, "plan")
                    .ok_or_else(|| format!("threat {id}: a retroactive threat needs its 'plan'"))?;
                let plan = plans
                    .iter()
                    .find(|p| name_of(&p.id) == plan_name || p.id == plan_name)
                    .ok_or_else(|| {
                        format!("threat {id}: plan '{plan_name}' is not a plan of phase {number}")
                    })?;
                let mut fields = json!({
                    "plan": plan.id,
                    "category": choice(item, "category", CATEGORY)?,
                    "component": text(item, "component").ok_or_else(|| format!("threat {id}: a retroactive threat needs 'component'"))?,
                    "severity": choice(item, "severity", SEVERITY)?,
                    "disposition": choice(item, "disposition", DISPOSITION)?,
                    "mitigation": text(item, "mitigation").ok_or_else(|| format!("threat {id}: a retroactive threat needs 'mitigation'"))?,
                    "status": new_status,
                    "retroactive": true,
                });
                if let Some(e) = &evidence {
                    fields["evidence"] = json!(e);
                }
                created.push((id.to_string(), plan, fields));
            }
            [] => {
                return Err(format!(
                    "threat {id} is in no plan of phase {number}: record it with \"retroactive\": true and the \"plan\" to put it in"
                ));
            }
            _ => {
                return Err(format!(
                    "threat {id} is claimed by more than one plan of phase {number}: renumber one first"
                ));
            }
        }
    }

    // the counts after the change
    let block_on = Settings::current()
        .string("workflow.security_block_on")
        .unwrap_or("high")
        .to_string();
    let (mut total, mut closed, mut blocking) = (0_i64, 0_i64, 0_i64);
    let mut count = |current: &str, severity: &str, resolved: bool| {
        total += 1;
        if current == "closed" || resolved {
            closed += 1;
        } else if crate::passes::blocks(severity, &block_on) {
            blocking += 1;
        }
    };
    for h in &held {
        let current = status
            .get(&h.node.id)
            .map_or_else(|| h.node.text("status").unwrap_or("open"), |s| s.0.as_str());
        count(current, h.node.text("severity").unwrap_or(""), h.accepted);
    }
    for (_, _, fields) in &created {
        count(
            fields["status"].as_str().unwrap_or("open"),
            fields["severity"].as_str().unwrap_or(""),
            false,
        );
    }
    let open = total - closed;

    let existing = graph
        .nodes_of_kind("security_audit")
        .find(|a| target(graph, &a.id, "phase") == Some(phase.id.as_str()));
    let audit_id = existing.map_or_else(
        || format!("{}/{number}-SECURITY", phase.id),
        |a| a.id.clone(),
    );
    let runs: Vec<&GraphNode> = graph
        .nodes_of_kind("audit_run")
        .filter(|r| target(graph, &r.id, "audit") == Some(audit_id.as_str()))
        .collect();
    let int = |n: &GraphNode, key: &str| {
        n.fields
            .get(key)
            .and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok()))
    };
    let last = runs
        .iter()
        .max_by_key(|r| super::support::natural(name_of(&r.id)));
    let changed = last.is_none_or(|r| {
        (int(r, "total"), int(r, "closed"), int(r, "open"))
            != (Some(total), Some(closed), Some(open))
    });

    let mut plan = call.plan();
    let file = audit_file(graph, phase, &plans);
    if existing.is_none() {
        let level = Settings::current()
            .integer("workflow.security_asvs_level")
            .unwrap_or(1);
        plan.create(
            "security_audit",
            &format!("{number}-SECURITY"),
            &format!("Phase {number} security"),
            &file,
            BTreeMap::from([
                ("phase".to_string(), json!(phase.id)),
                ("asvs_level".to_string(), json!(level)),
                ("created".to_string(), json!(at)),
            ]),
            Vec::new(),
        );
    }
    for (id, (new_status, evidence)) in &status {
        plan.set(id, "status", new_status.as_str());
        if let Some(e) = evidence {
            plan.set(id, "evidence", e.as_str());
        }
    }
    for (id, plan_node, fields) in &created {
        let Value::Object(map) = fields else { continue };
        plan.create(
            "threat",
            id,
            &format!("Threat {id}"),
            plan_node.file.as_deref().unwrap_or(&file),
            map.clone().into_iter().collect(),
            Vec::new(),
        );
    }
    if changed {
        let name = next_id(graph, "audit_run", &format!("{audit_id}/"), "AU");
        plan.create(
            "audit_run",
            &name,
            &format!("Audit run {name}"),
            &file,
            BTreeMap::from([
                ("audit".to_string(), json!(audit_id)),
                ("at".to_string(), json!(at)),
                ("total".to_string(), json!(total)),
                ("closed".to_string(), json!(closed)),
                ("open".to_string(), json!(open)),
                ("run_by".to_string(), json!(run_by)),
            ]),
            Vec::new(),
        );
    }
    let human = format!(
        "phase {number}: {total} threat(s), {closed} closed, {open} open, {blocking} blocking; {}\n",
        if changed {
            "audit run appended"
        } else {
            "no new audit run: the counts did not change"
        }
    );
    Ok(answer(
        call,
        json!({ "appended": changed, "threats_open": blocking }),
        human,
    )
    .with_plan(plan.finish()))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("record_security_audit", |cmd| {
        cmd.description(
            "Record the security auditor's verdict for a phase: the audit, each threat's status, and a run when the counts changed",
        )
        .writes()
        .arg("phase", |a| {
            a.required().description("the phase (its id, or its number)");
        })
        .arg("verdict", |a| {
            a.description("the verdict as JSON: {\"threats\": [{\"id\", \"status\", \"evidence\"?, \"retroactive\"?, \"plan\"?, ...}]}");
        })
        .arg("run_by", |a| {
            a.description("who ran the audit (default gsd-security-auditor)");
        })
        .handler(|call| run(call, record));
    });
}
