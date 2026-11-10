//! The `specforge:host` imports (ADR 0335 D4): the WIT's types at the guest's edge, and the call's
//! [`PortSession`](specforge_wasm::PortSession) deciding each. A WIT request has no field for what only the host sets.

use specforge_protocol_types::{
    Denial, EnvVar, FsEntryKind, GitBranch, GitChange, GitDiffAnswer, GitDiffHead, GitDiffQuery,
    GitEntry, GitEntryKind, GitLogAnswer, GitLogQuery, GitSignature, GitStatus, Header,
    HostError as WireError, HttpRequest, NotParsed, ProcessRequest, SyntaxAnswer, SyntaxCapture,
    SyntaxFile, SyntaxFiles, SyntaxMatch, SyntaxOutcome, SyntaxPoint, SyntaxRequest, SyntaxRole,
    SyntaxSource, SyntaxSpan,
};
use wasmtime::component::{HasSelf, Linker};

use crate::HostState;

wasmtime::component::bindgen!({
    path: "../extension-sdk/wit/host.wit",
    world: "ports",
});

use specforge::host::types::{self, HostError};
use specforge::host::{fs, git, http, process, syntax};

/// Link the five ports (and their shared types): every instance has them, whatever its grant.
pub(crate) fn link(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    Ports::add_to_linker::<_, HasSelf<_>>(linker, |state| state)
}

fn error(error: WireError) -> HostError {
    match error {
        WireError::Denied(Denial::CallKind(why)) => HostError::Denied(types::Denial::CallKind(why)),
        WireError::Denied(Denial::Scope(why)) => HostError::Denied(types::Denial::Scope(why)),
        WireError::Failed(failure) => HostError::Failed(types::Failure {
            class: failure.class,
            message: failure.message,
        }),
    }
}

impl types::Host for HostState {}

impl http::Host for HostState {
    fn send(&mut self, req: http::Request) -> Result<http::Response, HostError> {
        let request = HttpRequest {
            method: req.method,
            url: req.url,
            headers: req
                .headers
                .into_iter()
                .map(|h| Header {
                    name: h.name,
                    value: h.value,
                    sensitive: h.sensitive,
                })
                .collect(),
            body: req.body,
            body_cap: req.body_cap,
            timeout_ms: req.timeout_ms,
            max_redirects: req.max_redirects,
            keep_headers: req.keep_headers,
            ..HttpRequest::default()
        };
        let response = self
            .ports
            .timed(|session| session.http(request))
            .map_err(error)?;
        Ok(http::Response {
            status: response.status,
            headers: response
                .headers
                .into_iter()
                .map(|(name, value)| http::ResponseHeader { name, value })
                .collect(),
            body: response.body,
            truncated: response.truncated,
            final_url: response.final_url,
        })
    }
}

impl fs::Host for HostState {
    fn stat(&mut self, root: String, path: String) -> Result<Option<fs::Entry>, HostError> {
        let entry = self
            .ports
            .timed(|session| session.fs_stat(&root, &path))
            .map_err(error)?;
        Ok(entry.map(|entry| fs::Entry {
            kind: match entry.kind {
                FsEntryKind::File => fs::EntryKind::File,
                FsEntryKind::Dir => fs::EntryKind::Dir,
                FsEntryKind::Other => fs::EntryKind::Other,
            },
            size: entry.size,
            modified_ms: entry.modified_ms,
        }))
    }

    fn read(
        &mut self,
        root: String,
        path: String,
        max_bytes: u64,
    ) -> Result<fs::Content, HostError> {
        let content = self
            .ports
            .timed(|session| session.fs_read(&root, &path, max_bytes))
            .map_err(error)?;
        Ok(fs::Content {
            bytes: content.bytes,
            truncated: content.truncated,
        })
    }

    fn walk(
        &mut self,
        root: String,
        path: String,
        glob: Option<String>,
        max_entries: u32,
    ) -> Result<fs::Listing, HostError> {
        let listing = self
            .ports
            .timed(|session| session.fs_walk(&root, &path, glob.as_deref(), max_entries))
            .map_err(error)?;
        Ok(fs::Listing {
            entries: listing.entries,
            truncated: listing.truncated,
        })
    }
}

impl process::Host for HostState {
    fn run(&mut self, req: process::RunRequest) -> Result<process::RunOutput, HostError> {
        let request = ProcessRequest {
            argv: req.argv,
            env: req
                .env
                .into_iter()
                .map(|var| EnvVar {
                    name: var.name,
                    value: var.value,
                })
                .collect(),
            stdin: req.stdin,
            deadline_ms: req.deadline_ms,
            output_cap: req.output_cap,
            text: req.text,
            inherit: None,
        };
        let output = self
            .ports
            .timed(|session| session.process_run(request))
            .map_err(error)?;
        Ok(process::RunOutput {
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.truncated,
        })
    }
}

fn change(change: GitChange) -> git::Change {
    match change {
        GitChange::Modified => git::Change::Modified,
        GitChange::TypeChanged => git::Change::TypeChanged,
        GitChange::Added => git::Change::Added,
        GitChange::Deleted => git::Change::Deleted,
        GitChange::Renamed => git::Change::Renamed,
        GitChange::Copied => git::Change::Copied,
        GitChange::Unmerged => git::Change::Unmerged,
    }
}

fn status_entry(entry: GitEntry) -> git::Entry {
    git::Entry {
        kind: match entry.kind {
            GitEntryKind::Changed => git::EntryKind::Changed,
            GitEntryKind::Renamed => git::EntryKind::Renamed,
            GitEntryKind::Unmerged => git::EntryKind::Unmerged,
            GitEntryKind::Untracked => git::EntryKind::Untracked,
            GitEntryKind::Ignored => git::EntryKind::Ignored,
        },
        path: entry.path,
        orig_path: entry.orig_path,
        index: entry.index.map(change),
        worktree: entry.worktree.map(change),
    }
}

fn branch(branch: GitBranch) -> git::BranchStatus {
    git::BranchStatus {
        oid: branch.oid,
        head: branch.head,
        upstream: branch.upstream,
        ahead: branch.ahead,
        behind: branch.behind,
    }
}

fn status(status: GitStatus) -> git::StatusAnswer {
    git::StatusAnswer {
        branch: branch(status.branch),
        entries: status.entries.into_iter().map(status_entry).collect(),
    }
}

fn signature(signature: GitSignature) -> git::Signature {
    git::Signature {
        name: signature.name,
        email: signature.email,
        time: signature.time,
        seconds: signature.seconds,
    }
}

fn log_query(query: git::LogQuery) -> GitLogQuery {
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

fn log_answer(answer: GitLogAnswer) -> git::LogAnswer {
    git::LogAnswer {
        count: answer.count,
        commits: answer
            .commits
            .into_iter()
            .map(|commit| git::LogCommit {
                sha: commit.sha,
                parents: commit.parents,
                author: signature(commit.author),
                committer: signature(commit.committer),
                subject: commit.subject,
                body: commit.body,
                trailers: commit
                    .trailers
                    .into_iter()
                    .map(|t| git::Trailer {
                        token: t.token,
                        values: t.values,
                    })
                    .collect(),
            })
            .collect(),
        truncated: answer.truncated,
    }
}

fn diff_query(query: git::DiffQuery) -> GitDiffQuery {
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

fn diff_answer(answer: GitDiffAnswer) -> git::DiffAnswer {
    git::DiffAnswer {
        files: answer
            .files
            .into_iter()
            .map(|file| git::FileChange {
                path: file.path,
                old_path: file.old_path,
                status: change(file.status),
                added: file.added,
                removed: file.removed,
                binary: file.binary,
                truncated: file.truncated,
                withheld: file.withheld,
                patch: file.patch,
                hunks: file.hunks.map(|hunks| {
                    hunks
                        .into_iter()
                        .map(|h| git::Hunk {
                            before_start: h.before_start,
                            before_end: h.before_end,
                            after_start: h.after_start,
                            after_end: h.after_end,
                        })
                        .collect()
                }),
            })
            .collect(),
        truncated: answer.truncated,
    }
}

impl git::Host for HostState {
    fn locate(&mut self) -> Result<Option<git::Repo>, HostError> {
        let repo = self
            .ports
            .timed(|session| session.git_locate())
            .map_err(error)?;
        Ok(repo.map(|repo| git::Repo {
            toplevel: repo.toplevel,
            shallow: repo.shallow,
        }))
    }

    fn commit(&mut self, revision: String) -> Result<Option<String>, HostError> {
        self.ports
            .timed(|session| session.git_commit(&revision))
            .map_err(error)
    }

    fn merge_base(&mut self, a: String, b: String) -> Result<Option<String>, HostError> {
        self.ports
            .timed(|session| session.git_merge_base(&a, &b))
            .map_err(error)
    }

    fn tree(&mut self, commit: String, dir: String) -> Result<Vec<git::TreeEntry>, HostError> {
        let entries = self
            .ports
            .timed(|session| session.git_tree(&commit, &dir))
            .map_err(error)?;
        Ok(entries
            .into_iter()
            .map(|entry| git::TreeEntry {
                path: entry.path,
                file: entry.file,
            })
            .collect())
    }

    fn sizes(
        &mut self,
        commit: String,
        paths: Vec<String>,
    ) -> Result<Vec<git::FileSize>, HostError> {
        let sized = self
            .ports
            .timed(|session| session.git_sizes(&commit, &paths))
            .map_err(error)?;
        Ok(sized
            .into_iter()
            .map(|sized| git::FileSize {
                path: sized.path,
                size: sized.size,
            })
            .collect())
    }

    fn blobs(&mut self, commit: String, paths: Vec<String>) -> Result<git::BlobsAnswer, HostError> {
        let answer = self
            .ports
            .timed(|session| session.git_blobs(&commit, &paths))
            .map_err(error)?;
        Ok(git::BlobsAnswer {
            blobs: answer
                .blobs
                .into_iter()
                .map(|blob| git::Blob {
                    path: blob.path,
                    bytes: blob.bytes,
                })
                .collect(),
            truncated: answer.truncated,
        })
    }

    fn branch(&mut self) -> Result<String, HostError> {
        self.ports
            .timed(|session| session.git_branch())
            .map_err(error)
    }

    fn status(&mut self) -> Result<git::StatusAnswer, HostError> {
        self.ports
            .timed(|session| session.git_status())
            .map(status)
            .map_err(error)
    }

    fn log(&mut self, query: git::LogQuery) -> Result<git::LogAnswer, HostError> {
        let query = log_query(query);
        self.ports
            .timed(|session| session.git_log(&query))
            .map(log_answer)
            .map_err(error)
    }

    fn diff(&mut self, query: git::DiffQuery) -> Result<git::DiffAnswer, HostError> {
        let query = diff_query(query);
        self.ports
            .timed(|session| session.git_diff(&query))
            .map(diff_answer)
            .map_err(error)
    }
}

impl syntax::Host for HostState {
    fn parse(&mut self, req: syntax::Request) -> Result<syntax::SyntaxAnswer, HostError> {
        let request = SyntaxRequest {
            source: match req.source {
                syntax::ReadAt::Worktree(root) => SyntaxSource::Worktree { root },
                syntax::ReadAt::Revision(commit) => SyntaxSource::Revision { commit },
            },
            files: match req.files {
                syntax::FileSet::Paths(paths) => SyntaxFiles::Paths(paths),
                syntax::FileSet::Glob(glob) => SyntaxFiles::Glob(glob),
            },
            language: req.language,
            query: req.query,
        };
        self.ports
            .timed(|session| session.syntax_parse(request))
            .map(syntax_answer)
            .map_err(error)
    }
}

fn syntax_answer(answer: SyntaxAnswer) -> syntax::SyntaxAnswer {
    syntax::SyntaxAnswer {
        files: answer.files.into_iter().map(syntax_file).collect(),
        truncated: answer.truncated,
        parsed: answer.parsed,
        cached: answer.cached,
    }
}

fn syntax_file(file: SyntaxFile) -> syntax::File {
    syntax::File {
        path: file.path,
        digest: file.digest,
        outcome: match file.outcome {
            SyntaxOutcome::Parsed { matches, errors } => syntax::Outcome::Parsed(syntax::Parsed {
                matches: matches.into_iter().map(syntax_match).collect(),
                errors,
            }),
            SyntaxOutcome::NotParsed { reason } => syntax::Outcome::NotParsed(match reason {
                NotParsed::Deadline => syntax::NotParsed::Deadline,
                NotParsed::TooLarge => syntax::NotParsed::TooLarge,
                NotParsed::Missing => syntax::NotParsed::Missing,
                NotParsed::Unreadable => syntax::NotParsed::Unreadable,
                NotParsed::Denied => syntax::NotParsed::Denied,
                NotParsed::Capped => syntax::NotParsed::Capped,
            }),
        },
    }
}

fn syntax_match(found: SyntaxMatch) -> syntax::SyntaxMatch {
    syntax::SyntaxMatch {
        role: match found.role {
            SyntaxRole::Definition => syntax::Role::Definition,
            SyntaxRole::Reference => syntax::Role::Reference,
        },
        kind: found.kind,
        span: span(found.span),
        parent: found.parent,
        captures: found
            .captures
            .into_iter()
            .map(|capture: SyntaxCapture| syntax::Capture {
                name: capture.name,
                text: capture.text,
                cut: capture.cut,
                span: span(capture.span),
            })
            .collect(),
    }
}

fn span(span: SyntaxSpan) -> syntax::Span {
    let point = |point: SyntaxPoint| syntax::Point {
        byte: point.byte,
        line: point.line,
        col: point.col,
        byte_col: point.byte_col,
    };
    syntax::Span {
        start: point(span.start),
        end: point(span.end),
    }
}
