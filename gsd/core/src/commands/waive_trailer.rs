//! `waive_trailer` records a person's waiver of gsd/E521 on a recorded commit, beside the fields gsd-git recorded,
//! which it never changes (ADR 0381). The waiver is an immutable harness value: a commit is waived once, and the
//! command refuses a second waiver before the host's `I` check would. It waives only a commit gsd/E521 judges, by the
//! rule the pass reads ([`unlinked`]), and a refusal names why the rule does not judge the commit ([`Unjudged`]).

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, now, required, run, target};
use crate::collectors::LinkedBy;
use crate::passes::commits::{CommitType, Unjudged, unlinked};

/// `waive_trailer <commit> <reason> <by>`.
fn waive(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let commit = node(call, "commit", required(call, "commit")?)?;
    if commit.has_field("waiver") {
        return Err(format!(
            "commit '{}' is already waived: a commit's waiver is recorded once",
            commit.id
        ));
    }
    let graph = call.graph();
    let read = |field: &str| commit.fields.get(field).and_then(Value::as_str);
    let judged = unlinked(
        target(graph, &commit.id, "plan"),
        target(graph, &commit.id, "task"),
        read("type").and_then(CommitType::parse),
        read("linked_by").and_then(LinkedBy::parse),
    );
    if let Err(unjudged) = judged {
        let why = match unjudged {
            Unjudged::NoPlan => "it names no plan",
            Unjudged::Docs => "it is a docs commit, a plan's metadata commit",
            Unjudged::NotByTitle => "a Gsd-Task trailer links its task",
            Unjudged::NoTaskType => "it names no task and its type is not a task type",
        };
        return Err(format!(
            "gsd/E521 does not judge commit '{}': {why}, so there is nothing to waive",
            commit.id
        ));
    }
    let (reason, by) = (required(call, "reason")?, required(call, "by")?);
    let at = now(call)?;
    // The commit already sits in its plan's file: `set` edits it there, beside what gsd-git recorded.
    let mut plan = call.plan();
    plan.set(
        &commit.id,
        "waiver",
        json!({ "reason": reason, "by": by, "at": at }),
    );
    Ok(answer(
        call,
        json!({ "wrote": [commit.id], "waiver": { "reason": reason, "by": by, "at": at } }),
        format!("commit {} waived by {by} at {at}\n", commit.id),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("waive_trailer", |cmd| {
        cmd.description(
            "Waive gsd/E521 on a recorded commit with a reason and who waives it, at the host's clock; the commit keeps \
             every field gsd-git recorded, and a waiver is recorded once",
        )
        .human()
        .arg("commit", |a| {
            a.required()
                .description("the commit, by the 12-hex name gsd/E521 prints");
        })
        .arg("reason", |a| {
            a.required()
                .description("why its trailer error is waived, in the person's words");
        })
        .arg("by", |a| {
            a.required().description("who waives it");
        })
        .handler(|call| run(call, waive));
    });
}
