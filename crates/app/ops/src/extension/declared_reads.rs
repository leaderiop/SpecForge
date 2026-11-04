//! The inputs an extension declares (ADR 0293, ADR 0337 D2) and the capabilities it declares (ADR 0335 D2.1), as
//! `specforge extensions` and `specforge doctor` list them: what an extension may reach is something a user should
//! be able to see, so each is named with what it asks for.

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_protocol_types::{ExtensionDeclaration, Input};

/// One input a pass, command, prompt, hook or collector declares (`McpDeclaredRead`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct DeclaredRead {
    /// The call that declares it: `pass:<name>`, `command:<id>`, `prompt:<id>`, `hook:<id>` or `collector:<name>`.
    pub owner: String,
    /// The input's tag: `files`, `tests`, `state`, `graph`, `slash_commands`, `git` or `setting`.
    pub input: String,
    /// A `files` input's read name, a `setting`'s `<extension>:<name>`, `status` for a `git` input that asks for it;
    /// else empty.
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
    /// The line both surfaces print: `owner name: field or glob (text)` for a file read, else `owner input [name]`.
    pub fn line(&self) -> String {
        if self.input != "files" {
            return [self.owner.as_str(), &self.input, &self.name]
                .iter()
                .filter(|part| !part.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
        }
        let what = self.field.as_deref().or(self.glob.as_deref()).unwrap_or("");
        let text = if self.text { " (with text)" } else { "" };
        format!("{} {}: {what}{text}", self.owner, self.name)
    }
}

/// Every input the passes, commands, prompts, hooks and collectors of `declaration` declare, in that order, each in
/// declaration order.
pub fn declared_inputs(declaration: &ExtensionDeclaration) -> Vec<DeclaredRead> {
    let row = |owner: &str, input: &Input| {
        let mut row = DeclaredRead {
            owner: owner.to_string(),
            input: input.tag().to_string(),
            name: String::new(),
            field: None,
            glob: None,
            text: false,
        };
        match input {
            Input::Files(read) => {
                row.name = read.name.clone();
                row.field = read.field.clone();
                row.glob = read.glob.clone();
                row.text = read.text;
            }
            Input::Setting { extension, name } => row.name = format!("{extension}:{name}"),
            Input::Git { status: true, .. } => row.name = "status".to_string(),
            _ => {}
        }
        row
    };
    let surfaces = &declaration.surfaces;
    let owners = (declaration.passes.iter())
        .map(|p| (format!("pass:{}", p.name), &p.inputs))
        .chain((surfaces.commands.iter()).map(|c| (format!("command:{}", c.id), &c.inputs)))
        .chain((surfaces.prompts.iter()).map(|p| (format!("prompt:{}", p.id), &p.inputs)))
        .chain((surfaces.hooks.iter()).map(|h| (format!("hook:{}", h.id), &h.inputs)))
        .chain(
            (declaration.collectors.iter()).map(|c| (format!("collector:{}", c.name), &c.inputs)),
        );
    owners
        .flat_map(|(owner, inputs)| {
            inputs
                .iter()
                .map(move |input| row(&owner, input))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// One capability an extension declares (`McpDeclaredCapability`): the effects it may ask the host for. Whether it
/// is granted here is `specforge grants`'s (and doctor's) to say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct DeclaredCapability {
    /// Its scope name.
    pub name: String,
    /// Its port and scope in one line (`process cargo test`, `http GET api.example.com`).
    pub scope: String,
    pub why: String,
}

impl DeclaredCapability {
    /// The line both surfaces print: `name: scope`.
    pub fn line(&self) -> String {
        format!("{}: {}", self.name, self.scope)
    }
}

/// Every sound capability `declaration` declares (one with a W021 grants nothing), in declaration order.
pub fn declared_capabilities(declaration: &ExtensionDeclaration) -> Vec<DeclaredCapability> {
    declaration
        .sound_capabilities()
        .into_iter()
        .map(|c| DeclaredCapability {
            name: c.name.clone(),
            scope: specforge_effects::scope::rules(&c.port).summary(),
            why: c.why.clone(),
        })
        .collect()
}

/// One checker an extension declares (`McpDeclaredChecker`, ADR 0336).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct DeclaredChecker {
    pub name: String,
    /// The field types it reads.
    pub applies_to: Vec<String>,
    /// The capabilities its probes ask under; absent for a pure checker.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    /// The codes its findings may carry; absent when it declares none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
}

impl DeclaredChecker {
    /// The line both surfaces print: `name (types)`, with its scopes when it has some.
    pub fn line(&self) -> String {
        let reads = format!("{} ({})", self.name, self.applies_to.join(", "));
        match self.scopes.as_slice() {
            [] => reads,
            scopes => format!("{reads} through {}", scopes.join(", ")),
        }
    }
}

/// Every checker `declaration` declares, in declaration order. The registry keeps only sound ones; this lists what the
/// extension says.
pub fn declared_checkers(declaration: &ExtensionDeclaration) -> Vec<DeclaredChecker> {
    declaration
        .checkers
        .iter()
        .map(|c| DeclaredChecker {
            name: c.name.clone(),
            applies_to: c.applies_to.clone(),
            scopes: c.scopes.clone(),
            codes: c.codes.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta, FileReadSpec, Input};

    #[test]
    fn every_input_is_listed_by_owner_collectors_last() {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/reads", "1.0.0"));
        c.collector("node", |collector| {
            collector
                .input(Input::files(
                    "manifest",
                    FileReadSpec::glob("package.json").text(),
                ))
                .collect(|_| Ok(Default::default()));
        });
        c.command("go", |cmd| {
            cmd.title("Go")
                .description("Go")
                .input(Input::files("plans", FileReadSpec::field("plan.files")))
                .input(Input::State)
                .handler(|_| specforge_extension_sdk::CommandOutput::ok("go"));
        });

        c.hook(
            "guard",
            specforge_extension_sdk::prelude::HookEvent::PreTool,
            |h| {
                h.input(Input::git().status())
                    .input(Input::setting("@gsd/security", "x.y"))
                    .handler(|_| Ok(Default::default()));
            },
        );

        let owners: Vec<String> = declared_inputs(&c.declaration())
            .iter()
            .map(DeclaredRead::line)
            .collect();

        assert_eq!(
            owners,
            [
                "command:go plans: plan.files",
                "command:go state",
                "hook:guard git status",
                "hook:guard setting @gsd/security:x.y",
                "collector:node manifest: package.json (with text)"
            ]
        );
    }
}
