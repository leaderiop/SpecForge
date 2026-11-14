//! `graduate_lessons`: what the developer chose for a cluster of lessons in `/gsd-graduation` (upstream's
//! `**Graduated:**` mark and `graduation_backlog` rows), recorded on each lesson as `graduated`, with the host's clock.
//! A promotion names the decision the lessons became, a deferral the phase to look again after.

use std::collections::BTreeSet;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, now, required, run, texts};

const DISPOSITIONS: &[&str] = &["promoted", "deferred", "dismissed"];

/// `graduate_lessons <json list of lessons> --disposition <promoted|deferred|dismissed> [--target] [--until]`.
fn graduate_lessons(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let names = texts(call, "lessons")?.ok_or("'lessons' is required")?;
    if names.is_empty() {
        return Err("lessons: the list names no lesson".to_string());
    }
    let disposition = required(call, "disposition")?;
    if !DISPOSITIONS.contains(&disposition) {
        return Err(format!(
            "--disposition '{disposition}' is not one of {}",
            DISPOSITIONS.join(", ")
        ));
    }
    let target = call
        .str("target")
        .map(|d| node(call, "decision", d))
        .transpose()?;
    let until = call
        .str("until")
        .map(|p| node(call, "phase", p))
        .transpose()?;
    match (disposition, target.is_some(), until.is_some()) {
        ("promoted", false, _) => return Err("a promotion names its decision: --target".into()),
        ("deferred", _, false) => return Err("a deferral names its phase: --until".into()),
        ("promoted" | "dismissed", _, true) => {
            return Err(format!(
                "--until belongs to a deferral, not '{disposition}'"
            ));
        }
        ("deferred" | "dismissed", true, _) => {
            return Err(format!(
                "--target belongs to a promotion, not '{disposition}'"
            ));
        }
        _ => {}
    }
    let at = now(call)?;
    let mut seen = BTreeSet::new();
    let mut edit = call.plan();
    let mut wrote = Vec::new();
    for name in &names {
        let lesson = node(call, "lesson", name)?;
        if !seen.insert(lesson.id.as_str()) {
            return Err(format!("lessons: '{name}' is named twice"));
        }
        let held = lesson
            .fields
            .get("graduated")
            .and_then(|g| g.get("disposition"))
            .and_then(Value::as_str);
        if let Some(final_word @ ("promoted" | "dismissed")) = held {
            return Err(format!("lesson '{}' is already {final_word}", lesson.id));
        }
        let mut graduated = json!({ "disposition": disposition, "at": at });
        if let Some(decision) = target {
            graduated["target"] = json!(decision.id);
        }
        if let Some(phase) = until {
            graduated["until"] = json!(phase.id);
        }
        edit.set(&lesson.id, "graduated", graduated);
        wrote.push(lesson.id.clone());
    }
    Ok(answer(
        call,
        json!({ "wrote": wrote, "disposition": disposition, "at": at }),
        format!("{} lesson(s) {disposition} at {at}\n", wrote.len()),
        Some(&edit),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("graduate_lessons", |cmd| {
        cmd.description("Record what the developer chose for a cluster of lessons: promoted into a decision, deferred until a phase, or dismissed (the host's clock)")
            .writes()
            .arg("lessons", |a| {
                a.required().description("the lessons of the cluster, as a JSON list of ids");
            })
            .arg("disposition", |a| {
                a.one_of(DISPOSITIONS).description("required; promoted, deferred or dismissed");
            })
            .arg("target", |a| {
                a.description("a promotion's decision: the decision the lessons became");
            })
            .arg("until", |a| {
                a.description("a deferral's phase: the lessons are open again once it is complete");
            })
            .handler(|call| run(call, graduate_lessons));
    });
}
