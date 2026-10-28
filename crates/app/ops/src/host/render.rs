//! What the project's extensions contribute to a host, resolved against its settings (ADR 0292): the contributions
//! whose `when` setting is not false, each agent's and skill's body asked of the extension's `render__` export for the
//! host and the variant its setting picks, each slash command's description and argument hint read from the prompt or
//! command it calls, each hook with its paths spelled for the project. The renderer knows no GSD word: it hands the
//! adapter neutral values.

use serde_json::Value;
use specforge_protocol_types::command_args::option_name;
use specforge_protocol_types::{CommandArgType, PromptArgDescriptor, RenderInput, RenderKind};
use specforge_wasm::{CallError, CallFailure, ExtensionCalls, Operation};

use super::Host;
use super::port::{
    ModelChoice, Rendered, RenderedAgent, RenderedHook, RenderedSkill, RenderedSlash, SlashCall,
    Source,
};
use crate::view::ProjectView;
use crate::{OpError, OpErrorKind};

/// `name` as the extension's setting reads now (`None`: unset, or no such setting).
pub(crate) fn setting(view: &ProjectView<'_>, extension: &str, name: &str) -> Option<Value> {
    view.env()
        .settings()
        .of(extension)
        .and_then(|s| s.get(name))
        .and_then(|s| s.value.clone())
}

/// Whether the contribution whose `when` is `when` is on: no setting named, or one that is not false.
pub(crate) fn on(view: &ProjectView<'_>, extension: &str, when: Option<&String>) -> bool {
    when.is_none_or(|name| setting(view, extension, name) != Some(Value::Bool(false)))
}

/// A word that is a setting's current value when it reads `setting:<name>`.
fn word(view: &ProjectView<'_>, extension: &str, raw: &str) -> Option<String> {
    match raw.strip_prefix("setting:") {
        Some(name) => setting(view, extension, name).and_then(|v| v.as_str().map(str::to_string)),
        None => Some(raw.to_string()),
    }
}

fn model(view: &ProjectView<'_>, extension: &str, raw: Option<&String>) -> ModelChoice {
    match raw.and_then(|r| word(view, extension, r)).as_deref() {
        None | Some("" | "inherit") => ModelChoice::Inherit,
        Some("heavy") => ModelChoice::Tier("heavy"),
        Some("standard") => ModelChoice::Tier("standard"),
        Some("light") => ModelChoice::Tier("light"),
        Some(id) => ModelChoice::Id(id.to_string()),
    }
}

fn effort(view: &ProjectView<'_>, extension: &str, raw: Option<&String>) -> Option<String> {
    word(view, extension, raw?).filter(|e| !e.is_empty() && e != "inherit")
}

/// The call error as the operation's: E028, an extension that did not answer.
pub(crate) fn call_failed(error: &CallError) -> OpError {
    OpError::new(
        OpErrorKind::Internal,
        specforge_common::codes::E028.id(),
        error.diagnostic().message.clone(),
    )
}

/// The arguments of a prompt or command as a hint: `<phase> [--research] [--depth <value>]`.
fn hint<'a>(args: impl Iterator<Item = (&'a str, bool, bool)>) -> Option<String> {
    let words: Vec<String> = args
        .map(|(name, required, flag)| {
            let name = option_name(name);
            match (required, flag) {
                (true, _) => format!("<{name}>"),
                (false, true) => format!("[--{name}]"),
                (false, false) => format!("[--{name} <value>]"),
            }
        })
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

fn prompt_hint(args: &[PromptArgDescriptor]) -> Option<String> {
    hint(
        args.iter()
            .map(|a| (a.name.as_str(), a.required, a.field_type == "bool")),
    )
}

/// The project's contributions for `host`, with every body rendered.
pub fn rendered(view: &ProjectView<'_>, host: Host) -> Result<Rendered, OpError> {
    let build = view.registries();
    let calls = view
        .runtime()
        .map(|runtime| ExtensionCalls::new(runtime.as_ref()));
    let render = |extension: &str, kind: RenderKind, id: &str, variant: Option<String>| {
        let Some(calls) = &calls else {
            return Err(call_failed(&CallError::new(
                Operation::Render,
                extension,
                "render__",
                CallFailure::NotLoaded,
            )));
        };
        let input = RenderInput {
            kind,
            id: id.to_string(),
            host: host.as_str().to_string(),
            variant,
            // The date is not sent: a body that depended on it would make install drift every day.
            today: String::new(),
        };
        calls
            .render(extension, &input)
            .map(|out| out.body)
            .map_err(|e| call_failed(&e))
    };
    let version = |extension: &str| {
        build
            .declaration(extension)
            .map(|d| d.version().to_string())
            .unwrap_or_default()
    };
    let source = |extension: &str, kind: &'static str, id: &str| Source {
        extension: extension.to_string(),
        version: version(extension),
        kind,
        id: id.to_string(),
    };

    let mut agents = Vec::new();
    for agent in build.agents() {
        let (ext, a) = (agent.extension, agent.item);
        if !on(view, ext, a.when.as_ref()) {
            continue;
        }
        let variant = a
            .variant_setting
            .as_deref()
            .and_then(|s| word(view, ext, &format!("setting:{s}")))
            .filter(|v| a.variants.contains(v));
        let mut body = render(ext, RenderKind::Agent, &a.id, variant)?;
        body.push_str(&judged_section(view, &a.id));
        agents.push(RenderedAgent {
            source: source(ext, "agent", &a.id),
            id: a.id.clone(),
            description: a.description.clone(),
            tools: a.tools.clone(),
            model: model(view, ext, a.model.as_ref()),
            effort: effort(view, ext, a.effort.as_ref()),
            color: a.color.clone(),
            body,
            attribution: a.attribution.clone(),
        });
    }

    let mut skills = Vec::new();
    for skill in build.skills() {
        let (ext, s) = (skill.extension, skill.item);
        if !on(view, ext, s.when.as_ref()) {
            continue;
        }
        skills.push(RenderedSkill {
            source: source(ext, "skill", &s.id),
            id: s.id.clone(),
            description: s.description.clone(),
            tools: s.tools.clone(),
            argument_hint: s.argument_hint.clone(),
            body: render(ext, RenderKind::Skill, &s.id, None)?,
            attribution: s.attribution.clone(),
        });
    }

    let mut slashes = Vec::new();
    for slash in build.slash_commands() {
        let (ext, s) = (slash.extension, slash.item);
        if !on(view, ext, s.when.as_ref()) {
            continue;
        }
        let Some(declaration) = build.declaration(ext) else {
            continue;
        };
        let short = &slash.short;
        let (id, description, derived, call) = match (&s.prompt, &s.command) {
            (Some(id), _) => {
                let Some(prompt) = declaration.surfaces.prompts.iter().find(|p| p.id == *id) else {
                    continue;
                };
                (
                    id.clone(),
                    prompt.description.clone(),
                    prompt_hint(&prompt.args),
                    SlashCall::Prompt {
                        mcp_name: format!("specforge.{short}.{id}"),
                        cli: format!("specforge prompt {short}.{id}"),
                    },
                )
            }
            (None, Some(id)) => {
                let Some(command) = declaration.surfaces.commands.iter().find(|c| c.id == *id)
                else {
                    continue;
                };
                (
                    id.clone(),
                    command.description.clone(),
                    hint(command.args.iter().map(|a| {
                        (
                            a.name.as_str(),
                            a.required,
                            matches!(a.arg_type, CommandArgType::Bool),
                        )
                    })),
                    SlashCall::Command {
                        mcp_tool: format!("specforge.{short}.{id}"),
                        cli: format!("specforge {short} {}", option_name(id)),
                    },
                )
            }
            (None, None) => continue,
        };
        slashes.push(RenderedSlash {
            source: source(ext, "slash", &id),
            name: slash.name(),
            description,
            argument_hint: s.argument_hint.clone().or(derived),
            tools: s.tools.clone(),
            effort: effort(view, ext, s.effort.as_ref()),
            call,
            attribution: s.attribution.clone(),
        });
    }

    let mut hooks = build
        .hooks()
        .filter(|h| on(view, h.extension, h.item.when.as_ref()))
        .map(|h| RenderedHook {
            route: h.route(),
            description: h.item.description.clone(),
            event: h.item.event,
            tools: h.item.tools.clone(),
            paths: h
                .item
                .paths
                .iter()
                .map(|p| spec_root_spelled(view, p))
                .collect(),
            timeout_ms: h.item.timeout_ms,
        })
        .collect::<Vec<_>>();
    hooks.push(spec_guard(view));

    Ok(Rendered {
        host,
        agents,
        skills,
        slashes,
        hooks,
    })
}

/// The route of the host's own hook, the tool classes and the files it is wired on.
pub const SPEC_GUARD: &str = "core.spec_guard";
pub const SPEC_GUARD_TOOLS: [&str; 2] = ["write", "edit"];
pub const SPEC_GUARD_PATHS: &str = "{spec_root}/**/*.spec";

/// The host's own hook, wired by every install: a raw write of a computed, harness or human-only value in a spec file is
/// denied (ADR 0292). Its logic is `authoring::hook::spec_guard`.
fn spec_guard(view: &ProjectView<'_>) -> RenderedHook {
    RenderedHook {
        route: SPEC_GUARD.into(),
        description: "Deny a write of a computed, harness or human-only value in a spec file"
            .into(),
        event: specforge_protocol_types::HookEvent::PreTool,
        tools: SPEC_GUARD_TOOLS.iter().map(|t| (*t).to_string()).collect(),
        paths: vec![spec_root_spelled(view, SPEC_GUARD_PATHS)],
        timeout_ms: 10_000,
    }
}

/// `{spec_root}` in a hook's path glob, spelled as the project-relative spec root (nothing for the project root).
pub(crate) fn spec_root_spelled(view: &ProjectView<'_>, glob: &str) -> String {
    let root =
        specforge_common::ProjectPath::of(view.env().root(), view.env().spec_root()).into_string();
    let root = root.trim_matches('/').trim_start_matches("./");
    let root = if root == "." { "" } else { root };
    if root.is_empty() {
        glob.replace("{spec_root}/", "").replace("{spec_root}", ".")
    } else {
        glob.replace("{spec_root}", root)
    }
}

/// The section a host adds after the body of an agent that judges rules (K7): each rule, the tool that reads its
/// inputs and the outcomes it may answer.
fn judged_section(view: &ProjectView<'_>, agent: &str) -> String {
    let rules = view.registries().judged_rules_of(agent);
    if rules.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n## Judged rules\n\n");
    for rule in rules {
        out.push_str(&format!(
            "- `{}`: read the inputs with `specforge.verdict_inputs`; the outcomes are {} (passing: {}).\n",
            rule.code,
            rule.outcomes.join(", "),
            rule.passing.join(", ")
        ));
    }
    out
}
