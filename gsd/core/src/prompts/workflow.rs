//! A lifecycle prompt as data (D-r42-1): its arguments, subject, gates and steps, and the slash command it is bound
//! to. `declare` turns the row into a prompt whose handler is the one render pipeline.

use specforge_extension_sdk::prelude::*;

use super::args::{self, Arg};
use super::render::{self, Input};
use super::slash;
use crate::settings;

/// What one step of a prompt asks of the agent.
pub enum Do {
    /// Spawn the core agent `id` (a rendered step carries no agent field, so the title names it).
    Agent(&'static str),
    /// Run the command `id` of this extension.
    Command(&'static str),
    /// Continue with the prompt `id`.
    #[allow(
        dead_code,
        reason = "discuss_phase and the later prompts continue into one another"
    )]
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

fn handle(spec: &'static Spec, call: &PromptCall<'_>) -> Result<PromptOutput, String> {
    let arguments = args::text(call, spec.args);
    let tail = spec.tail.map(|f| f(call)).unwrap_or_default();
    let text = render::render(&Input {
        command: spec.command,
        host: call.host().unwrap_or("claude-code"),
        arguments: &arguments,
        settings: &settings::block(),
        tail: &tail,
    })?;
    Ok(PromptOutput::user(text))
}

/// Declares the prompt of `spec` and, when it has one, its slash command.
pub fn declare(c: &mut ContributionsBuilder, spec: &'static Spec) {
    c.prompt(spec.id, |p| {
        p.title(spec.title)
            .description(spec.description)
            .attribution(attribution(&format!("commands/gsd/{}.md", spec.command)));
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
