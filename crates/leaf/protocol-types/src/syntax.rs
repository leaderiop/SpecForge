//! What a guest asks the host's source reader and what it answers (`specforge:host/syntax`, ADR 0345 D1): the
//! matches of a pack's query over a project's source files, read by the host (ADR 0344). The WIT interface mirrors
//! these types and each side converts at its edge; the serde form is the host's syntax cache's.

use serde::{Deserialize, Serialize};

/// One parse request: which files, read where, in which language, through which query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxRequest {
    pub source: SyntaxSource,
    pub files: SyntaxFiles,
    /// `rust`, `typescript` (also JavaScript) or `tsx` (also JSX).
    pub language: String,
    /// The pack's query, in the tags convention (ADR 0344 D1).
    pub query: String,
}

/// Where the files are read: a root of the worktree, or a commit of the project's repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "at", rename_all = "snake_case")]
pub enum SyntaxSource {
    /// `spec`, `project` or a root `specforge.json` names; the paths are from it.
    Worktree { root: String },
    /// A commit id (hex); the paths are from the repository's top.
    Revision { commit: String },
}

/// The files asked: named, or matched by a glob (the hook glob: `*`, `?`, a `**` segment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxFiles {
    Paths(Vec<String>),
    Glob(String),
}

/// The files of a request, in the order asked (a glob: sorted).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxAnswer {
    pub files: Vec<SyntaxFile>,
    /// The walk or the answer's cap stopped them: ask for the rest again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// Files this request parsed now.
    pub parsed: u32,
    /// Files this request answered from the host's syntax cache.
    pub cached: u32,
}

/// One file's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxFile {
    pub path: String,
    /// `sha256:<hex>` of the bytes parsed; none when none were read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(flatten)]
    pub outcome: SyntaxOutcome,
}

/// What a file's parse gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SyntaxOutcome {
    /// The query's matches, sorted by (start byte, end byte descending); `errors` counts the tree's ERROR and
    /// MISSING nodes: what the error-tolerant parse recovered is answered.
    Parsed {
        matches: Vec<SyntaxMatch>,
        errors: u32,
    },
    NotParsed {
        reason: NotParsed,
    },
}

/// Why a file was not parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotParsed {
    /// Its parse or its query run passed the per-file deadline.
    Deadline,
    /// Over the file ceiling (64 MiB), or a blob the git port did not read.
    TooLarge,
    Missing,
    Unreadable,
    /// A file the fs port refuses to read: a withheld name, a link out of its root, a file with a second name.
    Denied,
    /// The answer's cap was reached before it.
    Capped,
}

impl NotParsed {
    /// The reason as the wire and the diagnostics name it.
    pub const fn name(self) -> &'static str {
        match self {
            NotParsed::Deadline => "deadline",
            NotParsed::TooLarge => "too_large",
            NotParsed::Missing => "missing",
            NotParsed::Unreadable => "unreadable",
            NotParsed::Denied => "denied",
            NotParsed::Capped => "capped",
        }
    }
}

/// One match of a pattern: its role capture's role and kind and node span, and every other named capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxMatch {
    pub role: SyntaxRole,
    /// What follows `definition.` or `reference.` in the role capture's name (`function`, `impl.method`).
    pub kind: String,
    pub span: SyntaxSpan,
    /// The index in the file's matches of the innermost definition whose span holds this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    /// Every capture of the match but the role capture and those starting with `_`, in capture order.
    #[serde(default)]
    pub captures: Vec<SyntaxCapture>,
}

/// A role capture's role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxRole {
    Definition,
    Reference,
}

/// A named capture: its node's text and span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxCapture {
    pub name: String,
    /// The node's UTF-8 text (lossy), cut at 4,096 bytes on a character boundary.
    pub text: String,
    /// The text was cut.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
    pub span: SyntaxSpan,
}

/// A node's extent, its end exclusive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxSpan {
    pub start: SyntaxPoint,
    pub end: SyntaxPoint,
}

/// A place in a file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxPoint {
    /// Byte offset from the file's start, 0-based.
    pub byte: u32,
    /// 1-based line.
    pub line: u32,
    /// 1-based column, in characters: SpecForge's convention for a person.
    pub col: u32,
    /// 1-based column, in bytes: what a fact's location stores.
    pub byte_col: u32,
}
