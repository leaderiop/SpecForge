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
