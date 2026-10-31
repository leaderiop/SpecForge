//! The commands of the broken-windows ledger (D-r41-1 rows 11-13). The waiver, the fix and the time a window was
//! recorded are immutable harness values: a window is resolved once, and the command refuses a second resolution
//! before the host's `I` check would (and before gsd/E513 would report both).

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, fields, next_id, node, now, ops_file, required, run, text, title};

const KINDS: &[&str] = &[
    "stub",
    "todo",
    "fixme",
    "skipped_test",
    "lint_warning",
    "unmet_truth",
    "unrun_verify",
    "deviation",
];

/// `record_window <kind> <phase> <description> [--file] [--line] [--refactor]`: a window, `BWn` in the project.
fn record(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let kind = required(call, "kind")?;
    let phase = node(call, "phase", required(call, "phase")?)?;
    let description = required(call, "description")?;
    let file = text(call, "file")?;
    let proposal = call
        .str("refactor")
        .map(|r| node(call, "refactor_proposal", r))
        .transpose()?;
    let at = now(call)?;
    let name = next_id(call.graph(), "broken_window", "", "BW");
    // Integers go as decimal text: the host's plan op reads a top-level integer from text.
    let line = call.count("line").map(|n| json!(n.to_string()));
    let mut plan = call.plan();
    plan.create(
        "broken_window",
        &name,
        &title(description),
        &ops_file(call.graph(), Some(&phase.id)),
        fields([
            ("kind", json!(kind)),
            ("phase", json!(phase.id)),
            ("description", json!(description)),
            ("file", file.map_or(Value::Null, |f| json!(f))),
            ("line", line.unwrap_or(Value::Null)),
            ("refactor", proposal.map_or(Value::Null, |r| json!(r.id))),
            ("recorded_at", json!(at)),
        ]),
        Vec::new(),
    );
    Ok(answer(
        call,
        json!({ "wrote": [name], "recorded_at": at }),
        format!("broken window {name} recorded at {at}\n"),
        Some(&plan),
    ))
}

/// A window that is open: not waived, not fixed.
fn open_window<'a>(
    call: &CommandCall<'a>,
    arg: &str,
    doing: &str,
) -> Result<&'a GraphNode, String> {
    let window = node(call, "broken_window", required(call, arg)?)?;
    // The status is read from the authored fields, as the derive pass does, so the command and the pass agree. A
    // window whose refactor proposal is disposed is resolved: accepted fixes it, declined waives it.
    let outcome = window
        .text("refactor")
        .and_then(|r| call.graph().node(r))
        .and_then(|p| p.fields.get("disposition"))
        .and_then(|d| d["outcome"].as_str());
    if window.has_field("waiver") || window.has_field("fixed_at") || outcome.is_some() {
        let now = if window.has_field("waiver") || outcome == Some("declined") {
            "waived"
        } else {
            "fixed"
        };
        return Err(format!(
            "broken window '{}' is already {now}: it cannot be {doing}",
            window.id
        ));
    }
    Ok(window)
}

/// `waive_window <window> <reason> <by>`.
fn waive(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let window = open_window(call, "window", "waived")?;
    let (reason, by) = (required(call, "reason")?, required(call, "by")?);
    let at = now(call)?;
    let mut plan = call.plan();
    plan.set(
        &window.id,
        "waiver",
        json!({ "reason": reason, "by": by, "at": at }),
    );
    Ok(answer(
        call,
        json!({ "wrote": [window.id], "waiver": { "reason": reason, "by": by, "at": at } }),
        format!("broken window {} waived by {by} at {at}\n", window.id),
        Some(&plan),
    ))
}

/// `fix_window <window>`.
fn fix(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let window = open_window(call, "window", "fixed")?;
    let at = now(call)?;
    let mut plan = call.plan();
    plan.set(&window.id, "fixed_at", at);
    Ok(answer(
        call,
        json!({ "wrote": [window.id], "fixed_at": at }),
        format!("broken window {} fixed at {at}\n", window.id),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("record_window", |cmd| {
        cmd.description(
            "Record a broken window: a defect left in the code, open until it is fixed or waived",
        )
        .writes()
        .arg("kind", |a| {
            a.required()
                .one_of(KINDS)
                .description("what kind of defect");
        })
        .arg("phase", |a| {
            a.required().description("the phase that found it");
        })
        .arg("description", |a| {
            a.required()
                .description("what is wrong, in the person's words");
        })
        .arg("file", |a| {
            a.description("the file it is in");
        })
        .arg("line", |a| {
            a.count().description("the line it is on");
        })
        .arg("refactor", |a| {
            a.description(
                "the refactor proposal a strict-mode trigger recorded it as a deviation of",
            );
        })
        .handler(|call| run(call, record));
    });
    c.command("waive_window", |cmd| {
        cmd.description("Waive a broken window with a reason and who waives it, at the host's clock; a window is resolved once")
            .human()
            .arg("window", |a| {
                a.required().description("the window (its id, or a short form that is not ambiguous)");
            })
            .arg("reason", |a| {
                a.required().description("why it is waived, in the person's words");
            })
            .arg("by", |a| {
                a.required().description("who waives it");
            })
            .handler(|call| run(call, waive));
    });
    c.command("fix_window", |cmd| {
        cmd.description(
            "Mark a broken window fixed, at the host's clock; a window is resolved once",
        )
        .writes()
        .arg("window", |a| {
            a.required()
                .description("the window (its id, or a short form that is not ambiguous)");
        })
        .handler(|call| run(call, fix));
    });
}
