//! The commands of a plan's execution (D-r41-3): `start_plan` opens a plan at the host's clock, and `complete_plan`
//! writes its summary's two times. Upstream keeps `PLAN_START_TIME` in the executor's shell; here the executor keeps
//! the time `start_plan` answered and hands it to `complete_plan --started`.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, now, required, run};
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

pub fn declare(c: &mut ContributionsBuilder) {
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
