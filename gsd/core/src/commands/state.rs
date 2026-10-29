//! The commands of STATE.md (D-r41-1 rows 7-10): a pause and its handoff, and the blockers. `at`, `by` and `raised` are
//! the host's clock and the caller's name; the person's words are refused if they hold a control character.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, next_id, node, now, ops_file, required, run, title};

/// `pause <reason> [--phase] [--plan] [--handoff] [--by]`: a pause, `PSn` in its phase (or the project).
fn pause(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let reason = required(call, "reason")?;
    let by = required(call, "by")?;
    let phase = call
        .str("phase")
        .map(|p| node(call, "phase", p))
        .transpose()?;
    let plan = call
        .str("plan")
        .map(|p| node(call, "plan", p))
        .transpose()?;
    let handoff = call
        .str("handoff")
        .map(|h| node(call, "handoff", h))
        .transpose()?;
    let at = now(call)?;
    let scope = phase.map_or(String::new(), |p| format!("{}/", p.id));
    let name = next_id(call.graph(), "pause", &scope, "PS");
    let id = format!("{scope}{name}");
    let file = ops_file(call.graph(), phase.map(|p| p.id.as_str()));
    let mut plan_builder = call.plan();
    plan_builder.create(
        "pause",
        &name,
        &title(reason),
        &file,
        fields([
            ("phase", phase.map_or(Value::Null, |p| json!(p.id))),
            ("plan", plan.map_or(Value::Null, |p| json!(p.id))),
            ("handoff", handoff.map_or(Value::Null, |h| json!(h.id))),
            ("reason", json!(reason)),
            ("by", json!(by)),
            ("at", json!(at)),
        ]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "wrote": [id], "at": at }),
        format!("paused: {id} at {at}\n"),
        Some(&plan_builder),
    ))
}

/// `resume <pause>`: the pause goes, and its handoff with it unless another pause still names it.
fn resume(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let pause = node(call, "pause", required(call, "pause")?)?;
    let mut plan = call.plan();
    let mut wrote = vec![pause.id.clone()];
    plan.delete(&pause.id, true);
    if let Some(handoff) = pause.text("handoff") {
        let shared =
            call.graph().nodes().iter().any(|n| {
                n.kind == "pause" && n.id != pause.id && n.text("handoff") == Some(handoff)
            });
        if !shared {
            plan.delete(handoff, true);
            wrote.push(handoff.to_string());
        }
    }
    Ok(answer(
        call,
        json!({ "wrote": wrote }),
        format!("resumed: removed {}\n", wrote.join(", ")),
        Some(&plan),
    ))
}

/// `raise_blocker <text> [--phase] [--plan]`: a blocker, `BLn` in the project.
fn raise_blocker(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let words = required(call, "text")?;
    let phase = call
        .str("phase")
        .map(|p| node(call, "phase", p))
        .transpose()?;
    let plan = call
        .str("plan")
        .map(|p| node(call, "plan", p))
        .transpose()?;
    let raised = now(call)?;
    let name = next_id(call.graph(), "blocker", "", "BL");
    let anchor = plan.or(phase).map(|n| n.id.as_str());
    let mut plan_builder = call.plan();
    plan_builder.create(
        "blocker",
        &name,
        &title(words),
        &ops_file(call.graph(), anchor),
        fields([
            ("text", json!(words)),
            ("phase", phase.map_or(Value::Null, |p| json!(p.id))),
            ("plan", plan.map_or(Value::Null, |p| json!(p.id))),
            ("raised", json!(raised)),
        ]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "wrote": [name], "raised": raised }),
        format!("blocker {name} raised at {raised}\n"),
        Some(&plan_builder),
    ))
}

/// `resolve_blocker <blocker> <how>`: how and when it was resolved. The time is authored, so E107 checks it.
fn resolve_blocker(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let blocker = node(call, "blocker", required(call, "blocker")?)?;
    let how = required(call, "how")?;
    if blocker.has_field("resolved") {
        return Err(format!("blocker '{}' is already resolved", blocker.id));
    }
    let at = now(call)?;
    let mut plan = call.plan();
    plan.set(&blocker.id, "resolved", json!({ "at": at, "how": how }));
    Ok(answer(
        call,
        json!({ "wrote": [blocker.id], "resolved": { "at": at, "how": how } }),
        format!("blocker {} resolved at {at}\n", blocker.id),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("pause", |cmd| {
        cmd.description("Pause the work for a handoff: records why, who and when (the host's clock), in a phase or the project")
            .writes()
            .arg("reason", |a| {
                a.required().description("why the work stopped, in the person's words");
            })
            .arg("phase", |a| {
                a.description("the phase the work was in");
            })
            .arg("plan", |a| {
                a.description("the plan that was open");
            })
            .arg("handoff", |a| {
                a.description("the handoff entity that carries what the next session needs");
            })
            .arg("by", |a| {
                a.description("required; who paused: the caller's attestation, not a proof of the person (K19)");
            })
            .handler(|call| run(call, pause));
    });
    c.command("resume", |cmd| {
        cmd.description("Resume paused work: removes the pause and its handoff (a handoff another pause still names stays)")
            .writes()
            .arg("pause", |a| {
                a.required().description("the pause (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, resume));
    });
    c.command("raise_blocker", |cmd| {
        cmd.description("Raise a blocker or concern, with the host's clock")
            .writes()
            .arg("text", |a| {
                a.required()
                    .description("the blocker, in the person's words");
            })
            .arg("phase", |a| {
                a.description("the phase it blocks");
            })
            .arg("plan", |a| {
                a.description("the plan it blocks");
            })
            .handler(|call| run(call, raise_blocker));
    });
    c.command("resolve_blocker", |cmd| {
        cmd.description("Resolve a blocker: how it was resolved, and when (the host's clock)")
            .writes()
            .arg("blocker", |a| {
                a.required()
                    .description("the blocker (its id, or a short form that is not ambiguous)");
            })
            .arg("how", |a| {
                a.required()
                    .description("how it was resolved, in the person's words");
            })
            .handler(|call| run(call, resolve_blocker));
    });
}
