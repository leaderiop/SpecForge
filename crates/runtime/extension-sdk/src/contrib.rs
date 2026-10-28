//! What ADR 0291 adds to the builders: prompts, the gates of a command and the codes a guard pass reports. They are
//! second `impl` blocks so the files of the builders they extend stay under the file-size cap.

use crate::prompt::{PromptBuilder, prompt_export};
use crate::{CommandBuilder, ContributionsBuilder, PassBuilder};

impl ContributionsBuilder {
    /// Contribute an MCP prompt, rendered by its handler ([`PromptBuilder::handler`]) at the export
    /// `prompt__<id>`; the host serves it as `specforge.<short>.<id>`.
    ///
    /// # Panics
    ///
    /// When the prompt declares no handler, or a prompt of that id is already declared.
    pub fn prompt(&mut self, id: &str, f: impl FnOnce(&mut PromptBuilder)) -> &mut Self {
        assert!(
            !self.prompts.iter().any(|p| p.id == id),
            "prompt '{id}' is declared twice"
        );
        let mut b = PromptBuilder::new(id);
        f(&mut b);
        let Some(render) = b.render else {
            panic!("prompt '{id}' declares no handler: render it with `p.handler(...)`");
        };
        self.add_operation(prompt_export(id), format!("prompt '{id}'"), render);
        self.prompts.push(b.descriptor);
        self
    }
}

impl CommandBuilder {
    /// The command is refused (`gate_failed`) while the local code `code` fails in its subject's scope (ADR 0291).
    pub fn gate(&mut self, code: &str) -> &mut Self {
        self.descriptor.gates.push(code.to_string());
        self
    }

    /// The arg holding the entity the command acts on; it scopes the gates, and the host sends it as a full path.
    pub fn subject(&mut self, arg: &str) -> &mut Self {
        self.descriptor.subject = Some(arg.to_string());
        self
    }
}

impl PassBuilder {
    /// A `guard` pass: the local codes it can report. A gate runs the guard passes whose codes meet its own.
    pub fn reports(&mut self, codes: &[&str]) -> &mut Self {
        self.descriptor
            .reports
            .extend(codes.iter().map(|c| c.to_string()));
        self
    }
}
