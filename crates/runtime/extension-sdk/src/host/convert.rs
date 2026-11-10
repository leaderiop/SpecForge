//! The protocol's types and what wit-bindgen generates, at the guest's edge (ADR 0335 D4). A request goes out as WIT
//! and an answer comes back; the reverse directions exist for the round-trip test that holds the two together.

use super::wit::git::{Blob, BlobsAnswer, Repo, StatusAnswer, TreeEntry};
use super::wit::http::{self, Request, Response};
use super::wit::process::{self, RunOutput, RunRequest};
use super::wit::{fs, git, syntax, types};
use super::{
    Denial, Failure, FileSize, FsContent, FsEntry, FsEntryKind, FsListing, GitBlob, GitBlobs,
    GitChange, GitDiffAnswer, GitDiffHead, GitDiffQuery, GitEntry, GitEntryKind, GitFileChange,
    GitHunk, GitLogAnswer, GitLogCommit, GitLogQuery, GitRepo, GitSignature, GitStatus, GitTrailer,
    GitTreeEntry, HostError, HttpRequest, HttpResponse, NotParsed, ProcessOutput, ProcessRequest,
    SyntaxAnswer, SyntaxCapture, SyntaxFile, SyntaxFiles, SyntaxMatch, SyntaxOutcome, SyntaxPoint,
    SyntaxRequest, SyntaxRole, SyntaxSource, SyntaxSpan,
};
use specforge_protocol_types::{GitBranch, Header};

impl From<&HttpRequest> for Request {
    /// What only the host sets (a private-address opt-in, an attached secret) has no place here.
    fn from(request: &HttpRequest) -> Self {
        Request {
            method: request.method.clone(),
            url: request.url.clone(),
            headers: request.headers.iter().map(Into::into).collect(),
            body: request.body.clone(),
            body_cap: request.body_cap,
            timeout_ms: request.timeout_ms,
            max_redirects: request.max_redirects,
            keep_headers: request.keep_headers.clone(),
        }
    }
}

impl From<&Header> for http::Header {
    fn from(header: &Header) -> Self {
        http::Header {
            name: header.name.clone(),
            value: header.value.clone(),
            sensitive: header.sensitive,
        }
    }
}

impl From<Response> for HttpResponse {
    fn from(response: Response) -> Self {
        HttpResponse {
            status: response.status,
            headers: response
                .headers
                .into_iter()
                .map(|h| (h.name, h.value))
                .collect(),
            body: response.body,
            truncated: response.truncated,
            final_url: response.final_url,
        }
    }
}

impl From<types::HostError> for HostError {
    fn from(error: types::HostError) -> Self {
        match error {
            types::HostError::Denied(types::Denial::CallKind(why)) => {
                HostError::Denied(Denial::CallKind(why))
            }
            types::HostError::Denied(types::Denial::Scope(why)) => {
                HostError::Denied(Denial::Scope(why))
            }
            types::HostError::Failed(failure) => HostError::Failed(Failure {
                class: failure.class,
                message: failure.message,
            }),
        }
    }
}

impl From<fs::Entry> for FsEntry {
    fn from(entry: fs::Entry) -> Self {
        FsEntry {
            kind: match entry.kind {
                fs::EntryKind::File => FsEntryKind::File,
                fs::EntryKind::Dir => FsEntryKind::Dir,
                fs::EntryKind::Other => FsEntryKind::Other,
            },
            size: entry.size,
            modified_ms: entry.modified_ms,
        }
    }
}

impl From<fs::Content> for FsContent {
    fn from(content: fs::Content) -> Self {
        FsContent {
            bytes: content.bytes,
            truncated: content.truncated,
        }
    }
}

impl From<fs::Listing> for FsListing {
    fn from(listing: fs::Listing) -> Self {
        FsListing {
            entries: listing.entries,
            truncated: listing.truncated,
        }
    }
}

impl From<&ProcessRequest> for RunRequest {
    fn from(request: &ProcessRequest) -> Self {
        RunRequest {
            argv: request.argv.clone(),
            env: request
                .env
                .iter()
                .map(|var| process::EnvVar {
                    name: var.name.clone(),
                    value: var.value.clone(),
                })
                .collect(),
            stdin: request.stdin.clone(),
            deadline_ms: request.deadline_ms,
            output_cap: request.output_cap,
            text: request.text,
        }
    }
}

impl From<RunOutput> for ProcessOutput {
    fn from(output: RunOutput) -> Self {
        ProcessOutput {
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.truncated,
        }
    }
}

impl From<Repo> for GitRepo {
    fn from(repo: Repo) -> Self {
        GitRepo {
            toplevel: repo.toplevel,
            shallow: repo.shallow,
        }
    }
}

impl From<TreeEntry> for GitTreeEntry {
    fn from(entry: TreeEntry) -> Self {
        GitTreeEntry {
            path: entry.path,
            file: entry.file,
        }
    }
}

impl From<git::FileSize> for FileSize {
    fn from(sized: git::FileSize) -> Self {
        FileSize {
            path: sized.path,
            size: sized.size,
        }
    }
}

impl From<Blob> for GitBlob {
    fn from(blob: Blob) -> Self {
        GitBlob {
            path: blob.path,
            bytes: blob.bytes,
        }
    }
}

impl From<BlobsAnswer> for GitBlobs {
    fn from(answer: BlobsAnswer) -> Self {
        GitBlobs {
            blobs: answer.blobs.into_iter().map(Into::into).collect(),
            truncated: answer.truncated,
        }
    }
}

impl From<git::Change> for GitChange {
    fn from(change: git::Change) -> Self {
        match change {
            git::Change::Modified => GitChange::Modified,
            git::Change::TypeChanged => GitChange::TypeChanged,
            git::Change::Added => GitChange::Added,
            git::Change::Deleted => GitChange::Deleted,
            git::Change::Renamed => GitChange::Renamed,
            git::Change::Copied => GitChange::Copied,
            git::Change::Unmerged => GitChange::Unmerged,
        }
    }
}

impl From<git::Entry> for GitEntry {
    fn from(entry: git::Entry) -> Self {
        GitEntry {
            kind: match entry.kind {
                git::EntryKind::Changed => GitEntryKind::Changed,
                git::EntryKind::Renamed => GitEntryKind::Renamed,
                git::EntryKind::Unmerged => GitEntryKind::Unmerged,
                git::EntryKind::Untracked => GitEntryKind::Untracked,
                git::EntryKind::Ignored => GitEntryKind::Ignored,
            },
            path: entry.path,
            orig_path: entry.orig_path,
            index: entry.index.map(Into::into),
            worktree: entry.worktree.map(Into::into),
        }
    }
}

impl From<StatusAnswer> for GitStatus {
    fn from(status: StatusAnswer) -> Self {
        GitStatus {
            branch: GitBranch {
                oid: status.branch.oid,
                head: status.branch.head,
                upstream: status.branch.upstream,
                ahead: status.branch.ahead,
                behind: status.branch.behind,
            },
            entries: status.entries.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&GitLogQuery> for git::LogQuery {
    fn from(query: &GitLogQuery) -> Self {
        git::LogQuery {
            tips: query.tips.clone(),
            hidden: query.hidden.clone(),
            paths: query.paths.clone(),
            first_parent: query.first_parent,
            max: query.max,
            oldest: query.oldest,
            grep: query.grep.clone(),
            since: query.since,
            until: query.until,
            no_merges: query.no_merges,
            pickaxe: query.pickaxe.clone(),
        }
    }
}

impl From<git::Signature> for GitSignature {
    fn from(signature: git::Signature) -> Self {
        GitSignature {
            name: signature.name,
            email: signature.email,
            time: signature.time,
            seconds: signature.seconds,
        }
    }
}

impl From<git::LogCommit> for GitLogCommit {
    fn from(commit: git::LogCommit) -> Self {
        GitLogCommit {
            sha: commit.sha,
            parents: commit.parents,
            author: commit.author.into(),
            committer: commit.committer.into(),
            subject: commit.subject,
            body: commit.body,
            trailers: commit
                .trailers
                .into_iter()
                .map(|t| GitTrailer {
                    token: t.token,
                    values: t.values,
                })
                .collect(),
        }
    }
}

impl From<git::LogAnswer> for GitLogAnswer {
    fn from(answer: git::LogAnswer) -> Self {
        GitLogAnswer {
            count: answer.count,
            commits: answer.commits.into_iter().map(Into::into).collect(),
            truncated: answer.truncated,
        }
    }
}

impl From<&GitDiffQuery> for git::DiffQuery {
    fn from(query: &GitDiffQuery) -> Self {
        git::DiffQuery {
            base: query.base.clone(),
            head: match &query.head {
                GitDiffHead::Commit(sha) => git::DiffHead::Commit(sha.clone()),
                GitDiffHead::Index => git::DiffHead::Index,
                GitDiffHead::Worktree => git::DiffHead::Worktree,
            },
            paths: query.paths.clone(),
            patch: query.patch,
            hunks: query.hunks,
            renames: query.renames,
        }
    }
}

impl From<git::FileChange> for GitFileChange {
    fn from(file: git::FileChange) -> Self {
        GitFileChange {
            path: file.path,
            old_path: file.old_path,
            status: file.status.into(),
            added: file.added,
            removed: file.removed,
            binary: file.binary,
            truncated: file.truncated,
            withheld: file.withheld,
            patch: file.patch,
            hunks: file.hunks.map(|hunks| {
                hunks
                    .into_iter()
                    .map(|h| GitHunk {
                        before_start: h.before_start,
                        before_end: h.before_end,
                        after_start: h.after_start,
                        after_end: h.after_end,
                    })
                    .collect()
            }),
        }
    }
}

impl From<git::DiffAnswer> for GitDiffAnswer {
    fn from(answer: git::DiffAnswer) -> Self {
        GitDiffAnswer {
            files: answer.files.into_iter().map(Into::into).collect(),
            truncated: answer.truncated,
        }
    }
}

impl From<&SyntaxRequest> for syntax::Request {
    fn from(request: &SyntaxRequest) -> Self {
        syntax::Request {
            source: match &request.source {
                SyntaxSource::Worktree { root } => syntax::ReadAt::Worktree(root.clone()),
                SyntaxSource::Revision { commit } => syntax::ReadAt::Revision(commit.clone()),
            },
            files: match &request.files {
                SyntaxFiles::Paths(paths) => syntax::FileSet::Paths(paths.clone()),
                SyntaxFiles::Glob(glob) => syntax::FileSet::Glob(glob.clone()),
            },
            language: request.language.clone(),
            query: request.query.clone(),
        }
    }
}

impl From<syntax::SyntaxAnswer> for SyntaxAnswer {
    fn from(answer: syntax::SyntaxAnswer) -> Self {
        SyntaxAnswer {
            files: answer.files.into_iter().map(Into::into).collect(),
            truncated: answer.truncated,
            parsed: answer.parsed,
            cached: answer.cached,
        }
    }
}

impl From<syntax::File> for SyntaxFile {
    fn from(file: syntax::File) -> Self {
        SyntaxFile {
            path: file.path,
            digest: file.digest,
            outcome: match file.outcome {
                syntax::Outcome::Parsed(parsed) => SyntaxOutcome::Parsed {
                    matches: parsed.matches.into_iter().map(Into::into).collect(),
                    errors: parsed.errors,
                },
                syntax::Outcome::NotParsed(reason) => SyntaxOutcome::NotParsed {
                    reason: reason.into(),
                },
            },
        }
    }
}

impl From<syntax::NotParsed> for NotParsed {
    fn from(reason: syntax::NotParsed) -> Self {
        match reason {
            syntax::NotParsed::Deadline => NotParsed::Deadline,
            syntax::NotParsed::TooLarge => NotParsed::TooLarge,
            syntax::NotParsed::Missing => NotParsed::Missing,
            syntax::NotParsed::Unreadable => NotParsed::Unreadable,
            syntax::NotParsed::Denied => NotParsed::Denied,
            syntax::NotParsed::Capped => NotParsed::Capped,
        }
    }
}

impl From<syntax::SyntaxMatch> for SyntaxMatch {
    fn from(found: syntax::SyntaxMatch) -> Self {
        SyntaxMatch {
            role: match found.role {
                syntax::Role::Definition => SyntaxRole::Definition,
                syntax::Role::Reference => SyntaxRole::Reference,
            },
            kind: found.kind,
            span: found.span.into(),
            parent: found.parent,
            captures: found
                .captures
                .into_iter()
                .map(|capture| SyntaxCapture {
                    name: capture.name,
                    text: capture.text,
                    cut: capture.cut,
                    span: capture.span.into(),
                })
                .collect(),
        }
    }
}

impl From<syntax::Span> for SyntaxSpan {
    fn from(span: syntax::Span) -> Self {
        let point = |point: syntax::Point| SyntaxPoint {
            byte: point.byte,
            line: point.line,
            col: point.col,
            byte_col: point.byte_col,
        };
        SyntaxSpan {
            start: point(span.start),
            end: point(span.end),
        }
    }
}

/// A request as the host reads it back: for the round trip, as the host's conversion does.
#[cfg(test)]
pub(super) fn request_back(request: Request) -> HttpRequest {
    HttpRequest {
        method: request.method,
        url: request.url,
        headers: request
            .headers
            .into_iter()
            .map(|h| Header {
                name: h.name,
                value: h.value,
                sensitive: h.sensitive,
            })
            .collect(),
        body: request.body,
        body_cap: request.body_cap,
        timeout_ms: request.timeout_ms,
        max_redirects: request.max_redirects,
        keep_headers: request.keep_headers,
        ..HttpRequest::default()
    }
}

#[cfg(test)]
pub(super) fn run_request_back(request: RunRequest) -> ProcessRequest {
    ProcessRequest {
        argv: request.argv,
        env: request
            .env
            .into_iter()
            .map(|var| specforge_protocol_types::EnvVar {
                name: var.name,
                value: var.value,
            })
            .collect(),
        stdin: request.stdin,
        deadline_ms: request.deadline_ms,
        output_cap: request.output_cap,
        text: request.text,
        inherit: None,
    }
}

#[cfg(test)]
pub(super) fn response_of(response: &HttpResponse) -> Response {
    Response {
        status: response.status,
        headers: response
            .headers
            .iter()
            .map(|(name, value)| http::ResponseHeader {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
        body: response.body.clone(),
        truncated: response.truncated,
        final_url: response.final_url.clone(),
    }
}

#[cfg(test)]
pub(super) fn syntax_request_back(request: syntax::Request) -> SyntaxRequest {
    SyntaxRequest {
        source: match request.source {
            syntax::ReadAt::Worktree(root) => SyntaxSource::Worktree { root },
            syntax::ReadAt::Revision(commit) => SyntaxSource::Revision { commit },
        },
        files: match request.files {
            syntax::FileSet::Paths(paths) => SyntaxFiles::Paths(paths),
            syntax::FileSet::Glob(glob) => SyntaxFiles::Glob(glob),
        },
        language: request.language,
        query: request.query,
    }
}

/// A log query as the host reads it back.
#[cfg(test)]
pub(super) fn log_query_back(query: git::LogQuery) -> GitLogQuery {
    GitLogQuery {
        tips: query.tips,
        hidden: query.hidden,
        paths: query.paths,
        first_parent: query.first_parent,
        max: query.max,
        oldest: query.oldest,
        grep: query.grep,
        since: query.since,
        until: query.until,
        no_merges: query.no_merges,
        pickaxe: query.pickaxe,
    }
}

/// A diff query as the host reads it back.
#[cfg(test)]
pub(super) fn diff_query_back(query: git::DiffQuery) -> GitDiffQuery {
    GitDiffQuery {
        base: query.base,
        head: match query.head {
            git::DiffHead::Commit(sha) => GitDiffHead::Commit(sha),
            git::DiffHead::Index => GitDiffHead::Index,
            git::DiffHead::Worktree => GitDiffHead::Worktree,
        },
        paths: query.paths,
        patch: query.patch,
        hunks: query.hunks,
        renames: query.renames,
    }
}
