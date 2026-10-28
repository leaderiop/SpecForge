//! The rules a rendered prompt judges (ADR 0295): the analogue of the section an agent's body lists. A prompt that
//! judges none carries no `judged` key, so no prompt a project has changes.

use serde::Serialize;

use super::ExtensionPrompt;
use crate::view::ProjectView;

/// One judged rule whose judge is the prompt (`McpExtensionPromptPayload.judged`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JudgedRuleRef {
    /// The rule's qualified code: the verdict's name.
    pub rule: String,
    pub outcomes: Vec<String>,
    pub passing: Vec<String>,
}

/// The rules `prompt` judges, in code order.
pub(super) fn of(view: &ProjectView<'_>, prompt: &ExtensionPrompt) -> Vec<JudgedRuleRef> {
    view.registries()
        .judged_rules_of_prompt(prompt.extension(), &prompt.declaration.id)
        .into_iter()
        .map(|rule| JudgedRuleRef {
            rule: rule.code.clone(),
            outcomes: rule.outcomes.clone(),
            passing: rule.passing.clone(),
        })
        .collect()
}
