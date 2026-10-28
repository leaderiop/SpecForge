//! File reads (protocol 1.11.0, ADR 0293): the project files a pass or command declares it reads, and the facts and
//! capped text its input carries for them. A guest has no file system; the host reads and hands over.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A project file read a pass or command declares (`reads`): the host reads the files it names and sends their facts
/// under `name` in the input's `files`.
///
/// Exactly one of `field` and `glob` names the files; a read that names neither or both, a `name` that is no word or
/// repeats, or a `base` on a glob is W021 and served empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRead {
    /// The key of this read in the input's `files`: a word `[a-z][a-z0-9_]*`, unique among the owner's reads.
    #[serde(default)]
    pub name: String,
    /// `kind.field` or `kind.field.member`: the paths the entities of that kind write there, a scalar's text or each
    /// item of a list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// A project-relative, `/` separated glob: `*`, `?` and a `**` segment (the hook globs of ADR 0292). It is always
    /// from the project root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    /// Where a field's relative paths start: `spec` (the spec root) or `project` (the default; a path in a spec names
    /// a project file). A string, as a field's `file_base` is: an unknown base costs the read, not the declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Send each file's text (UTF-8, at most 64 KiB) as well as its facts.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub text: bool,
}

/// What the host read for one [`FileRead`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFacts {
    /// One per (entity, path), sorted by `(entity, path)`; a glob's by `path`.
    #[serde(default)]
    pub facts: Vec<FileFact>,
    /// The glob matched more files than the host reads (2000).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

/// One file a read names.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFact {
    /// Project-relative, `/` separated, lexically normalised: the key a pass looks a file up by.
    pub path: String,
    /// The full path of the entity whose field named the file; absent for a glob.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// The disk says the path exists (a link is followed).
    #[serde(default)]
    pub exists: bool,
    /// Present for a regular file that was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// `\n` count plus one for a last line without one; present with `bytes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<u64>,
    /// `sha256:<hex>` of the CRLF-normalised bytes, as a judged rule and a verdict digest a file; present with `bytes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// The file as UTF-8, only when the read asks for `text` and the file is within the cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Why facts or text are missing: `too_large`, `not_utf8`, `unreadable`, `outside_project` or `budget`. A path
    /// that does not exist is `exists: false`, not an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The `files` of an input: each read's [`FileFacts`] by the read's name.
pub type FileMap = BTreeMap<String, FileFacts>;
