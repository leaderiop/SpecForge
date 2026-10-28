//! The prompts' routes (ADR 0291): a prompt is routed under `(short name, id with '_' spelled '-')`, the name a
//! command of the same extension takes, so `specforge <short> <name>` is never ambiguous. Commands are routed first
//! (the routes already in [`CommandRoutes`] stand), then prompts in load order: a prompt whose name a command or an
//! earlier prompt has is E039 and runs on no surface.

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::command_args::option_name;
use specforge_protocol_types::{ExtensionDeclaration, PromptDescriptor};

use super::CommandRoutes;

/// One extension prompt and the name it is routed under: `specforge <short> <cli_name>`, MCP
/// `specforge.<short>.<id>`.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptRoute {
    extension: String,
    version: String,
    short: String,
    cli_name: String,
    declaration: PromptDescriptor,
}

impl PromptRoute {
    fn new(declaration: &ExtensionDeclaration, prompt: &PromptDescriptor) -> Self {
        Self {
            extension: declaration.name().to_string(),
            version: declaration.version().to_string(),
            short: declaration.short().into_owned(),
            cli_name: option_name(&prompt.id),
            declaration: prompt.clone(),
        }
    }

    /// The extension that declares it.
    pub fn extension(&self) -> &str {
        &self.extension
    }

    /// Its extension's version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Its extension's short name.
    pub fn short(&self) -> &str {
        &self.short
    }

    /// Its name on the command line: its id, `_` spelled `-`.
    pub fn cli_name(&self) -> &str {
        &self.cli_name
    }

    /// The prompt as declared.
    pub fn declaration(&self) -> &PromptDescriptor {
        &self.declaration
    }

    fn name(&self) -> (&str, &str) {
        (&self.short, &self.cli_name)
    }
}

impl CommandRoutes {
    /// The prompts that are served, in load then declaration order.
    pub fn prompts(&self) -> &[PromptRoute] {
        &self.prompts
    }

    /// Route `prompt` of `declaration`; `Some(E039)` when a command or an earlier prompt has its name.
    pub(crate) fn add_prompt(
        &mut self,
        declaration: &ExtensionDeclaration,
        prompt: &PromptDescriptor,
    ) -> Option<Diagnostic> {
        let route = PromptRoute::new(declaration, prompt);
        let first = self
            .routed
            .iter()
            .find(|r| r.name() == route.name())
            .map(|r| (r.extension.clone(), "command", r.declaration().id.clone()))
            .or_else(|| {
                self.prompts
                    .iter()
                    .find(|r| r.name() == route.name())
                    .map(|r| (r.extension.clone(), "prompt", r.declaration.id.clone()))
            });
        let Some((owner, kind, id)) = first else {
            self.prompts.push(route);
            return None;
        };
        let by = if owner == route.extension {
            format!("as the {kind} '{id}'")
        } else {
            format!("as the {kind} '{id}' of '{owner}'")
        };
        Some(
            Diagnostic::new(
                codes::E039,
                format!(
                    "prompt '{}' of '{}' is not routed: `specforge {} {}` is routed first, {by}",
                    route.declaration.id, route.extension, route.short, route.cli_name
                ),
            )
            .with_suggestion(
                "rename the prompt or the command, or give one extension another short name (ext_short; \
                 the SDK's #[extension(short = \"...\")])"
                    .to_string(),
            ),
        )
    }
}
