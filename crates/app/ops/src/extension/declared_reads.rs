//! The file reads an extension declares (ADR 0293), as `specforge extensions` and `specforge doctor` list them: a
//! declared read is a capability a user should be able to see, so each is named with what it asks for.

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_protocol_types::ExtensionDeclaration;

/// One read a pass or command declares (`McpDeclaredRead`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct DeclaredRead {
    /// The pass or command that declares it: `pass:<name>` or `command:<id>`.
    pub owner: String,
    pub name: String,
    /// The `kind.field` whose paths are read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// The project-relative glob that is read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    /// Whether the file's text, not only its facts, is handed to the extension.
    pub text: bool,
}

impl DeclaredRead {
    /// The line both surfaces print: `owner name: field or glob (text)`.
    pub fn line(&self) -> String {
        let what = self.field.as_deref().or(self.glob.as_deref()).unwrap_or("");
        let text = if self.text { " (with text)" } else { "" };
        format!("{} {}: {what}{text}", self.owner, self.name)
    }
}

/// Every read the passes and commands of `declaration` declare, passes first, in declaration order.
pub fn declared_reads(declaration: &ExtensionDeclaration) -> Vec<DeclaredRead> {
    let row = |owner: String, read: &specforge_protocol_types::FileRead| DeclaredRead {
        owner,
        name: read.name.clone(),
        field: read.field.clone(),
        glob: read.glob.clone(),
        text: read.text,
    };
    let passes = declaration
        .passes
        .iter()
        .flat_map(|p| p.reads.iter().map(|r| row(format!("pass:{}", p.name), r)));
    let commands = declaration
        .surfaces
        .commands
        .iter()
        .flat_map(|c| c.reads.iter().map(|r| row(format!("command:{}", c.id), r)));
    passes.chain(commands).collect()
}
