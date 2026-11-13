//! The commands of a plan's execution (D-r41-3): `start_plan` opens a plan at the host's clock and records that time, and
//! the commit it opened at, in its `plan_start`, and `complete_plan` writes its summary's two times. Upstream keeps `PLAN_START_TIME` in the
//! executor's shell; a shell does not outlive a fence on every host, so here the graph keeps it.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, node, now, required, run, text};

/// `field` of `plan`'s `plan_start`: `at` is when it was opened, `head` the commit it opened at.
fn of_start<'a>(call: &CommandCall<'a>, plan: &str, field: &str) -> Option<&'a str> {
    call.graph()
        .node(&format!("{plan}/START"))?
        .fields
        .get(field)?
        .as_str()
}

/// `--head`: a commit id in full, or empty for an unborn branch, which has none: what git's own `HEAD` line names, never a
/// revision to resolve later.
fn head_given<'a>(call: &'a CommandCall<'_>) -> Result<Option<&'a str>, String> {
    let head = text(call, "head")?.filter(|head| !head.is_empty());
    let sha = |head: &str| {
        matches!(head.len(), 40 | 64)
            && head
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    match head {
        Some(head) if !sha(head) => Err(format!(
            "head must be a full lowercase commit id, got '{head}': pass `specforge git head --pick .sha`"
        )),
        _ => Ok(head),
    }
}

/// The route whose pack `start-plan` records on its plan, each call.
const EXECUTOR: &str = "gsd/executor";

/// `start_plan <plan> [--head <sha>] [--max-tokens <n>]`: gated. Records the host's `now` and the commit in the plan's
/// `plan_start`, each once: a second call answers the first values and leaves them, so re-running the step never makes
/// a false duration or a false commit count. A prompt's step opens a plan with no head and the executor names it next,
/// so a head comes once whichever call brings it. Every call records the plan's executor pack, so the continuation
/// after a checkpoint re-records what the executor reads and silences `gsd/W524`.
fn start(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let plan = node(call, "plan", required(call, "plan")?)?;
    let head = head_given(call)?;
    let max_tokens = call
        .count("max_tokens")
        .map(|n| u32::try_from(n).map_err(|_| format!("'max_tokens' is above {}", u32::MAX)))
        .transpose()?;
    if let Some(first) = of_start(call, &plan.id, "at") {
        let kept = of_start(call, &plan.id, "head");
        let mut edit = call.plan();
        let fills = kept.is_none() && head.is_some();
        if let (true, Some(head)) = (fills, head) {
            edit.set(&format!("{}/START", plan.id), "head", head);
        }
        edit.record_context(&plan.id, EXECUTOR, max_tokens);
        return Ok(answer(
            call,
            json!({ "plan": plan.id, "started": first, "head": kept.or(head), "recorded": EXECUTOR }),
            format!("plan {} may go on; started {first}\n", plan.id),
            Some(&edit),
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
        fields([
            ("plan", json!(plan.id)),
            ("at", json!(started)),
            ("head", json!(head)),
        ]),
        Vec::new(),
    );
    edit.record_context(&plan.id, EXECUTOR, max_tokens);
    Ok(answer(
        call,
        json!({ "plan": plan.id, "started": started, "head": head, "recorded": EXECUTOR }),
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
    let started = of_start(call, plan, "at").ok_or_else(|| {
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
        cmd.description("Open a plan: runs the plan's entry gates (W318, E516), records the host's clock and the commit it opens at in its `plan_start`, each once (a second call answers the first values), and on every call records the plan's gsd/executor pack in its `context_pack` (gsd/W524)")
            .writes()
            .gate("W318")
            .gate("E516")
            .subject("plan")
            .arg("plan", |a| {
                a.required().description("the plan about to run (its path, or a short form that is not ambiguous)");
            })
            .arg("head", |a| {
                a.description("the commit it opens at, in full (`specforge git head --pick .sha`); empty on an unborn branch, which has none");
            })
            .arg("max_tokens", |a| {
                a.count().description("the budget of the executor pack it records (default: the route's own)");
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
