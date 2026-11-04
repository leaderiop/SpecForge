//! The protocol's types and what wit-bindgen generates, at the guest's edge (ADR 0335 D4). A request goes out as WIT
//! and an answer comes back; the reverse directions exist for the round-trip test that holds the two together.

use super::wit::git::{Blob, BlobsAnswer, Repo, Sized, StatusAnswer, TreeEntry};
use super::wit::http::{self, Request, Response};
use super::wit::process::{self, RunOutput, RunRequest};
use super::wit::{fs, git, types};
use super::{
    Denial, Failure, FsContent, FsEntry, FsEntryKind, FsListing, GitBlob, GitBlobs, GitChange,
    GitEntry, GitEntryKind, GitRepo, GitSized, GitStatus, GitTreeEntry, HostError, HttpRequest,
    HttpResponse, ProcessOutput, ProcessRequest,
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

impl From<Sized> for GitSized {
    fn from(sized: Sized) -> Self {
        GitSized {
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
