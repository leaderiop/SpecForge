//! Declaring a call's inputs (protocol 2.0.0, ADR 0337 D2) and reading the files it is given back (`files`, ADR 0293).

use specforge_protocol_types::{FileFacts, FileRead, Input, Root};

use crate::{
    CollectorBuilder, CommandBuilder, CommandCall, HookBuilder, PassBuilder, PromptBuilder,
};

/// A file read under construction: the files of a field, or a glob, with optional text.
///
/// ```
/// # use specforge_extension_sdk::FileReadSpec;
/// let modified = FileReadSpec::field("plan.files_modified").text();
/// let sources = FileReadSpec::glob("src/**/*.rs");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadSpec(FileRead);

impl FileReadSpec {
    /// The paths the entities of `kind.field` (or `kind.field.member`) write there.
    pub fn field(path: &str) -> Self {
        Self(FileRead {
            field: Some(path.to_string()),
            ..FileRead::default()
        })
    }

    /// The files a project-relative glob matches (`*`, `?`, a `**` segment).
    pub fn glob(pattern: &str) -> Self {
        Self(FileRead {
            glob: Some(pattern.to_string()),
            ..FileRead::default()
        })
    }

    /// The root a field's bare paths start at (the default is the project root). A field read only.
    pub fn root(mut self, root: Root) -> Self {
        self.0.root = Some(root.as_str().to_string());
        self
    }

    /// Send each file's text (at most 64 KiB) as well as its facts.
    pub fn text(mut self) -> Self {
        self.0.text = true;
        self
    }
}

impl From<FileReadSpec> for FileRead {
    fn from(spec: FileReadSpec) -> Self {
        spec.0
    }
}

/// What a call declares the host serves it (ADR 0337 D2): each builder of a call kind that takes inputs has one
/// `input` method, and the registry's table says which inputs its kind is served (another is W021 and served empty).
macro_rules! takes_inputs {
    ($($builder:ty),*) => {$(
        impl $builder {
            /// The host serves the call `input` before it runs: `Input::files(name, FileReadSpec::glob(..))`,
            /// `Input::Tests`, `Input::State`, `Input::Graph`, `Input::SlashCommands`, `Input::git().status()`,
            /// `Input::setting(extension, name)`, `Input::CheckOutcomes`.
            pub fn input(&mut self, input: Input) -> &mut Self {
                self.descriptor.inputs.push(input);
                self
            }
        }
    )*};
}

takes_inputs!(
    PassBuilder,
    CommandBuilder,
    CollectorBuilder,
    PromptBuilder,
    HookBuilder
);

impl CommandCall<'_> {
    /// The facts of the read `name`, when the command declared it and the host sent it.
    pub fn files(&self, name: &str) -> Option<&FileFacts> {
        self.input.files.get(name)
    }

    /// The host's clock when the command was called: `YYYY-MM-DDTHH:MM:SSZ`, RFC 3339 UTC with seconds (protocol
    /// 1.11.0, ADR 0294); the same instant as [`Self::today`]. Empty when the host passed none.
    pub fn now(&self) -> &str {
        &self.input.now
    }
}
