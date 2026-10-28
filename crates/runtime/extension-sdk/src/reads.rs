//! Declaring and reading file reads (protocol 1.11.0, ADR 0293): what a pass or command asks the host to read
//! (`reads`, `reads_tests`) and what it is given back (`files`).

use specforge_protocol_types::{FileBase, FileFacts, FileRead};

use crate::{CommandBuilder, CommandCall, PassBuilder};

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

    /// Where a field's relative paths start (the default is the project root). A field read only.
    pub fn base(mut self, base: FileBase) -> Self {
        self.0.base = Some(base.as_str().to_string());
        self
    }

    /// Send each file's text (at most 64 KiB) as well as its facts.
    pub fn text(mut self) -> Self {
        self.0.text = true;
        self
    }

    fn named(mut self, name: &str) -> FileRead {
        self.0.name = name.to_string();
        self.0
    }
}

impl PassBuilder {
    /// The pass reads `read`; its input's `files` holds the facts under `name`.
    pub fn reads(&mut self, name: &str, read: FileReadSpec) -> &mut Self {
        self.descriptor.reads.push(read.named(name));
        self
    }

    /// A check, compute or guard pass receives the recorded test report as `test_results` (an analyze pass always
    /// does). Without a report the host reports I205 once.
    pub fn reads_tests(&mut self) -> &mut Self {
        self.descriptor.reads_tests = true;
        self
    }
}

impl CommandBuilder {
    /// The command reads `read`; its input's `files` holds the facts under `name`.
    pub fn reads(&mut self, name: &str, read: FileReadSpec) -> &mut Self {
        self.descriptor.reads.push(read.named(name));
        self
    }
}

impl CommandCall<'_> {
    /// The facts of the read `name`, when the command declared it and the host sent it.
    pub fn files(&self, name: &str) -> Option<&FileFacts> {
        self.input.files.get(name)
    }
}
