//! The agents, skills, slash commands and hooks an extension declares, as the registry judges them (ADR 0292).
//!
//! Every fault is W021 and costs the one item: an id that is not kebab (agent, skill) or snake (hook), a tool class,
//! `model` or `effort` outside its vocabulary, a `setting:` that names no setting of the right type, a variant
//! without its setting, a `when` that is no bool setting, a slash target the extension does not declare, `tools` or
//! `paths` on an event that takes none, a `timeout_ms` of 0 or above the ceiling, `writes` on an event that cannot,
//! a hook export that is not `hook__<id>`, a hook host outside claude-code, codex and cursor. Host names are one namespace per project: two claimants of one name,
//! across kinds and extensions, are E039, and the later is not rendered. Hooks route as `<short>.<id>`.

use std::collections::{BTreeSet, HashMap};

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::command_args::option_name;
use specforge_protocol_types::{
    AgentDescriptor, Attribution, ExtensionDeclaration, FieldType, HOOK_HOSTS, HookDescriptor,
    SkillDescriptor, SlashCommandDescriptor,
};

use super::prompts::snake_case;
use crate::SettingsRegistry;

/// The tier a `model` names, and the effort an `effort` names, when it is not a `setting:`.
const MODELS: [&str; 4] = ["heavy", "standard", "light", "inherit"];
const EFFORTS: [&str; 6] = ["minimal", "low", "medium", "high", "max", "inherit"];
const CLASSES: [&str; 11] = [
    "read",
    "write",
    "edit",
    "shell",
    "search",
    "web_fetch",
    "web_search",
    "ask_user",
    "agent",
    "skill",
    "todo",
];
/// The longest a hook may be given, whatever the sandbox allows.
const HOOK_CEILING_MS: u32 = 30_000;

/// An item an extension contributes, with who contributes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Contributed<'a, T> {
    /// The extension's name.
    pub extension: &'a str,
    /// Its short name.
    pub short: String,
    pub item: &'a T,
}

impl Contributed<'_, SlashCommandDescriptor> {
    /// The host's name for the slash command: `<short>-<route>`, the route being the target's CLI name.
    pub fn name(&self) -> String {
        slash_name(&self.short, self.item)
    }
}

impl Contributed<'_, HookDescriptor> {
    /// The route `specforge hook` takes: `<short>.<id>`.
    pub fn route(&self) -> String {
        format!("{}.{}", self.short, self.item.id)
    }
}

/// `<short>-<route>`: the host name of `slash`.
pub fn slash_name(short: &str, slash: &SlashCommandDescriptor) -> String {
    let target = slash.prompt.as_deref().or(slash.command.as_deref());
    format!("{short}-{}", option_name(target.unwrap_or_default()))
}

fn kebab_case(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|part| {
            part.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn warn(kind: &str, id: &str, extension: &str, reason: &str) -> Diagnostic {
    Diagnostic::new(
        codes::W021,
        format!("{kind} '{id}' of '{extension}' is not served: {reason}"),
    )
}

/// What the checks of one extension read: its settings and its sandbox.
struct Ctx<'a> {
    settings: &'a SettingsRegistry,
    extension: &'a str,
    ceiling_ms: u32,
}

impl Ctx<'_> {
    fn setting_type(&self, name: &str) -> Option<FieldType> {
        self.settings
            .of(self.extension)
            .and_then(|s| s.find(name))
            .map(|s| s.form.field_type())
    }

    fn tools(&self, tools: &[String]) -> Result<(), String> {
        for tool in tools {
            let known = CLASSES.contains(&tool.as_str())
                || tool
                    .strip_prefix("mcp:")
                    .and_then(|rest| rest.split_once('/'))
                    .is_some_and(|(server, glob)| !server.is_empty() && !glob.is_empty());
            if !known {
                return Err(format!("its tool class '{tool}' is not one a host maps"));
            }
        }
        Ok(())
    }

    /// `value` is one of `words`, or `setting:<name>` naming a string or enum setting of the extension.
    fn word_or_setting(&self, what: &str, value: &str, words: &[&str]) -> Result<(), String> {
        if let Some(name) = value.strip_prefix("setting:") {
            return match self.setting_type(name) {
                Some(FieldType::String | FieldType::Enum) => Ok(()),
                Some(_) => Err(format!(
                    "its {what} '{value}' names a setting that is neither a string nor an enum"
                )),
                None => Err(format!(
                    "its {what} '{value}' names no setting of the extension"
                )),
            };
        }
        if words.contains(&value) {
            Ok(())
        } else {
            Err(format!(
                "its {what} '{value}' is not one of {}",
                words.join(", ")
            ))
        }
    }

    fn when(&self, when: &[String]) -> Result<(), String> {
        match when
            .iter()
            .find(|s| self.setting_type(s) != Some(FieldType::Bool))
        {
            Some(setting) => Err(format!(
                "its 'when' '{setting}' is not a bool setting of the extension"
            )),
            None => Ok(()),
        }
    }

    fn agent(&self, agent: &AgentDescriptor) -> Result<(), String> {
        if !kebab_case(&agent.id) {
            return Err("its id is not kebab-case".into());
        }
        self.tools(&agent.tools)?;
        if let Some(model) = &agent.model {
            self.word_or_setting("model", model, &MODELS)?;
        }
        if let Some(effort) = &agent.effort {
            self.word_or_setting("effort", effort, &EFFORTS)?;
        }
        match (&agent.variant_setting, agent.variants.is_empty()) {
            (None, false) => return Err("it declares variants and no variant_setting".into()),
            (Some(_), true) => return Err("it declares a variant_setting and no variants".into()),
            (Some(setting), false)
                if !matches!(
                    self.setting_type(setting),
                    Some(FieldType::String | FieldType::Enum)
                ) =>
            {
                return Err(format!(
                    "its variant_setting '{setting}' is not a string or enum setting of the extension"
                ));
            }
            _ => {}
        }
        self.when(&agent.when)
    }

    fn skill(&self, skill: &SkillDescriptor) -> Result<(), String> {
        if !kebab_case(&skill.id) {
            return Err("its id is not kebab-case".into());
        }
        self.tools(&skill.tools)?;
        self.when(&skill.when)
    }

    fn hook(&self, hook: &HookDescriptor) -> Result<(), String> {
        if !snake_case(&hook.id) {
            return Err("its id is not snake_case".into());
        }
        if hook.export != format!("hook__{}", hook.id) {
            return Err(format!(
                "its export '{}' is not 'hook__{}'",
                hook.export, hook.id
            ));
        }
        if hook.timeout_ms == 0 || hook.timeout_ms > self.ceiling_ms {
            return Err(format!(
                "its timeout_ms {} is not between 1 and {}",
                hook.timeout_ms, self.ceiling_ms
            ));
        }
        if (!hook.event.takes_tools() && !hook.tools.is_empty())
            || (!hook.event.takes_paths() && !hook.paths.is_empty())
        {
            return Err(format!(
                "the event '{}' takes no tools or paths",
                hook.event.as_str()
            ));
        }
        if hook.event == specforge_protocol_types::HookEvent::FileChanged {
            if hook.paths.is_empty() {
                return Err("a file_changed hook names the files it watches in paths".into());
            }
            if let Some(glob) = hook.paths.iter().find(|p| p.contains(['*', '?', '[', '{'])) {
                return Err(format!(
                    "a file_changed hook's paths are file names, not globs: '{glob}'"
                ));
            }
        }
        self.tools(&hook.tools)?;
        if hook.writes && !hook.event.can_write() {
            return Err(format!(
                "the event '{}' cannot answer a plan, yet it declares writes",
                hook.event.as_str()
            ));
        }
        if let Some(host) = hook
            .hosts
            .iter()
            .find(|h| !HOOK_HOSTS.contains(&h.as_str()))
        {
            return Err(format!(
                "its host '{host}' is not claude-code, codex or cursor"
            ));
        }
        self.when(&hook.when)
    }

    fn slash(
        &self,
        slash: &SlashCommandDescriptor,
        declaration: &ExtensionDeclaration,
    ) -> Result<(), String> {
        let surfaces = &declaration.surfaces;
        match (&slash.prompt, &slash.command) {
            (Some(prompt), None) if surfaces.prompts.iter().any(|p| p.id == *prompt) => {}
            (Some(prompt), None) => {
                return Err(format!(
                    "it names the prompt '{prompt}', which the extension does not declare"
                ));
            }
            (None, Some(command)) if surfaces.commands.iter().any(|c| c.id == *command) => {}
            (None, Some(command)) => {
                return Err(format!(
                    "it names the command '{command}', which the extension does not declare"
                ));
            }
            _ => return Err("it names exactly one of a prompt and a command".into()),
        }
        self.tools(&slash.tools)?;
        if let Some(effort) = &slash.effort {
            self.word_or_setting("effort", effort, &EFFORTS)?;
        }
        self.when(&slash.when)
    }
}

/// An attribution with an empty field is dropped alone.
fn clean(
    attribution: &mut Option<Attribution>,
    kind: &str,
    id: &str,
    extension: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if attribution.as_ref().is_some_and(|a| {
        [&a.upstream, &a.path, &a.version, &a.license, &a.copyright]
            .iter()
            .any(|field| field.trim().is_empty())
    }) {
        *attribution = None;
        diagnostics.push(Diagnostic::new(
            codes::W021,
            format!(
                "{kind} '{id}' of '{extension}' declares an attribution with an empty field: the attribution is ignored"
            ),
        ));
    }
}

fn ctx<'a>(settings: &'a SettingsRegistry, declaration: &'a ExtensionDeclaration) -> Ctx<'a> {
    let sandbox = declaration
        .handshake
        .sandbox_policy
        .as_ref()
        .and_then(|s| s.max_execution_ms);
    Ctx {
        settings,
        extension: declaration.name(),
        ceiling_ms: sandbox.map_or(HOOK_CEILING_MS, |ms| ms.min(HOOK_CEILING_MS)),
    }
}

/// Drop each agent, skill and hook of `declarations` the registry refuses, with the W021 that says why.
pub fn refuse_host_contributions(
    declarations: &mut [ExtensionDeclaration],
    settings: &SettingsRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for declaration in declarations.iter_mut() {
        let name = declaration.name().to_string();
        let mut surfaces = std::mem::take(&mut declaration.surfaces);
        let ctx = ctx(settings, declaration);
        let mut seen = BTreeSet::new();
        surfaces.agents.retain_mut(|agent| {
            let verdict = if seen.insert(agent.id.clone()) {
                ctx.agent(agent)
            } else {
                Err("an agent or skill of that id is declared twice".into())
            };
            clean(
                &mut agent.attribution,
                "agent",
                &agent.id,
                &name,
                &mut diagnostics,
            );
            keep(verdict, "agent", &agent.id, &name, &mut diagnostics)
        });
        surfaces.skills.retain_mut(|skill| {
            let verdict = if seen.insert(skill.id.clone()) {
                ctx.skill(skill)
            } else {
                Err("an agent or skill of that id is declared twice".into())
            };
            clean(
                &mut skill.attribution,
                "skill",
                &skill.id,
                &name,
                &mut diagnostics,
            );
            keep(verdict, "skill", &skill.id, &name, &mut diagnostics)
        });
        let mut ids = BTreeSet::new();
        surfaces.hooks.retain(|hook| {
            let verdict = if ids.insert(hook.id.clone()) {
                ctx.hook(hook)
            } else {
                Err("it is declared twice".into())
            };
            keep(verdict, "hook", &hook.id, &name, &mut diagnostics)
        });
        declaration.surfaces = surfaces;
    }
    diagnostics
}

fn keep(
    verdict: Result<(), String>,
    kind: &str,
    id: &str,
    extension: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    match verdict {
        Ok(()) => true,
        Err(reason) => {
            diagnostics.push(warn(kind, id, extension, &reason));
            false
        }
    }
}

/// Drop each slash command of `declarations` the registry refuses (its prompts and commands are already judged), with
/// the W021 that says why.
pub fn refuse_slash_commands(
    declarations: &mut [ExtensionDeclaration],
    settings: &SettingsRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for declaration in declarations.iter_mut() {
        let name = declaration.name().to_string();
        let mut slashes = std::mem::take(&mut declaration.surfaces.slash_commands);
        let ctx = ctx(settings, declaration);
        let mut seen = BTreeSet::new();
        slashes.retain_mut(|slash| {
            let target = slash
                .prompt
                .clone()
                .or_else(|| slash.command.clone())
                .unwrap_or_default();
            let verdict = if seen.insert((slash.prompt.clone(), slash.command.clone())) {
                ctx.slash(slash, declaration)
            } else {
                Err("it is declared twice".into())
            };
            clean(
                &mut slash.attribution,
                "slash command",
                &target,
                &name,
                &mut diagnostics,
            );
            keep(verdict, "slash command", &target, &name, &mut diagnostics)
        });
        declaration.surfaces.slash_commands = slashes;
    }
    diagnostics
}

/// E039 for a host name, or a hook route, an earlier claimant has; the later claimant is dropped. Agents, skills and
/// slash commands share one namespace: they are host file names.
pub fn claim_host_names(declarations: &mut [ExtensionDeclaration]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut names: HashMap<String, String> = HashMap::new();
    let mut routes: HashMap<String, String> = HashMap::new();
    for declaration in declarations.iter_mut() {
        let extension = declaration.name().to_string();
        let short = declaration.short().into_owned();
        let mut claim = |name: String, what: String, diagnostics: &mut Vec<Diagnostic>| -> bool {
            match names.get(&name) {
                Some(first) => {
                    diagnostics.push(
                        Diagnostic::new(
                            codes::E039,
                            format!(
                                "host name '{name}' is claimed twice: {what} is not rendered, {first} has it first (a host file is named by it)"
                            ),
                        )
                        .with_suggestion(
                            "rename one of them, or give one extension another short name (ext_short)"
                                .to_string(),
                        ),
                    );
                    false
                }
                None => {
                    names.insert(name, what);
                    true
                }
            }
        };
        let surfaces = &mut declaration.surfaces;
        surfaces.agents.retain(|a| {
            claim(
                a.id.clone(),
                format!("the agent '{}' of '{extension}'", a.id),
                &mut diagnostics,
            )
        });
        surfaces.skills.retain(|s| {
            claim(
                s.id.clone(),
                format!("the skill '{}' of '{extension}'", s.id),
                &mut diagnostics,
            )
        });
        surfaces.slash_commands.retain(|s| {
            let target = s
                .prompt
                .as_deref()
                .or(s.command.as_deref())
                .unwrap_or_default();
            claim(
                slash_name(&short, s),
                format!("the slash command of '{target}' of '{extension}'"),
                &mut diagnostics,
            )
        });
        surfaces.hooks.retain(|h| {
            let route = format!("{short}.{}", h.id);
            match routes.get(&route) {
                Some(first) => {
                    diagnostics.push(Diagnostic::new(
                        codes::E039,
                        format!(
                            "hook route '{route}' is claimed twice: the hook '{}' of '{extension}' is not wired, '{first}' has it first",
                            h.id
                        ),
                    ));
                    false
                }
                None => {
                    routes.insert(route, extension.clone());
                    true
                }
            }
        });
    }
    diagnostics
}

/// The ids of the agents `declaration` may name: its own and those of its declared peers that are loaded. `None`
/// when a declared peer is not loaded, so some agent it may name cannot be seen.
pub(crate) fn visible_agents(
    declarations: &[ExtensionDeclaration],
    declaration: &ExtensionDeclaration,
) -> (BTreeSet<String>, bool) {
    let mut ids: BTreeSet<String> = declaration
        .surfaces
        .agents
        .iter()
        .map(|a| a.id.clone())
        .collect();
    let mut complete = true;
    for peer in declaration.peers() {
        match declarations.iter().find(|d| d.name() == peer.name) {
            Some(loaded) => ids.extend(loaded.surfaces.agents.iter().map(|a| a.id.clone())),
            None => complete = false,
        }
    }
    (ids, complete)
}
