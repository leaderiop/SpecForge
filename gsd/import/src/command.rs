//! `specforge gsd import` (ADR 0391 D2): no positional; the host serves the project root's `.planning/**` and its
//! `specforge.json`, and the command answers one edit plan that writes `spec/gsd/`, with the report as its output.

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use crate::preflight::Manifest;
use crate::read::{self, Served};
use crate::records::Records;
use crate::report::Report;
use crate::{emit, owed, preflight};

pub const INVALID: &str = "invalid_input";

/// The read of the `.planning/` folder.
pub const PLANNING: &str = "planning";
/// The read of `specforge.json`.
pub const MANIFEST: &str = "manifest";

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("import", |cmd| {
        cmd.description("Import a GSD project once: read the project root's .planning/ folder into one edit plan that writes spec/gsd/, and report every file, section and row it did not map, with the reason")
            // History, not authoring: a rule error the `.planning/` carries lands and `check` reports it (ADR 0392).
            .imports()
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

/// The served files of the read `name`, sorted by path as the host sends a glob's, and whether the read stopped early.
fn served(call: &CommandCall<'_>, name: &str) -> (Vec<Served>, bool) {
    call.files(name)
        .map(|files| {
            let served = files
                .facts
                .iter()
                .map(|fact| Served {
                    path: fact.path.clone(),
                    text: fact.text.clone(),
                    error: fact.error.clone(),
                })
                .collect();
            (served, files.truncated)
        })
        .unwrap_or_default()
}

/// Preflight, read, emit, answer: a refusal anywhere answers no plan.
fn import(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let graph = call.graph();
    preflight::refuse_reimport(graph)?;
    let manifest = preflight::manifest(&served(call, MANIFEST).0)?;
    let (planning, truncated) = served(call, PLANNING);
    preflight::check_reads(&planning, truncated, manifest.security)?;
    let imported = imported(planning, &manifest, graph, call.plan())?;
    Ok(answer(
        call,
        imported.payload,
        imported.text,
        Some(&imported.plan),
    ))
}

/// What one import answers: its plan, and its report as JSON and as text.
pub struct Imported {
    pub plan: PlanBuilder,
    pub payload: Value,
    pub text: String,
}

/// Reads `planning`, in path order whatever the order it is given in (an import is deterministic, ADR 0391 D8), into
/// a plan built on `plan` over `graph`, with its report and the verdicts it owes.
pub fn imported(
    mut planning: Vec<Served>,
    manifest: &Manifest,
    graph: &CommandGraph,
    plan: PlanBuilder,
) -> Result<Imported, String> {
    planning.sort_by(|a, b| a.path.cmp(&b.path));
    let mut records = Records::default();
    let mut report = Report::default();
    read::all(&planning, manifest, &mut records, &mut report);
    let (plan, counts, kept) = emit::emit(&records, graph, plan, &mut report)?;
    for owed in owed::owed(kept.iter().copied()) {
        report.owed(owed);
    }
    let mut payload = report.payload(&counts);
    payload["security"] = json!({ "loaded": manifest.security, "check": SECURITY_CHECK });
    let mut text = report.text(&counts);
    text.push_str(&format!(
        "@gsd/security {}loaded ({SECURITY_CHECK})\n",
        if manifest.security { "" } else { "not " }
    ));
    Ok(Imported {
        plan,
        payload,
        text,
    })
}

/// How the import knows the project loads `@gsd/security` (ADR 0391 D11).
const SECURITY_CHECK: &str = "a text check of specforge.json's extensions, not a registry query";

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
