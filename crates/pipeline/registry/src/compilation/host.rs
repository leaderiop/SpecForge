//! What the build holds of the agents, skills, slash commands and hooks the extensions contribute (ADR 0292), in load
//! then declaration order, once the registry has refused what it cannot serve and named the host's files one by one.

use specforge_protocol_types::{
    AgentDescriptor, HookDescriptor, SkillDescriptor, SlashCommandDescriptor,
};

use super::RegistryBuild;
use crate::judged::JudgedRule;
use crate::surface::Contributed;

impl RegistryBuild {
    fn contributed<'a, T: 'a>(
        &'a self,
        of: impl Fn(&'a specforge_protocol_types::ExtensionDeclaration) -> &'a [T] + 'a,
    ) -> impl Iterator<Item = Contributed<'a, T>> + 'a {
        self.declarations().iter().flat_map(move |d| {
            let short = d.short().into_owned();
            of(d).iter().map(move |item| Contributed {
                extension: d.name(),
                short: short.clone(),
                item,
            })
        })
    }

    /// The agents the extensions contribute.
    pub fn agents(&self) -> impl Iterator<Item = Contributed<'_, AgentDescriptor>> {
        self.contributed(|d| &d.surfaces.agents)
    }

    /// The skills the extensions contribute.
    pub fn skills(&self) -> impl Iterator<Item = Contributed<'_, SkillDescriptor>> {
        self.contributed(|d| &d.surfaces.skills)
    }

    /// The slash commands the extensions contribute; [`Contributed::name`] is the host's name for each.
    pub fn slash_commands(&self) -> impl Iterator<Item = Contributed<'_, SlashCommandDescriptor>> {
        self.contributed(|d| &d.surfaces.slash_commands)
    }

    /// The hooks the extensions contribute; [`Contributed::route`] is the one `specforge hook` takes.
    pub fn hooks(&self) -> impl Iterator<Item = Contributed<'_, HookDescriptor>> {
        self.contributed(|d| &d.surfaces.hooks)
    }

    /// The agent `id` and the extension that declares it.
    pub fn agent(&self, id: &str) -> Option<(&str, &AgentDescriptor)> {
        self.agents()
            .find(|a| a.item.id == id)
            .map(|a| (a.extension, a.item))
    }

    /// The hook routed as `route` (`<short>.<id>`).
    pub fn hook(&self, route: &str) -> Option<Contributed<'_, HookDescriptor>> {
        self.hooks().find(|h| h.route() == route)
    }

    /// The judged rules of `extension` whose judge is its prompt `prompt` (ADR 0295).
    pub fn judged_rules_of_prompt(&self, extension: &str, prompt: &str) -> Vec<&JudgedRule> {
        let judge = format!("{}{prompt}", crate::judged::PROMPT_JUDGE);
        self.rules()
            .judged()
            .filter(|r| r.extension == extension && r.judge == judge)
            .collect()
    }

    /// The judged rules whose judge is the agent `agent` of the extension that declares the rule.
    pub fn judged_rules_of(&self, agent: &str) -> Vec<&JudgedRule> {
        self.rules()
            .judged()
            .filter(|r| r.judge == agent && self.agent(agent).is_some())
            .collect()
    }
}
