//! The prompts an extension declares and the gates of its commands, as the registry judges them (ADR 0291).
//!
//! Every fault is W021. A prompt or command the registry cannot serve as declared is dropped whole (it fails closed:
//! a gate it cannot evaluate, an argument it cannot read, a setting it cannot consult would otherwise be ignored),
//! one W021 naming the item and the first reason; a step's `agent` and an empty attribution are dropped alone.

use std::collections::BTreeSet;

use specforge_common::{Diagnostic, codes};
use specforge_diagnostics::Codebook;
use specforge_graph::FieldForm;
use specforge_protocol_types::{
    CommandArgType, CommandDescriptor, ExtensionDeclaration, FieldType, PromptArgDescriptor,
    PromptDescriptor, PromptStep,
};

use crate::{KindRegistry, SettingsRegistry};

/// The argument names the host owns on every surface of a prompt: its help, the tool arguments it never reads
/// (`settings`, `secrets`), the prompt's `host` and the raw `argv` string. `path` and `format` are a command's
/// options, not a prompt's: a prompt may declare them, and the command line leaves them to it (ADR 0314).
const PROMPT_HOST_NAMES: [&str; 5] = ["help", "settings", "secrets", "host", "argv"];

fn reserved(name: &str) -> bool {
    PROMPT_HOST_NAMES.contains(&name)
}

pub(super) fn snake_case(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// What the checks read of the build, and what the loaded extensions declare for a step to name.
pub struct Judge<'a> {
    pub codes: &'a Codebook,
    pub settings: &'a SettingsRegistry,
    pub kinds: &'a KindRegistry,
}

/// One loaded extension as a step names it: by name or short name, with the commands and prompts it declares.
struct Loaded {
    name: String,
    short: String,
    commands: BTreeSet<String>,
    prompts: BTreeSet<String>,
    agents: BTreeSet<String>,
}

/// Drop each prompt and gated command of `declarations` the registry refuses, with the W021 that says why.
pub fn refuse_prompts(
    declarations: &mut [ExtensionDeclaration],
    judge: &Judge<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    // Dropping a command or prompt can leave a step naming nothing: go again until a pass drops no more.
    loop {
        let before = diagnostics.len();
        let loaded: Vec<Loaded> = declarations
            .iter()
            .map(|d| Loaded {
                name: d.name().to_string(),
                short: d.short().into_owned(),
                commands: d.surfaces.commands.iter().map(|c| c.id.clone()).collect(),
                prompts: d.surfaces.prompts.iter().map(|p| p.id.clone()).collect(),
                agents: d.surfaces.agents.iter().map(|a| a.id.clone()).collect(),
            })
            .collect();
        for declaration in declarations.iter_mut() {
            let name = declaration.name().to_string();
            let peers: Vec<String> = declaration
                .handshake
                .peer_dependencies
                .iter()
                .map(|p| p.name.clone())
                .collect();
            let ctx = Ctx {
                judge,
                extension: &name,
                peers: &peers,
                loaded: &loaded,
            };
            let surfaces = &mut declaration.surfaces;
            surfaces
                .commands
                .retain(|command| match ctx.command(command) {
                    Ok(()) => true,
                    Err(reason) => {
                        diagnostics.push(warn(format!(
                            "command '{}' of '{name}' is not served: {reason}",
                            command.id
                        )));
                        false
                    }
                });
            let mut seen = BTreeSet::new();
            surfaces.prompts.retain_mut(|prompt| {
                let verdict = if seen.insert(prompt.id.clone()) {
                    ctx.prompt(prompt, &mut diagnostics)
                } else {
                    Err("it is declared twice".to_string())
                };
                match verdict {
                    Ok(()) => true,
                    Err(reason) => {
                        diagnostics.push(warn(format!(
                            "prompt '{}' of '{name}' is not served: {reason}",
                            prompt.id
                        )));
                        false
                    }
                }
            });
        }
        if diagnostics.len() == before {
            break;
        }
    }
    diagnostics
}

fn warn(message: String) -> Diagnostic {
    Diagnostic::new(codes::W021, message)
}

struct Ctx<'a> {
    judge: &'a Judge<'a>,
    extension: &'a str,
    peers: &'a [String],
    loaded: &'a [Loaded],
}

impl Ctx<'_> {
    /// A gate code of this extension: catalogued in its `codes`.
    fn gates(&self, gates: &[String]) -> Result<(), String> {
        match gates
            .iter()
            .find(|code| !self.judge.codes.catalogued(self.extension, code))
        {
            Some(code) => Err(format!(
                "its gate '{code}' is not a code the extension catalogs"
            )),
            None => Ok(()),
        }
    }

    fn command(&self, command: &CommandDescriptor) -> Result<(), String> {
        if command.human && !command.writes {
            return Err("it is human and does not write".to_string());
        }
        self.gates(&command.gates)?;
        if let Some(subject) = &command.subject {
            let takes_entity = command
                .args
                .iter()
                .any(|a| a.name == *subject && matches!(a.arg_type, CommandArgType::String));
            if !takes_entity {
                return Err(format!(
                    "its subject '{subject}' is not a string arg it declares"
                ));
            }
        }
        Ok(())
    }

    fn prompt(
        &self,
        prompt: &mut PromptDescriptor,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), String> {
        if !snake_case(&prompt.id) {
            return Err("its id is not snake_case".to_string());
        }
        if prompt.export != format!("prompt__{}", prompt.id) {
            return Err(format!(
                "its export '{}' is not 'prompt__{}'",
                prompt.export, prompt.id
            ));
        }
        let mut names = BTreeSet::new();
        for arg in &prompt.args {
            if !names.insert(arg.name.as_str()) {
                return Err(format!("it declares the argument '{}' twice", arg.name));
            }
            self.arg(arg)?;
        }
        let tails: Vec<String> = (prompt.args.iter().filter(|a| a.rest))
            .map(|a| format!("'{}'", a.name))
            .collect();
        if tails.len() > 1 {
            return Err(format!(
                "a prompt takes one free-text tail, and it declares {}",
                tails.join(" and ")
            ));
        }
        // A subject is a reference and a tail a string, so no argument is both.
        if let Some(tail) = prompt.args.iter().find(|a| a.rest)
            && tail.field_type != FieldType::String.as_str()
        {
            return Err(format!(
                "its free-text tail '{}' is not a string argument",
                tail.name
            ));
        }
        self.gates(&prompt.gates)?;
        if let Some(subject) = &prompt.subject {
            let reference = prompt
                .args
                .iter()
                .any(|a| a.name == *subject && a.field_type == FieldType::Reference.as_str());
            if !reference {
                return Err(format!(
                    "its subject '{subject}' is not a reference argument it declares"
                ));
            }
            // An optional subject is positional, so a required positional after it would take its word (ADR 0314).
            let at = prompt.args.iter().position(|a| a.name == *subject);
            if let Some(at) = at.filter(|&at| !prompt.args[at].required)
                && prompt.args[at + 1..]
                    .iter()
                    .any(|a| a.required && a.field_type != FieldType::Bool.as_str())
            {
                return Err(format!(
                    "its optional subject '{subject}' is declared before a required positional argument"
                ));
            }
        }
        for setting in &prompt.when {
            let is_bool = self
                .judge
                .settings
                .of(self.extension)
                .and_then(|s| s.find(setting))
                .is_some_and(|s| s.form.field_type() == FieldType::Bool);
            if !is_bool {
                return Err(format!(
                    "its 'when' '{setting}' is not a bool setting of the extension"
                ));
            }
        }
        for step in &prompt.steps {
            self.step(step)?;
        }
        let mut ids = BTreeSet::new();
        if let Some(step) = prompt.steps.iter().find(|s| !ids.insert(s.id.as_str())) {
            return Err(format!("it declares the step '{}' twice", step.id));
        }
        for step in &mut prompt.steps {
            let Some(agent) = step.agent.clone() else {
                continue;
            };
            if let Err(reason) = self.reachable(&step.id, &agent, |l| &l.agents, "agent") {
                step.agent = None;
                diagnostics.push(warn(format!(
                    "prompt '{}' of '{}': {reason}: the agent is ignored",
                    prompt.id, self.extension
                )));
            }
        }
        if prompt.attribution.as_ref().is_some_and(|a| {
            [&a.upstream, &a.path, &a.version, &a.license, &a.copyright]
                .iter()
                .any(|field| field.trim().is_empty())
        }) {
            prompt.attribution = None;
            diagnostics.push(warn(format!(
                "prompt '{}' of '{}' declares an attribution with an empty field: the attribution is ignored",
                prompt.id, self.extension
            )));
        }
        Ok(())
    }

    fn arg(&self, arg: &PromptArgDescriptor) -> Result<(), String> {
        let name = &arg.name;
        if name.is_empty() || reserved(name) {
            return Err(format!(
                "its argument name '{name}' is empty or taken by the host"
            ));
        }
        if !snake_case(name) {
            return Err(format!("its argument name '{name}' is not snake_case"));
        }
        let field_type = FieldType::parse(&arg.field_type)
            .filter(|t| {
                matches!(
                    t,
                    FieldType::String
                        | FieldType::Integer
                        | FieldType::Bool
                        | FieldType::Date
                        | FieldType::Enum
                        | FieldType::StringList
                        | FieldType::Reference
                        | FieldType::ReferenceList
                        | FieldType::EnumList
                )
            })
            .ok_or_else(|| {
                format!(
                    "the type '{}' of its argument '{name}' is not one an argument takes",
                    arg.field_type
                )
            })?;
        if matches!(field_type, FieldType::Enum | FieldType::EnumList) && arg.enum_values.is_empty()
        {
            return Err(format!("its argument '{name}' is an enum with no values"));
        }
        let targets: Vec<String> = arg
            .target_kind
            .iter()
            .flat_map(|kinds| kinds.split('|').map(|k| k.trim().to_string()))
            .collect();
        if field_type.is_reference() {
            if targets.is_empty() {
                return Err(format!(
                    "its reference argument '{name}' names no target_kind"
                ));
            }
            if let Some(kind) = targets.iter().find(|k| !self.judge.kinds.contains(k)) {
                return Err(format!(
                    "its reference argument '{name}' names the kind '{kind}', which no loaded extension declares"
                ));
            }
        } else if !targets.is_empty() {
            return Err(format!(
                "its argument '{name}' names a target_kind and is no reference"
            ));
        }
        if (arg.minimum.is_some() || arg.maximum.is_some()) && field_type != FieldType::Integer {
            return Err(format!(
                "its argument '{name}' has bounds and is no integer"
            ));
        }
        if let (Some(least), Some(most)) = (arg.minimum, arg.maximum)
            && least > most
        {
            return Err(format!("the bounds of its argument '{name}' are empty"));
        }
        if arg.required && arg.default.is_some() {
            return Err(format!(
                "its argument '{name}' is required and has a default"
            ));
        }
        if let Some(default) = &arg.default {
            let form = FieldForm::new(field_type, arg.enum_values.clone())
                .targeting_any(&targets)
                .bounded(arg.minimum, arg.maximum);
            if form.admit_json(default).is_err() {
                return Err(format!(
                    "the default of its argument '{name}' is not a {}",
                    arg.field_type
                ));
            }
        }
        Ok(())
    }

    /// A step names at most one of a command, a prompt and an agent, of this extension or a declared peer's.
    fn step(&self, step: &PromptStep) -> Result<(), String> {
        let named = [&step.command, &step.agent, &step.prompt]
            .iter()
            .filter(|n| n.is_some())
            .count();
        if named > 1 {
            return Err(format!(
                "its step '{}' names more than one of a command, a prompt and an agent",
                step.id
            ));
        }
        if let Some(command) = &step.command {
            self.reachable(&step.id, command, |l| &l.commands, "command")?;
        }
        if let Some(prompt) = &step.prompt {
            self.reachable(&step.id, prompt, |l| &l.prompts, "prompt")?;
        }
        Ok(())
    }

    fn reachable(
        &self,
        step: &str,
        target: &str,
        of: impl Fn(&Loaded) -> &BTreeSet<String>,
        what: &str,
    ) -> Result<(), String> {
        let (owner, id) = match target.split_once(':') {
            Some((peer, id)) => (Some(peer), id),
            None => (None, target),
        };
        let Some(owner) = owner else {
            let own = self.loaded.iter().find(|l| l.name == self.extension);
            return if own.is_some_and(|l| of(l).contains(id)) {
                Ok(())
            } else {
                Err(format!(
                    "its step '{step}' names the {what} '{id}', which the extension does not declare"
                ))
            };
        };
        let declared_peer = self.peers.iter().any(|p| p == owner)
            || self
                .loaded
                .iter()
                .any(|l| (l.name == owner || l.short == owner) && self.peers.contains(&l.name));
        if !declared_peer {
            return Err(format!(
                "its step '{step}' names '{owner}', which is not a declared peer"
            ));
        }
        // A peer that is not loaded (an optional one) cannot be checked, and is not refused.
        match self
            .loaded
            .iter()
            .find(|l| l.name == owner || l.short == owner)
        {
            Some(peer) if !of(peer).contains(id) => Err(format!(
                "its step '{step}' names the {what} '{id}', which '{owner}' does not declare"
            )),
            _ => Ok(()),
        }
    }
}
