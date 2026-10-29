//! `close_milestone` (D-r41-7): the write behind `/gsd-complete-milestone`. Its gate (E106, E511, E516) has already
//! proved the milestone's latest audit passes and its phases are verified; the command records the completion, names
//! the audit it closed on and, when the milestone has none, creates its retrospective.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, json as json_arg, node, required, run};

const LISTS: [&str; 4] = ["built", "worked", "inefficient", "patterns"];

/// The number a name ends in: `A10` is after `A9`.
fn number(id: &str) -> u64 {
    let digits: String = id
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse().unwrap_or(0)
}

fn close(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let milestone = node(call, "milestone", required(call, "milestone")?)?;
    if milestone.has_field("completion") {
        return Err(format!("milestone '{}' is already closed", milestone.id));
    }
    let audit = call
        .graph()
        .nodes()
        .iter()
        .filter(|n| n.kind == "milestone_audit" && n.text("milestone") == Some(&milestone.id))
        .max_by_key(|n| number(&n.id))
        .ok_or_else(|| format!("milestone '{}' has no audit", milestone.id))?;
    let today = call.today();
    let mut plan = call.plan();
    plan.set(
        &milestone.id,
        "completion",
        json!({ "at": today, "audit": audit.id }),
    );
    let name = format!(
        "{}-RETROSPECTIVE",
        milestone.id.rsplit('/').next().unwrap_or(&milestone.id)
    );
    let retrospective = format!("{}/{name}", milestone.id);
    let creates = call.graph().node(&retrospective).is_none();
    if creates {
        let mut members = Vec::new();
        for list in LISTS {
            let items = json_arg(call, list, Vec::<String>::new(), |t| {
                serde_json::from_str(t)
            })?;
            members.push((list, json!(items)));
        }
        let mut all = vec![("milestone", json!(milestone.id))];
        all.extend(members.into_iter().filter(|(_, v)| v != &json!([])));
        let file = milestone.file.clone().unwrap_or_default();
        plan.create(
            "retrospective",
            &name,
            &format!("Retrospective of {}", milestone.id),
            &file,
            all.into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<std::collections::BTreeMap<String, Value>>(),
            Vec::new(),
        );
    }
    let mut wrote = vec![milestone.id.clone()];
    if creates {
        wrote.push(retrospective.clone());
    }
    Ok(answer(
        call,
        json!({ "wrote": wrote, "completion": { "at": today, "audit": audit.id },
                "archived": false }),
        format!(
            "milestone {} closed on {today} by audit {}{}\nnothing is archived: a shipped milestone's entities stay where they are\n",
            milestone.id,
            audit.id,
            if creates {
                format!("; {retrospective} created")
            } else {
                String::new()
            },
        ),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("close_milestone", |cmd| {
        cmd.description("Close a milestone whose audit passes and whose phases are verified: record its completion against the latest audit and create its retrospective if it has none")
            .writes()
            .gate("E106")
            .gate("E511")
            .gate("E516")
            .subject("milestone")
            .arg("milestone", |a| {
                a.required().description("the milestone (its id, or a short form that is not ambiguous)");
            })
            .arg("built", |a| {
                a.description("what was built, as a JSON list of strings, for a retrospective this creates");
            })
            .arg("worked", |a| {
                a.description("what worked, as a JSON list of strings");
            })
            .arg("inefficient", |a| {
                a.description("what was inefficient, as a JSON list of strings");
            })
            .arg("patterns", |a| {
                a.description("the patterns established, as a JSON list of strings");
            })
            .handler(|call| run(call, close));
    });
}
