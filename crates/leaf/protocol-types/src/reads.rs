//! File reads (protocol 1.11.0, ADR 0293): the project files a pass or command declares it reads, and the facts and
//! capped text its input carries for them. A guest has no file system; the host reads and hands over.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A project file read a pass or command declares (`reads`): the host reads the files it names and sends their facts
/// under `name` in the input's `files`.
///
/// Exactly one of `field` and `glob` names the files; a read that names neither or both, a `name` that is no word or
/// repeats, a `root` on a glob, or a `text_max` that is zero, over [`TEXT_MAX_CEILING_BYTES`] or without `text` is W021
/// and served empty.
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
    /// The root a field's bare paths start at: `spec`, `project` (the default; a path in a spec names a project file)
    /// or a root `specforge.json` names; a value's own `<root>/` prefix overrides it. A string, as a name that is no
    /// root's costs the read, not the declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Send each file's text (UTF-8, at most 64 KiB, or `text_max`) as well as its facts.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub text: bool,
    /// The most text of one file this read sends, in bytes, instead of 64 KiB; at most 4 MiB, and only with `text`
    /// (protocol 2.3.0). The read's text budget is then the larger of 1 MiB and four times it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_max: Option<u32>,
}

/// The most files one [`FileRead`] names: past it the host reads that many and says [`FileFacts::truncated`].
pub const FILE_READ_MAX: usize = 2000;

/// The most entries (files and directories, matched or not) the walk of one glob visits: past it the walk stops and
/// the host says [`FileFacts::truncated`], whatever it matched so far.
pub const GLOB_VISITS_MAX: usize = 20_000;

/// The largest [`FileRead::text_max`] (4 MiB): the registry refuses a larger one (W021) and the host never sends more.
pub const TEXT_MAX_CEILING_BYTES: u32 = 4 * 1024 * 1024;

/// What the host read for one [`FileRead`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFacts {
    /// One per (entity, path), sorted by `(entity, path)`; a glob's by `path`.
    #[serde(default)]
    pub facts: Vec<FileFact>,
    /// The read stopped early: it named more files than the host reads ([`FILE_READ_MAX`]), or its glob's walk
    /// reached [`GLOB_VISITS_MAX`] entries. The guest cannot tell which.
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
