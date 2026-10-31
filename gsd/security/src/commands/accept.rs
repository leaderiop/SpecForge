//! `accept_risk <threat> <rationale> <by>`: a person accepts a threat's risk. Upstream's workflow asks the person and
//! the agent records the answer ("Accept all open - document in accepted risks log"); here the person runs the command
//! at a terminal, as `waive_window` (`[accept-risk-human]`, ADR 0301, decision U4). An accepted threat never blocks
//! again and the audit's later runs leave it as it is.

use std::collections::BTreeMap;

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::record::audit_file;
use super::support::{answer, name_of, next_id, node, now, required, run, target};

fn accept(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let threat = node(call, "threat", required(call, "threat")?)?;
    let (rationale, by) = (
        required(call, "rationale")?.trim(),
        required(call, "by")?.trim(),
    );
    let at = now(call)?;
    let graph = call.graph();
    if let Some(done) = graph
        .nodes_of_kind("accepted_risk")
        .filter(|r| r.fields.contains_key("at"))
        .find(|r| target(graph, &r.id, "threat") == Some(threat.id.as_str()))
    {
        return Err(format!(
            "threat '{}' is already accepted as {}: one risk per threat",
            threat.id, done.id
        ));
    }
    let plan = target(graph, &threat.id, "plan")
        .ok_or_else(|| format!("threat '{}' names no plan", threat.id))?;
    let phase = target(graph, plan, "phase")
        .and_then(|p| graph.node(p))
        .ok_or_else(|| {
            format!(
                "threat '{}' is in a plan with no phase: a quick plan has no audit",
                threat.id
            )
        })?;
    let number = name_of(&phase.id);
    let existing = graph
        .nodes_of_kind("security_audit")
        .find(|a| target(graph, &a.id, "phase") == Some(phase.id.as_str()));
    let audit_id = existing.map_or_else(
        || format!("{}/{number}-SECURITY", phase.id),
        |a| a.id.clone(),
    );
    let mut edit = call.plan();
    if existing.is_none() {
        let plans: Vec<&GraphNode> = graph
            .nodes_of_kind("plan")
            .filter(|p| target(graph, &p.id, "phase") == Some(phase.id.as_str()))
            .collect();
        edit.create(
            "security_audit",
            &format!("{number}-SECURITY"),
            &format!("Phase {number} security"),
            &audit_file(graph, phase, &plans),
            BTreeMap::from([
                ("phase".to_string(), json!(phase.id)),
                (
                    "asvs_level".to_string(),
                    json!(
                        Settings::current()
                            .integer("workflow.security_asvs_level")
                            .unwrap_or(1)
                    ),
                ),
                ("created".to_string(), json!(at)),
            ]),
            Vec::new(),
        );
    }
    let name = next_id(graph, "accepted_risk", &format!("{audit_id}/"), "AR");
    let file = graph
        .node(&audit_id)
        .and_then(|a| a.file.clone())
        .unwrap_or_else(|| "spec/security.spec".to_string());
    let day = at.chars().take(10).collect::<String>();
    edit.create(
        "accepted_risk",
        &name,
        &format!("Accepted {}", name_of(&threat.id)),
        &file,
        BTreeMap::from([
            ("audit".to_string(), json!(audit_id)),
            ("threat".to_string(), json!(threat.id)),
            ("rationale".to_string(), json!(rationale)),
            ("accepted_by".to_string(), json!(by)),
            ("at".to_string(), json!(day)),
        ]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "wrote": [name], "threat": threat.id, "accepted_by": by, "at": day }),
        format!(
            "threat {} accepted by {by} on {day} as {name}\n",
            name_of(&threat.id)
        ),
    )
    .with_plan(edit.finish()))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("accept_risk", |cmd| {
        cmd.description("Accept a threat's risk with a rationale and who accepts it, at the host's clock; a person's record")
            .human()
            .arg("threat", |a| {
                a.required().description("the threat (its id, or a short form that is not ambiguous)");
            })
            .arg("rationale", |a| {
                a.required().description("why the risk is accepted, in the person's words");
            })
            .arg("by", |a| {
                a.required().description("who accepts it");
            })
            .handler(|call| run(call, accept));
    });
}
