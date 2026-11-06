//! The commands of a plan's execution (D-r41-3): `start_plan` opens a plan at the host's clock and records that time in
//! its `plan_start`, and `complete_plan` writes its summary's two times. Upstream keeps `PLAN_START_TIME` in the
//! executor's shell; a shell does not outlive a fence on every host, so here the graph keeps it.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, node, now, required, run, text};

/// The time `plan` was opened: its `plan_start`'s `at`.
fn opened_at<'a>(call: &CommandCall<'a>, plan: &str) -> Option<&'a str> {
    call.graph()
        .node(&format!("{plan}/START"))?
        .fields
        .get("at")?
        .as_str()
}

/// `start_plan <plan>`: gated. Records the host's `now` in the plan's `plan_start`, once: a second call answers the
/// first value and writes nothing, so re-running the step never makes a false duration.
fn start(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let plan = node(call, "plan", required(call, "plan")?)?;
    if let Some(first) = opened_at(call, &plan.id) {
        return Ok(answer(
            call,
            json!({ "plan": plan.id, "started": first }),
            format!("plan {} may go on; started {first}\n", plan.id),
            None,
        ));
    }
    let started = now(call)?;
    let file = plan.file.as_deref().unwrap_or("spec/quick.spec");
    let mut edit = call.plan();
    edit.create(
        "plan_start",
        "START",
        "Plan opened",
        file,
        fields([("plan", json!(plan.id)), ("at", json!(started))]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "plan": plan.id, "started": started }),
        format!("plan {} may start; started {started}\n", plan.id),
        Some(&edit),
    ))
}

/// `complete_plan <summary>`: both times of the summary, in one plan: `started` from its `plan_start`, `completed` the
/// host's.
fn complete(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let summary = node(call, "summary", required(call, "summary")?)?;
    let completed = now(call)?;
    if summary.has_field("completed") {
        return Err(format!("summary '{}' is already completed", summary.id));
    }
    let plan = call
        .graph()
        .edges_from(&summary.id)
        .into_iter()
        .find(|edge| edge.label == "plan")
        .map(|edge| edge.target.as_str())
        .ok_or_else(|| format!("summary '{}' names no plan", summary.id))?;
    let started = opened_at(call, plan).ok_or_else(|| {
        format!("plan '{plan}' has no start recorded: run `specforge gsd start-plan {plan}` first")
    })?;
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

/// `answer_checkpoint <checkpoint> --by <who> [--option] [--text]`: the reply a person records, with
/// the host's clock. `--by` is what the person types at their terminal.
fn answer_checkpoint(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let checkpoint = node(call, "checkpoint", required(call, "checkpoint")?)?;
    let (option, reply) = (text(call, "option")?, text(call, "text")?);
    let by = required(call, "by")?;
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
    for (name, member) in [("option", option), ("text", reply), ("by", Some(by))] {
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

/// `confirm_setup <user_setup> --by <who>`: the caller records that the person did the setup.
fn confirm_setup(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let setup = node(call, "user_setup", required(call, "user_setup")?)?;
    let by = required(call, "by")?;
    if setup.has_field("done") {
        return Err(format!("user setup '{}' is already confirmed", setup.id));
    }
    let at = now(call)?;
    let mut value = serde_json::Map::new();
    value.insert("at".to_string(), json!(at));
    value.insert("by".to_string(), json!(by));
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
            .human()
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
                a.description("required; who answered (typed by the person at their terminal)");
            })
            .handler(|call| run(call, answer_checkpoint));
    });
    c.command("confirm_setup", |cmd| {
        cmd.description("Record that a human did a plan's user setup step, with the host's clock")
            .human()
            .arg("user_setup", |a| {
                a.required().description(
                    "the user setup (its path, or a short form that is not ambiguous)",
                );
            })
            .arg("by", |a| {
                a.description("required; who did it (typed by the person at their terminal)");
            })
            .handler(|call| run(call, confirm_setup));
    });
    c.command("start_plan", |cmd| {
        cmd.description("Open a plan: runs the plan's entry gates (W318, E516) and records the host's clock in its `plan_start`, once; a second call answers the first value")
            .writes()
            .gate("W318")
            .gate("E516")
            .subject("plan")
            .arg("plan", |a| {
                a.required().description("the plan about to run (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, start));
    });
    c.command("complete_plan", |cmd| {
        cmd.description("Close a plan: write its summary's `started` (what `start-plan` recorded) and `completed` (the host's clock); the summary must exist with its required fields")
            .writes()
            .gate("W404")
            .subject("summary")
            .arg("summary", |a| {
                a.required().description("the plan's summary (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, complete));
    });
}
