//! What the project's extensions contribute to a host, resolved against its settings (ADR 0292): the contributions
//! whose `when` setting is not false, each agent's and skill's body asked of the extension's `render__` export for the
//! host and the variant its setting picks, each slash command's description and argument hint read from the prompt or
//! command it calls, each hook with its paths spelled for the project. The renderer knows no GSD word: it hands the
//! adapter neutral values.

use serde_json::Value;
use specforge_protocol_types::command_args::option_name;
use specforge_protocol_types::{
    CommandArgType, ExtensionDeclaration, OnFailure, PromptArgDescriptor, RenderInput, RenderKind,
    ServedSlash, SlashCommandDescriptor,
};
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

/// Whether the contribution whose `when` is `when` is on: every setting named is not false (ADR 0317).
pub(crate) fn on(view: &ProjectView<'_>, extension: &str, when: &[String]) -> bool {
    when.iter()
        .all(|name| setting(view, extension, name) != Some(Value::Bool(false)))
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
    let named = hint(
        args.iter()
            .filter(|a| !a.rest)
            .map(|a| (a.name.as_str(), a.required, a.field_type == "bool")),
    );
    // A free-text tail is written last, as upstream's hint writes the task description.
    let tail = args
        .iter()
        .find(|a| a.rest)
        .map(|a| format!("[<{}>\u{2026}]", option_name(&a.name)));
    match (named, tail) {
        (Some(named), Some(tail)) => Some(format!("{named} {tail}")),
        (named, tail) => named.or(tail),
    }
}

/// What the slash command `s` of the extension `declaration` (short name `short`) calls: the target's id and
/// description, the hint derived from its arguments, and the call; none when the target is not declared.
fn slash_target(
    declaration: &ExtensionDeclaration,
    s: &SlashCommandDescriptor,
    short: &str,
) -> Option<(String, String, Option<String>, SlashCall)> {
    match (&s.prompt, &s.command) {
        (Some(id), _) => {
            let prompt = declaration.surfaces.prompts.iter().find(|p| p.id == *id)?;
            Some((
                id.clone(),
                prompt.description.clone(),
                prompt_hint(&prompt.args),
                SlashCall::Prompt {
                    mcp_name: format!("specforge.{short}.{id}"),
                    cli: format!("specforge prompt {short}.{id}"),
                },
            ))
        }
        (None, Some(id)) => {
            let command = declaration.surfaces.commands.iter().find(|c| c.id == *id)?;
            Some((
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
            ))
        }
        (None, None) => None,
    }
}

/// The slash commands the project serves, sorted by name: what an install writes for a host, as a prompt that
/// reads them is sent (ADR 0316). A slash is served while its `when` setting is on and its target is declared.
pub fn served_slashes(view: &ProjectView<'_>) -> Vec<ServedSlash> {
    let build = view.registries();
    let mut served: Vec<ServedSlash> = build
        .slash_commands()
        .filter(|slash| on(view, slash.extension, &slash.item.when))
        .filter_map(|slash| {
            let declaration = build.declaration(slash.extension)?;
            let (_, _, _, call) = slash_target(declaration, slash.item, &slash.short)?;
            let (prompt, tool, cli) = match call {
                SlashCall::Prompt { mcp_name, cli } => (Some(mcp_name), None, cli),
                SlashCall::Command { mcp_tool, cli } => (None, Some(mcp_tool), cli),
            };
            Some(ServedSlash {
                name: slash.name(),
                extension: slash.extension.to_string(),
                prompt,
                tool,
                cli,
            })
        })
        .collect();
    served.sort_by(|a, b| a.name.cmp(&b.name));
    served
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
        if !on(view, ext, &a.when) {
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
        if !on(view, ext, &s.when) {
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
        if !on(view, ext, &s.when) {
            continue;
        }
        let Some(declaration) = build.declaration(ext) else {
            continue;
        };
        let Some((id, description, derived, call)) = slash_target(declaration, s, &slash.short)
        else {
            continue;
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
        .filter(|h| on(view, h.extension, &h.item.when))
        // The extension chose its hosts, so a hook left out has no `not_rendered` row (ADR 0322).
        .filter(|h| h.item.hosts.is_empty() || h.item.hosts.iter().any(|x| x == host.as_str()))
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
            on_failure: h.item.on_failure,
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
        on_failure: OnFailure::Deny,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(
        behavior = "install_into_a_host",
        verify = "a slash command's hint writes a prompt's free-text tail last"
    )]
    fn a_free_text_tail_is_written_last_in_the_hint() {
        let arg = |name: &str, field_type: &str, required: bool, rest: bool| PromptArgDescriptor {
            name: name.into(),
            field_type: field_type.into(),
            required,
            rest,
            ..Default::default()
        };
        let args = [
            arg("task", "string", false, true),
            arg("full", "bool", false, false),
        ];
        assert_eq!(
            prompt_hint(&args).as_deref(),
            Some("[--full] [<task>\u{2026}]")
        );
        assert_eq!(prompt_hint(&args[..1]).as_deref(), Some("[<task>\u{2026}]"));
    }
}
