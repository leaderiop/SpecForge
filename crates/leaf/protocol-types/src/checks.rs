//! Value checks on the wire (ADR 0336): what a checker declares, what a field names, and the `__check_values` call. A
//! checker plans the host's probes from a value and judges their outcomes; it never performs an effect itself.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{FieldDescriptor, Header, OutcomeClass, Root};

/// The export one extension's checkers are all called by; the guest routes each item by its checker's name.
pub const CHECK_VALUES_EXPORT: &str = "__check_values";

/// What became of one check on one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    /// Not performed or not answered: not judged, and nothing goes red for it.
    NotRun,
}

impl CheckStatus {
    /// Its name in every output.
    pub fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Passed => "passed",
            CheckStatus::Failed => "failed",
            CheckStatus::NotRun => "not_run",
        }
    }
}

/// One checker's reading of one value, as a collector that declares `check_outcomes` receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOutcome {
    pub entity: String,
    /// The written path of the field or member (`links[1].href`).
    pub field: String,
    pub item: u32,
    pub value: String,
    /// `<extension>:<checker>`.
    pub checker: String,
    pub status: CheckStatus,
    /// The class of what its probes answered; absent when it did not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<OutcomeClass>,
    /// When its oldest journaled answer was observed (Unix milliseconds, the journal record's); absent when no answer
    /// has a record: a pure checker, or one that read only the disk or git.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<u64>,
    #[serde(default)]
    pub from_cache: bool,
    /// The digest of what its probes answered, free of when they were observed (`sha256:<hex>`); absent when it did
    /// not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// The codes it found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
}

/// A check a field names: one checker, or a value that passes when one branch does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CheckRef {
    One {
        /// `<extension>:<checker>`, or a checker of the declaring extension by its name.
        checker: String,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        args: Map<String, Value>,
    },
    /// One level: each branch is a [`CheckRef::One`] (W189 otherwise).
    AnyOf { any_of: Vec<CheckRef> },
}

impl CheckRef {
    /// The check `checker`, with no arguments.
    pub fn named(checker: &str) -> Self {
        CheckRef::One {
            checker: checker.to_string(),
            args: Map::new(),
        }
    }

    /// The check `checker` with `args`; anything but a JSON object has no arguments.
    pub fn with(checker: &str, args: Value) -> Self {
        CheckRef::One {
            checker: checker.to_string(),
            args: match args {
                Value::Object(args) => args,
                _ => Map::new(),
            },
        }
    }
}

/// What a checker's values are, for the editor only: it changes no check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueForm {
    #[default]
    Text,
    Url,
    Path,
    /// A form this host does not read: the checker is W189, not the whole describe payload.
    #[serde(other)]
    Unknown,
}

/// A value checker an extension declares.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckerDescriptor {
    /// `snake_case`, unique in its extension.
    pub name: String,
    /// The field types it reads: `string`, `string_list`, `identifier`, `date`, `integer`.
    #[serde(default)]
    pub applies_to: Vec<String>,
    /// Its arguments, in the forms settings declare theirs. None may be named `severity`: a gentler reading is an enum
    /// argument that selects another code.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<FieldDescriptor>,
    /// The capabilities its probes use, by scope name; none: a pure checker.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    /// Every code a finding of it may carry, each catalogued (ADR 0286).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
    #[serde(default, skip_serializing_if = "is_text")]
    pub value: ValueForm,
    /// A `path` checker's argument that names a bare path's default root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_arg: Option<String>,
    /// Bool settings of the extension: while one is false the checker is not called, sends no probe and asks no scope.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        with = "crate::contrib::when_names"
    )]
    pub when: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

fn is_text(value: &ValueForm) -> bool {
    *value == ValueForm::Text
}

/// Which half of a check a `__check_values` call asks for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckPhase {
    /// Findings from the text alone, and the probes the checker needs performed.
    #[default]
    Plan,
    /// Findings from what the probes answered.
    Judge,
}

/// What the host sends `__check_values`. The extension's settings are carried beside these members, as on every call.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckValuesInput {
    pub phase: CheckPhase,
    pub items: Vec<CheckItem>,
}

/// One value under one check of its field.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckItem {
    /// Names the item in the answer; unique in its call.
    pub id: u32,
    /// The checker, by the name it declared.
    pub checker: String,
    pub entity: String,
    /// The declared path of the field or member (`requires.phase`).
    pub field: String,
    /// Its place in a list; 0 for a single value.
    #[serde(default)]
    pub item: u32,
    pub text: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub args: Map<String, Value>,
    /// What this item's probes answered: the judge phase's only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<ProbeOutcome>,
}

/// What `__check_values` answers, in either phase.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckPlan {
    #[serde(default)]
    pub items: Vec<PlannedItem>,
}

/// One item's answer. A judge phase's has no probes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedItem {
    pub id: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<CheckFinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub probes: Vec<Probe>,
}

impl PlannedItem {
    /// A plan that needs `probes` performed.
    pub fn probes(probes: Vec<Probe>) -> Self {
        PlannedItem {
            probes,
            ..Default::default()
        }
    }

    /// A plan that settles the value with `findings`.
    pub fn findings(findings: Vec<CheckFinding>) -> Self {
        PlannedItem {
            findings,
            ..Default::default()
        }
    }
}

/// What a checker finds about a value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFinding {
    /// One of the codes its checker declares; another is W150, kept under the extension's namespace.
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// The byte range inside the text, when the finding is about part of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<[usize; 2]>,
}

impl CheckFinding {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        CheckFinding {
            code: code.to_string(),
            message: message.into(),
            ..Default::default()
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Point at the bytes `start..end` of the value's text.
    pub fn at(mut self, start: usize, end: usize) -> Self {
        self.at = Some([start, end]);
        self
    }
}

/// An effect a checker asks the host to perform. A `root` is the bare path's default and `path` the written text: the
/// host alone reads a `<root>/` prefix and decides where a path may lead.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "probe")]
pub enum Probe {
    #[serde(rename = "fs.stat")]
    FsStat { root: Root, path: String },
    /// The bytes `range` names: a prefix is held to 64 KiB, any other range to 1 MiB. A file of any size is read,
    /// since no range holds more than that.
    #[serde(rename = "fs.read")]
    FsRead {
        root: Root,
        path: String,
        range: ByteRange,
    },
    /// The SHA-256 of a whole file, of any size: the host streams it, bounded by the run's time.
    #[serde(rename = "fs.digest")]
    FsDigest { root: Root, path: String },
    #[serde(rename = "fs.walk")]
    FsWalk {
        root: Root,
        path: String,
        glob: String,
    },
    /// Follows at most `redirects`, held to 5, each under the grant. A 405 or 501 is asked once more as a GET.
    #[serde(rename = "http.head")]
    HttpHead {
        url: String,
        #[serde(default)]
        redirects: u32,
    },
    /// The bytes `range` names, held to 1 MiB: a range other than `whole` is asked with a `Range` header, and a server
    /// that answers the whole body anyway is sliced when the body is within the cap (`range:ignored` otherwise).
    #[serde(rename = "http.get")]
    HttpGet { url: String, range: ByteRange },
    #[serde(rename = "http.tls")]
    HttpTls { host: String, port: u16 },
    /// An operation of the Http port, under the http capability's host scope: there is no dns capability.
    #[serde(rename = "dns.resolve")]
    DnsResolve { host: String },
    #[serde(rename = "git.object")]
    GitObject { rev: String },
    #[serde(rename = "git.ref")]
    GitRef { name: String },
    /// A file at a revision, its `path` from the repository's top: never `.`, `..` or absolute.
    #[serde(rename = "git.file")]
    GitFile { rev: String, path: String },
    #[serde(rename = "git.tracked")]
    GitTracked { root: Root, path: String },
}

/// Which bytes a read asks for. The host holds each to its cap and answers where the bytes start and how long the whole
/// is, so one shape says which bytes on a file and on a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ByteRange {
    /// From the start, at most `max` bytes; a request names no range.
    Whole { max: u64 },
    /// The first `len` bytes.
    Prefix { len: u32 },
    /// The last `len` bytes: where a PDF keeps its trailer.
    Suffix { len: u32 },
    /// `len` bytes from `offset`.
    Span { offset: u64, len: u32 },
}

impl ByteRange {
    /// The most bytes it asks for.
    pub fn len(self) -> u64 {
        match self {
            ByteRange::Whole { max } => max,
            ByteRange::Prefix { len } | ByteRange::Suffix { len } | ByteRange::Span { len, .. } => {
                u64::from(len)
            }
        }
    }

    /// Whether it asks for no byte.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The same range, asking for at most `cap` bytes.
    pub fn held_to(self, cap: u32) -> ByteRange {
        let len = |len: u32| len.min(cap);
        match self {
            ByteRange::Whole { max } => ByteRange::Whole {
                max: max.min(u64::from(cap)),
            },
            ByteRange::Prefix { len: n } => ByteRange::Prefix { len: len(n) },
            ByteRange::Suffix { len: n } => ByteRange::Suffix { len: len(n) },
            ByteRange::Span { offset, len: n } => ByteRange::Span {
                offset,
                len: len(n),
            },
        }
    }

    /// The bytes it names of a whole `length` bytes long: empty past the end.
    pub fn within(self, length: u64) -> std::ops::Range<u64> {
        let start = match self {
            ByteRange::Whole { .. } | ByteRange::Prefix { .. } => 0,
            ByteRange::Suffix { len } => length.saturating_sub(u64::from(len)),
            ByteRange::Span { offset, .. } => offset.min(length),
        };
        start..start.saturating_add(self.len()).min(length)
    }

    /// The `Range` header's value that asks for it; none for [`ByteRange::Whole`].
    pub fn header(self) -> Option<String> {
        match self {
            ByteRange::Whole { .. } => None,
            ByteRange::Suffix { len } => Some(format!("bytes=-{len}")),
            ByteRange::Prefix { .. } | ByteRange::Span { .. } => {
                let range = self.within(u64::MAX);
                Some(format!(
                    "bytes={}-{}",
                    range.start,
                    range.end.saturating_sub(1)
                ))
            }
        }
    }
}

/// What one performed probe answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub probe: Probe,
    pub class: OutcomeClass,
    #[serde(default)]
    pub data: OutcomeData,
    /// Unix milliseconds.
    pub observed_at: u64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_cache: bool,
}

/// The facts of a probe's answer, beside its class. Bytes are JSON numbers, as a request's stdin and body are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutcomeData {
    /// The class says it all.
    #[default]
    None,
    Stat {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entry: Option<StatEntry>,
        #[serde(default)]
        size: u64,
        /// A missing path's closest sibling, listed only when its parent is inside the root.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        near: Option<String>,
    },
    Bytes {
        bytes: Vec<u8>,
        /// Where they start in the file.
        #[serde(default, skip_serializing_if = "is_zero")]
        start: u64,
        /// The whole file's length.
        length: u64,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
    Digest {
        /// Lowercase hex.
        sha256: String,
        length: u64,
    },
    Walk {
        entries: Vec<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
    Http {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<u16>,
        final_url: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        headers: Vec<Header>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        body: Vec<u8>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
        /// Where a ranged body starts in the whole.
        #[serde(default, skip_serializing_if = "is_zero")]
        start: u64,
        /// The whole's length, when the server said it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total_len: Option<u64>,
    },
    Resolve {
        addresses: Vec<String>,
    },
    Tls {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        not_after: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        issuer: Option<String>,
        chain_valid: bool,
    },
    /// An object, a ref or a tracked path: whether it is there, and the object it names.
    Git {
        found: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        oid: Option<String>,
    },
}

/// What a path names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatEntry {
    File,
    Dir,
}
