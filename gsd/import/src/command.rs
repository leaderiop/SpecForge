//! `specforge gsd import` (ADR 0391 D2): no positional; the host serves the project root's `.planning/**` and its
//! `specforge.json`, and the command answers one edit plan that writes `spec/gsd/`, with the report as its output.

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use crate::read::{self, Served};
use crate::records::Records;
use crate::report::Report;

pub const INVALID: &str = "invalid_input";

/// The read of the `.planning/` folder.
pub const PLANNING: &str = "planning";
/// The read of `specforge.json`.
pub const MANIFEST: &str = "manifest";

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("import", |cmd| {
        cmd.description("Import a GSD project once: read the project root's .planning/ folder into one edit plan that writes spec/gsd/, and report every file, section and row it did not map, with the reason")
            .writes()
            .input(Input::files(
                PLANNING,
                FileReadSpec::glob(".planning/**").text_up_to(TEXT_MAX_CEILING_BYTES),
            ))
            .input(Input::files(
                MANIFEST,
                FileReadSpec::glob("specforge.json").text(),
            ))
            .handler(|call| run(call, import));
    });
}

/// The served files of the read `name`, sorted by path as the host sends a glob's.
fn served(call: &CommandCall<'_>, name: &str) -> Vec<Served> {
    call.files(name)
        .map(|files| {
            files
                .facts
                .iter()
                .map(|fact| Served {
                    path: fact.path.clone(),
                    text: fact.text.clone(),
                    error: fact.error.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn import(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let mut records = Records::default();
    let mut report = Report::default();
    for file in served(call, PLANNING) {
        read::file(&file, &mut records, &mut report);
    }
    let mut plan = call.plan();
    for record in records.iter() {
        plan.create(
            record.kind,
            &record.id,
            &record.title,
            &record.file,
            record.fields.clone(),
            Vec::new(),
        );
    }
    let counts = records.counts();
    Ok(answer(
        call,
        report.payload(&counts),
        report.text(&counts),
        Some(&plan),
    ))
}

/// Runs a command's body: its `Err` is the refusal, reported as `invalid_input` (core's `support::run`).
pub fn run(
    call: &CommandCall<'_>,
    body: impl FnOnce(&CommandCall<'_>) -> Result<CommandOutput, String>,
) -> CommandOutput {
    match body(call) {
        Ok(out) => out,
        Err(why) => call.fail(&CommandError::new(INVALID, why), 1),
    }
}

/// `payload` through `render`, with `human` as the text form, and `plan` as the edit plan (core's `support::answer`).
pub fn answer(
    call: &CommandCall<'_>,
    payload: Value,
    human: String,
    plan: Option<&PlanBuilder>,
) -> CommandOutput {
    let out = call.render(&payload, |text| text.push_str(&human));
    match plan {
        Some(plan) => out.with_plan(plan.finish()),
        None => out,
    }
}
