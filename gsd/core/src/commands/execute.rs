//! The commands of a plan's execution (D-r41-3): `start_plan` opens a plan at the host's clock, and `complete_plan`
//! writes its summary's two times. Upstream keeps `PLAN_START_TIME` in the executor's shell; here the executor keeps
//! the time `start_plan` answered and hands it to `complete_plan --started`.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, now, required, run, text};
use crate::time;

/// `start_plan <plan>`: gated, writes nothing, answers the host's `now`. No summary exists yet, and one made without
/// its required fields would be refused, so the start time is written by `complete_plan`.
fn start(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let plan = node(call, "plan", required(call, "plan")?)?;
    let started = now(call)?;
    Ok(answer(
        call,
        json!({ "plan": plan.id, "started": started }),
        format!(
            "plan {} may start; started {started}\nkeep the time and pass it to `complete-plan --started`\n",
            plan.id
        ),
        None,
    ))
}

/// `complete_plan <summary> --started <ts>`: both times of the summary, in one plan.
fn complete(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let summary = node(call, "summary", required(call, "summary")?)?;
    let completed = now(call)?;
    let Some(started) = call.str("started") else {
        return Err("--started is required: the time `start-plan` answered".to_string());
    };
    if summary.has_field("completed") {
        return Err(format!("summary '{}' is already completed", summary.id));
    }
    let Some(began) =
        time::seconds(started).filter(|_| started.ends_with('Z') && !started.contains('.'))
    else {
        return Err(format!(
            "--started '{started}' is not an RFC 3339 UTC time (YYYY-MM-DDTHH:MM:SSZ)"
        ));
    };
    if time::seconds(completed).is_some_and(|end| began > end) {
        return Err(format!("--started '{started}' is after now ({completed})"));
    }
    let mut plan = call.plan();
    plan.set(&summary.id, "started", started)
        .set(&summary.id, "completed", completed);
    Ok(answer(
        call,
        json!({ "wrote": [summary.id], "started": started, "completed": completed }),
        format!(
            "summary {} started {started}, completed {completed}\n",
            summary.id
        ),
        Some(&plan),
    ))
}

/// `answer_checkpoint <checkpoint> [--option] [--text] [--by]`: the human's reply, with the host's clock.
fn answer_checkpoint(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let checkpoint = node(call, "checkpoint", required(call, "checkpoint")?)?;
    let (option, reply, by) = (
        text(call, "option")?,
        text(call, "text")?,
        text(call, "by")?,
    );
    if checkpoint.has_field("answer") {
        return Err(format!(
            "checkpoint '{}' is already answered",
            checkpoint.id
        ));
    }
    if option.is_none() && reply.is_none() {
        return Err("an answer needs --option or --text".to_string());
    }
    if let Some(option) = option {
        let names: Vec<&str> = checkpoint
            .fields
            .get("options")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter_map(|r| r["name"].as_str()).collect())
            .unwrap_or_default();
        if !names.contains(&option) {
            return Err(format!(
                "--option '{option}' is no option of checkpoint '{}' ({})",
                checkpoint.id,
                if names.is_empty() {
                    "it has none".to_string()
                } else {
                    names.join(", ")
                }
            ));
        }
    }
    let at = now(call)?;
    let mut value = serde_json::Map::new();
    for (name, member) in [("option", option), ("text", reply), ("by", by)] {
        if let Some(member) = member {
            value.insert(name.to_string(), json!(member));
        }
    }
    value.insert("at".to_string(), json!(at));
    let mut plan = call.plan();
    plan.set(&checkpoint.id, "answer", Value::Object(value.clone()));
    Ok(answer(
        call,
        json!({ "wrote": [checkpoint.id], "answer": value }),
        format!("checkpoint {} answered at {at}\n", checkpoint.id),
        Some(&plan),
    ))
}

/// `confirm_setup <user_setup> [--by]`: the human did the setup.
fn confirm_setup(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let setup = node(call, "user_setup", required(call, "user_setup")?)?;
    let by = text(call, "by")?;
    if setup.has_field("done") {
        return Err(format!("user setup '{}' is already confirmed", setup.id));
    }
    let at = now(call)?;
    let mut value = serde_json::Map::new();
    value.insert("at".to_string(), json!(at));
    if let Some(by) = by {
        value.insert("by".to_string(), json!(by));
    }
    let mut plan = call.plan();
    plan.set(&setup.id, "done", Value::Object(value.clone()));
    Ok(answer(
        call,
        json!({ "wrote": [setup.id], "done": value }),
        format!("user setup {} done at {at}\n", setup.id),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("answer_checkpoint", |cmd| {
        cmd.description("Record a human's answer to a checkpoint of a plan, with the host's clock: a decision's option, or free text")
            .writes()
            .arg("checkpoint", |a| {
                a.required().description("the checkpoint (its path, or a short form that is not ambiguous)");
            })
            .arg("option", |a| {
                a.description("the name of one of a decision checkpoint's options");
            })
            .arg("text", |a| {
                a.description("the reply in the person's words, kept verbatim");
            })
            .arg("by", |a| {
                a.description("who answered");
            })
            .handler(|call| run(call, answer_checkpoint));
    });
    c.command("confirm_setup", |cmd| {
        cmd.description("Record that a human did a plan's user setup step, with the host's clock")
            .writes()
            .arg("user_setup", |a| {
                a.required().description(
                    "the user setup (its path, or a short form that is not ambiguous)",
                );
            })
            .arg("by", |a| {
                a.description("who did it");
            })
            .handler(|call| run(call, confirm_setup));
    });
    c.command("start_plan", |cmd| {
        cmd.description("Open a plan: runs the plan's entry gates (W318, E516), writes nothing and answers the host's clock, which the executor passes to `complete-plan --started`")
            .gate("W318")
            .gate("E516")
            .subject("plan")
            .arg("plan", |a| {
                a.required().description("the plan about to run (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, start));
    });
    c.command("complete_plan", |cmd| {
        cmd.description("Close a plan: write its summary's `started` and `completed` (the host's clock); the summary must exist with its required fields")
            .writes()
            .gate("W404")
            .subject("summary")
            .arg("summary", |a| {
                a.required().description("the plan's summary (its path, or a short form that is not ambiguous)");
            })
            .arg("started", |a| {
                a.description("the time `start-plan` answered, YYYY-MM-DDTHH:MM:SSZ (required)");
            })
            .handler(|call| run(call, complete));
    });
}
