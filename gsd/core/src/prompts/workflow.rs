//! A lifecycle prompt as data (D-r42-1): its arguments, subject, gates and steps, and the slash command it is bound
//! to. `declare` turns the row into a prompt whose handler is the one render pipeline.
//!
//! Also ports gsd-core bin/lib modules: workflow-fragments.cjs.

use specforge_extension_sdk::prelude::*;

use super::args::{self, Arg};
use super::render::{self, Input};
use super::slash;
use crate::settings;

/// What one step of a prompt asks of the agent.
pub enum Do {
    /// Spawn the core agent `id` (the rendered step's `agent` names it, ADR 0313).
    Agent(&'static str),
    /// Run the command `id` of this extension.
    Command(&'static str),
    /// Continue with the prompt `id`.
    Prompt(&'static str),
}

pub struct Step {
    pub id: &'static str,
    pub title: &'static str,
    pub does: Do,
}

/// The slash binding of a prompt.
pub struct Slash {
    pub hint: &'static str,
    /// Tool classes (`read`, `shell`, `mcp:context7/*`, ...), upstream's `allowed-tools` mapped.
    pub tools: &'static [&'static str],
    pub effort: Option<&'static str>,
}

pub struct Spec {
    pub id: &'static str,
    pub title: &'static str,
    /// Upstream's command description, verbatim.
    pub description: &'static str,
    /// The stem of upstream's command file: `plan-phase`.
    pub command: &'static str,
    pub args: &'static [Arg],
    /// The reference argument whose entity scopes the gates.
    pub subject: Option<&'static str>,
    pub gates: &'static [&'static str],
    pub steps: &'static [Step],
    pub slash: Option<Slash>,
    /// The prompt reads the extension's state (`call.state()`).
    pub reads_state: bool,
    /// The prompt receives the slash commands the project serves (`PromptCall::slash_commands`, ADR 0316).
    pub reads_slash_commands: bool,
    /// A closing section of the prompt's own (`spec_phase`'s judging text).
    pub tail: Option<fn(&PromptCall<'_>) -> String>,
}

/// The notice of an upstream file: `commands/gsd/plan-phase.md`.
pub fn attribution(path: &str) -> Attribution {
    Attribution {
        upstream: "gsd-core".to_string(),
        path: path.to_string(),
        version: "v1.18.0".to_string(),
        license: "MIT".to_string(),
        copyright: "Copyright (c) 2026 Open GSD".to_string(),
    }
}

/// The ids the shell blocks use: the project, and the milestone the run is about (the subject's, else the one
/// still open, else the last).
fn shell_ids(call: &PromptCall<'_>, subject: Option<&str>) -> String {
    let nodes: &[_] = call.graph().map_or(&[], |g| &g.nodes);
    let of_kind = |kind: &'static str| nodes.iter().filter(move |n| n.kind == kind);
    let project = of_kind("project").map(|n| n.id.as_str()).next();
    let milestone = subject
        .map(|id| id.split('/').next().unwrap_or(id))
        .or_else(|| {
            of_kind("milestone")
                .find(|n| {
                    !n.fields.contains_key("completion") && !n.fields.contains_key("abandoned")
                })
                .or_else(|| of_kind("milestone").next_back())
                .map(|n| n.id.as_str())
        });
    let say = |var: &str, id: Option<&str>| match id {
        Some(id) => format!("`${var}` is `{id}`"),
        None => format!("`${var}` is not in the spec yet (leave that command out)"),
    };
    format!(
        "{} and {}.",
        say("MILESTONE", milestone),
        say("PROJECT", project)
    )
}

/// The first phase of the current milestone that no plan names, in phase order (`999.` backlog phases left out).
fn next_unplanned(call: &PromptCall<'_>) -> Option<String> {
    // a prompt's graph is the wire one: the milestone is picked as `reports::current` does
    let nodes: &[_] = call.graph().map_or(&[], |g| &g.nodes);
    let of_kind = |kind: &'static str| nodes.iter().filter(move |n| n.kind == kind);
    let milestone = of_kind("milestone")
        .find(|m| m.text("state") == Some("in_progress"))
        .or_else(|| of_kind("milestone").next_back())?;
    let scope = format!("{}/", milestone.id);
    let planned: std::collections::BTreeSet<&str> =
        of_kind("plan").filter_map(|p| p.text("phase")).collect();
    let number = |id: &str| id.rsplit('/').next().unwrap_or_default().to_string();
    of_kind("phase")
        .filter(|p| p.id.starts_with(&scope) && !number(&p.id).starts_with("999."))
        .filter(|p| !planned.contains(p.id.as_str()))
        .min_by(|a, b| crate::commands::smart_entry::compare_phase(&number(&a.id), &number(&b.id)))
        .map(|p| p.id.clone())
}

fn handle(spec: &'static Spec, call: &PromptCall<'_>) -> Result<PromptOutput, String> {
    let mut arguments = args::text(call, spec.args);
    let mut tail = spec.tail.map(|f| f(call)).unwrap_or_default();
    let mut picked = None;
    let mut subject = spec.subject.and_then(|n| call.str(n).map(|id| (n, id)));
    // An optional subject that was not given: the next unplanned phase, else upstream's ask.
    if let (None, Some(name)) = (subject, spec.subject)
        && spec
            .args
            .iter()
            .any(|a| a.name == name && matches!(a.kind, args::Kind::Subject(_)))
    {
        match next_unplanned(call) {
            Some(id) => picked = Some(id),
            None => {
                return Ok(PromptOutput::user(
                    "No unplanned phase in this milestone: ask the user which phase to plan.",
                ));
            }
        }
        subject = picked.as_deref().map(|id| (name, id));
        let number = picked
            .as_deref()
            .and_then(|id| id.rsplit('/').next())
            .unwrap_or_default();
        arguments = format!("{number} {arguments}").trim().to_string();
    }
    let mut ids = shell_ids(call, subject.map(|(_, id)| id));
    if let Some((name, id)) = subject {
        let var = match name {
            "phase" => "PHASE_ID",
            "plan" => "PLAN_ID",
            "milestone" => "MILESTONE",
            _ => "SUBJECT_ID",
        };
        if name != "milestone" {
            ids = format!("`${var}` is `{id}`; {ids}");
        }
    }
    // a router (`ns_*`) and `update` have no shell block of their own to bind the ids in
    if !spec.id.starts_with("ns_") && spec.id != "update" {
        tail = format!("\n\nIn the shell blocks above, {ids}{tail}");
    }
    let text = render::render(&Input {
        command: spec.command,
        host: call.host().unwrap_or("claude-code"),
        arguments: &arguments,
        settings: &settings::block(),
        tail: &tail,
    })?;
    let out = PromptOutput::user(text);
    Ok(match picked {
        Some(id) => out.with_subject(id),
        None => out,
    })
}

/// A prompt's closing line about the active workstream (`reads_state`).
pub fn workstream_tail(call: &PromptCall<'_>) -> String {
    match crate::commands::support::active_workstream(call.state()) {
        Some(id) => format!("\n\nActive workstream: {id}."),
        None => "\n\nNo workstream is active.".to_string(),
    }
}

/// The upstream file the prompt is ported from: its command, or the workflow when upstream has no command for it.
fn source_path(spec: &Spec) -> Option<String> {
    let command = format!("commands/gsd/{}.md", spec.command);
    if super::assets::get(&command).is_some() {
        return Some(command);
    }
    let workflow = format!("gsd-core/workflows/{}.md", spec.command);
    super::assets::get(&workflow).map(|_| workflow)
}

/// Declares the prompt of `spec` and, when it has one, its slash command.
pub fn declare(c: &mut ContributionsBuilder, spec: &'static Spec) {
    c.prompt(spec.id, |p| {
        p.title(spec.title)
            .reads_graph()
            .description(spec.description);
        // `update` is our text, not a port: its prompt names no upstream file (its slash binding does)
        if let Some(path) = source_path(spec) {
            p.attribution(attribution(&path));
        }
        if spec.reads_state {
            p.reads_state();
        }
        if spec.reads_slash_commands {
            p.reads_slash_commands();
        }
        args::declare(p, spec.args);
        if let Some(subject) = spec.subject {
            p.subject(subject);
        }
        for gate in spec.gates {
            p.gate(gate);
        }
        for step in spec.steps {
            p.step(step.id, |s| {
                s.title(step.title);
                match step.does {
                    Do::Agent(id) => s.agent(id),
                    Do::Command(id) => s.command(id),
                    Do::Prompt(id) => s.prompt(id),
                };
            });
        }
        p.handler(move |call| handle(spec, call));
    });
    slash::bind(c, spec);
}
