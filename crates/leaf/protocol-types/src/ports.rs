//! What a guest's `specforge:host` port call answers (ADR 0335 D4), in the kernel's terms: the WIT package mirrors
//! these, and each side converts at its edge. The requests are [`crate::effects`]'; the answers are here because a
//! guest reads them, and an answer holds what may reach a guest and nothing else (ADR 0288).

/// The host refused the call before any effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denial {
    /// This call kind may not use the port: the message says who may.
    CallKind(String),
    /// The request is outside the extension's grant for this call: the message names what was asked.
    Scope(String),
}

/// The port did not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The outcome class (`unreachable:dns`, `deadline`, `missing`, ...), `not_performed` when the call's effects
    /// mode performs no such effect, `not_in_journal` when `replay` has no record of it.
    pub class: String,
    pub message: String,
}

/// Why a port call has no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    Denied(Denial),
    Failed(Failure),
}

/// An HTTP answer that came, whatever its status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    /// The headers the request's `keep_headers` names, lowercased, in the order they came.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// The body stopped at a cap.
    pub truncated: bool,
    /// The URL that answered, after the redirects followed.
    pub final_url: String,
}

/// What a path names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsEntryKind {
    File,
    Dir,
    Other,
}

/// A path's kind, size and modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsEntry {
    pub kind: FsEntryKind,
    pub size: u64,
    pub modified_ms: Option<u64>,
}

/// A file's bytes, up to a cap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FsContent {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

/// A walk's matches: root-relative, `/` separated, sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FsListing {
    pub entries: Vec<String>,
    pub truncated: bool,
}

/// A program's run that ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessOutput {
    /// None: it was stopped at the output cap.
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// The output stopped at a cap.
    pub truncated: bool,
}

/// Where the project's repository is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitRepo {
    pub toplevel: String,
    /// A shallow clone: history before its boundary is not there.
    pub shallow: bool,
}

/// An entry of a commit's tree, from the repository's top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTreeEntry {
    pub path: String,
    /// A regular file (not a link or a submodule).
    pub file: bool,
}

/// A file asked of a commit: its size when the commit holds it as a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSize {
    pub path: String,
    pub size: Option<u64>,
}

/// A file asked of a commit: its bytes when the commit holds it as a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBlob {
    pub path: String,
    pub bytes: Option<Vec<u8>>,
}

/// The files asked of a commit, in the order asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitBlobs {
    pub blobs: Vec<GitBlob>,
    /// The answer's cap stopped them: the blobs after it have no bytes.
    pub truncated: bool,
}

/// One changed range of a text file's diff, with no context: the lines `before_start..before_end` of the old side
/// became `after_start..after_end` of the new side (0-based, end exclusive). An empty side is a pure addition or a
/// pure deletion, placed before that side's line `*_start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitHunk {
    pub before_start: u32,
    pub before_end: u32,
    pub after_start: u32,
    pub after_end: u32,
}

/// The commits a guest's log walks: those `tips` reach that `hidden` do not, newest first. Commits are named by id and
/// paths from the repository's top; the host decides the repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitLogQuery {
    pub tips: Vec<String>,
    pub hidden: Vec<String>,
    /// Only the commits that change something at one of these paths. Empty keeps every commit.
    pub paths: Vec<String>,
    /// Follow only each commit's first parent.
    pub first_parent: bool,
    /// At most this many listed (the newest, or the oldest with `oldest`); none lists every commit the answer holds.
    pub max: Option<u32>,
    pub oldest: bool,
    /// Only the commits a line of whose message this regex matches.
    pub grep: Option<String>,
    /// Only the commits whose committer date is at or after (`since`) or at or before (`until`) this time, seconds
    /// since the epoch.
    pub since: Option<i64>,
    pub until: Option<i64>,
    /// No commit with more than one parent.
    pub no_merges: bool,
    /// Only the commits that change how many times this regex matches in a file of `paths`; a withheld file is never
    /// read.
    pub pickaxe: Option<String>,
}

/// Who made a commit, and when: a name and epoch seconds, never an email address nor the signer's offset (ADR 0352).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitSignature {
    pub name: String,
    /// Seconds since the epoch.
    pub seconds: i64,
}

/// A trailer of a message's last paragraph: every value of its token, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitTrailer {
    pub token: String,
    pub values: Vec<String>,
}

/// One commit of a log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitLogCommit {
    pub sha: String,
    pub parents: Vec<String>,
    pub author: GitSignature,
    pub committer: GitSignature,
    /// The message's first paragraph on one line.
    pub subject: String,
    /// The rest of the message, trailers included, trimmed.
    pub body: String,
    /// By token.
    pub trailers: Vec<GitTrailer>,
}

/// What a log walked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitLogAnswer {
    /// Every commit the log keeps; none when a pickaxe stopped past its listing, or the host's counted walk stopped.
    pub count: Option<u64>,
    pub commits: Vec<GitLogCommit>,
    /// The answer's cap stopped the listing (older commits were kept but not listed), or the host's counted walk
    /// stopped (ADR 0352).
    pub truncated: bool,
}

/// The side a diff compares its base with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDiffHead {
    Commit(String),
    /// The index.
    Index,
    /// The tracked files as they are on disk.
    Worktree,
}

/// A diff a guest asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffQuery {
    /// The commit compared from; none for the empty tree (a root commit's diff).
    pub base: Option<String>,
    pub head: GitDiffHead,
    /// Only the files at or under these paths. Empty keeps every file.
    pub paths: Vec<String>,
    /// Each text file's unified hunks, three lines of context.
    pub patch: bool,
    /// Each text file's changed line ranges, no context.
    pub hunks: bool,
    /// Pair a deletion with an addition by the host's rules, never the repository's config (ADR 0352); false lists them
    /// apart.
    pub renames: bool,
}

/// One changed file of a guest's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileChange {
    pub path: String,
    /// The path it had at the base, for a rename or a copy.
    pub old_path: Option<String>,
    pub status: crate::GitChange,
    pub added: u32,
    pub removed: u32,
    /// Either side is binary: no lines are counted.
    pub binary: bool,
    /// A side is over the host's blob cap, or past the bytes the host reads for one diff (no lines are counted), or its
    /// patch was cut.
    pub truncated: bool,
    /// Its text is not handed to this guest: a withheld name (ADR 0293 D3a), or a worktree file its fs grant does not
    /// read (ADR 0352). No patch and no hunks; its status and counts stay.
    pub withheld: bool,
    pub patch: Option<String>,
    pub hunks: Option<Vec<GitHunk>>,
}

/// A guest's diff, by path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitDiffAnswer {
    pub files: Vec<GitFileChange>,
    /// The answer's cap stopped the patches and hunks (the files after it have none), or the host's counted read stopped
    /// (the files past it count no lines).
    pub truncated: bool,
}
