use std::cell::RefCell;
use std::rc::Rc;

use specforge_protocol_types::{EnvVar, GitBranch};
use specforge_test_macros::test as specforge_test;

use super::in_process::{self, InProcessHost};
use super::*;

/// One value of each kind in every request and answer the conversions carry, there and back.
#[specforge_test(
    behavior = "call_host_ports_from_the_sdk",
    verify = "every port request and answer survives the SDK's WIT conversion"
)]
fn every_port_request_and_answer_survives_the_wit_conversion() {
    use bindings::specforge::host::{fs, git, process, syntax, types};
    let request = HttpRequest {
        method: "POST".into(),
        url: "https://example.test/a".into(),
        headers: vec![Header::plain("a", "1"), Header::sensitive("b", "2")],
        body: Some(vec![1, 2]),
        body_cap: 3,
        timeout_ms: 4,
        max_redirects: 5,
        keep_headers: vec!["etag".into()],
        ..HttpRequest::default()
    };
    assert_eq!(convert::request_back((&request).into()), request);
    let run = ProcessRequest {
        argv: vec!["tool".into(), "--flag".into()],
        env: vec![EnvVar::set("A", "1"), EnvVar::removed("B")],
        stdin: Some(vec![9]),
        deadline_ms: 6,
        output_cap: 7,
        text: true,
        inherit: None,
    };
    assert_eq!(convert::run_request_back((&run).into()), run);
    for (source, files) in [
        (
            SyntaxSource::Worktree {
                root: "project".into(),
            },
            SyntaxFiles::Paths(vec!["src/lib.rs".into()]),
        ),
        (
            SyntaxSource::Revision {
                commit: "a".repeat(40),
            },
            SyntaxFiles::Glob("src/**/*.rs".into()),
        ),
    ] {
        let asked = SyntaxRequest {
            source,
            files,
            language: "rust".into(),
            query: "(identifier) @reference.name".into(),
        };
        assert_eq!(convert::syntax_request_back((&asked).into()), asked);
    }
    let point = |n: u32| syntax::Point {
        byte: n,
        line: n + 1,
        col: n + 2,
        byte_col: n + 3,
    };
    let our_point = |n: u32| SyntaxPoint {
        byte: n,
        line: n + 1,
        col: n + 2,
        byte_col: n + 3,
    };
    let reasons = [
        (syntax::NotParsed::Deadline, NotParsed::Deadline),
        (syntax::NotParsed::TooLarge, NotParsed::TooLarge),
        (syntax::NotParsed::Missing, NotParsed::Missing),
        (syntax::NotParsed::Unreadable, NotParsed::Unreadable),
        (syntax::NotParsed::Denied, NotParsed::Denied),
        (syntax::NotParsed::Capped, NotParsed::Capped),
    ];
    for ((wit_role, role), (wit_reason, reason)) in [
        (syntax::Role::Definition, SyntaxRole::Definition),
        (syntax::Role::Reference, SyntaxRole::Reference),
    ]
    .into_iter()
    .cycle()
    .zip(reasons)
    {
        let parsed = syntax::File {
            path: "a.rs".into(),
            digest: Some("sha256:00".into()),
            outcome: syntax::Outcome::Parsed(syntax::Parsed {
                matches: vec![syntax::SyntaxMatch {
                    role: wit_role,
                    kind: "function".into(),
                    span: syntax::Span {
                        start: point(1),
                        end: point(5),
                    },
                    parent: Some(0),
                    captures: vec![syntax::Capture {
                        name: "name".into(),
                        text: "f".into(),
                        cut: true,
                        span: syntax::Span {
                            start: point(2),
                            end: point(3),
                        },
                    }],
                }],
                errors: 2,
            }),
        };
        let not_parsed = syntax::File {
            path: "b.rs".into(),
            digest: None,
            outcome: syntax::Outcome::NotParsed(wit_reason),
        };
        assert_eq!(
            SyntaxAnswer::from(syntax::SyntaxAnswer {
                files: vec![parsed, not_parsed],
                truncated: true,
                parsed: 1,
                cached: 3,
            }),
            SyntaxAnswer {
                files: vec![
                    SyntaxFile {
                        path: "a.rs".into(),
                        digest: Some("sha256:00".into()),
                        outcome: SyntaxOutcome::Parsed {
                            matches: vec![SyntaxMatch {
                                role,
                                kind: "function".into(),
                                span: SyntaxSpan {
                                    start: our_point(1),
                                    end: our_point(5),
                                },
                                parent: Some(0),
                                captures: vec![SyntaxCapture {
                                    name: "name".into(),
                                    text: "f".into(),
                                    cut: true,
                                    span: SyntaxSpan {
                                        start: our_point(2),
                                        end: our_point(3),
                                    },
                                }],
                            }],
                            errors: 2,
                        },
                    },
                    SyntaxFile {
                        path: "b.rs".into(),
                        digest: None,
                        outcome: SyntaxOutcome::NotParsed { reason },
                    },
                ],
                truncated: true,
                parsed: 1,
                cached: 3,
            }
        );
    }
    let response = HttpResponse {
        status: 404,
        headers: vec![("etag".into(), "x".into())],
        body: vec![8],
        truncated: true,
        final_url: "https://example.test/b".into(),
    };
    assert_eq!(
        HttpResponse::from(convert::response_of(&response)),
        response
    );
    for (wit, ours) in [
        (
            types::HostError::Denied(types::Denial::CallKind("k".into())),
            HostError::Denied(Denial::CallKind("k".into())),
        ),
        (
            types::HostError::Denied(types::Denial::Scope("s".into())),
            HostError::Denied(Denial::Scope("s".into())),
        ),
        (
            types::HostError::Failed(types::Failure {
                class: "deadline".into(),
                message: "m".into(),
            }),
            HostError::Failed(Failure {
                class: "deadline".into(),
                message: "m".into(),
            }),
        ),
    ] {
        assert_eq!(HostError::from(wit), ours);
    }
    for (wit, ours) in [
        (fs::EntryKind::File, FsEntryKind::File),
        (fs::EntryKind::Dir, FsEntryKind::Dir),
        (fs::EntryKind::Other, FsEntryKind::Other),
    ] {
        assert_eq!(
            FsEntry::from(fs::Entry {
                kind: wit,
                size: 1,
                modified_ms: Some(2),
            }),
            FsEntry {
                kind: ours,
                size: 1,
                modified_ms: Some(2),
            }
        );
    }
    assert_eq!(
        FsContent::from(fs::Content {
            bytes: vec![1],
            truncated: true,
        }),
        FsContent {
            bytes: vec![1],
            truncated: true,
        }
    );
    assert_eq!(
        FsListing::from(fs::Listing {
            entries: vec!["a".into()],
            truncated: true,
        }),
        FsListing {
            entries: vec!["a".into()],
            truncated: true,
        }
    );
    assert_eq!(
        ProcessOutput::from(process::RunOutput {
            exit_code: Some(3),
            stdout: vec![1],
            stderr: vec![2],
            truncated: true,
        }),
        ProcessOutput {
            exit_code: Some(3),
            stdout: vec![1],
            stderr: vec![2],
            truncated: true,
        }
    );
    assert_eq!(
        GitRepo::from(git::Repo {
            toplevel: "/r".into(),
            shallow: true,
        }),
        GitRepo {
            toplevel: "/r".into(),
            shallow: true,
        }
    );
    assert_eq!(
        GitTreeEntry::from(git::TreeEntry {
            path: "a".into(),
            file: true,
        }),
        GitTreeEntry {
            path: "a".into(),
            file: true,
        }
    );
    assert_eq!(
        FileSize::from(git::FileSize {
            path: "a".into(),
            size: Some(1),
        }),
        FileSize {
            path: "a".into(),
            size: Some(1),
        }
    );
    assert_eq!(
        GitBlobs::from(git::BlobsAnswer {
            blobs: vec![git::Blob {
                path: "a".into(),
                bytes: Some(vec![1]),
            }],
            truncated: true,
        }),
        GitBlobs {
            blobs: vec![GitBlob {
                path: "a".into(),
                bytes: Some(vec![1]),
            }],
            truncated: true,
        }
    );
    let entries = [
        (git::EntryKind::Changed, GitEntryKind::Changed),
        (git::EntryKind::Renamed, GitEntryKind::Renamed),
        (git::EntryKind::Unmerged, GitEntryKind::Unmerged),
        (git::EntryKind::Untracked, GitEntryKind::Untracked),
        (git::EntryKind::Ignored, GitEntryKind::Ignored),
    ];
    let changes = [
        (git::Change::Modified, GitChange::Modified),
        (git::Change::TypeChanged, GitChange::TypeChanged),
        (git::Change::Added, GitChange::Added),
        (git::Change::Deleted, GitChange::Deleted),
        (git::Change::Renamed, GitChange::Renamed),
        (git::Change::Copied, GitChange::Copied),
        (git::Change::Unmerged, GitChange::Unmerged),
    ];
    for ((wit_kind, kind), (wit_change, change)) in entries.into_iter().zip(changes) {
        assert_eq!(
            GitStatus::from(git::StatusAnswer {
                branch: git::BranchStatus {
                    oid: Some("o".into()),
                    head: Some("h".into()),
                    upstream: Some("u".into()),
                    ahead: 1,
                    behind: 2,
                },
                entries: vec![git::Entry {
                    kind: wit_kind,
                    path: "p".into(),
                    orig_path: Some("q".into()),
                    index: Some(wit_change),
                    worktree: None,
                }],
            }),
            GitStatus {
                branch: GitBranch {
                    oid: Some("o".into()),
                    head: Some("h".into()),
                    upstream: Some("u".into()),
                    ahead: 1,
                    behind: 2,
                },
                entries: vec![GitEntry {
                    kind,
                    path: "p".into(),
                    orig_path: Some("q".into()),
                    index: Some(change),
                    worktree: None,
                }],
            }
        );
    }
}

/// A guest's log and diff queries, and their answers, there and back.
#[test]
fn a_guest_log_and_diff_survive_the_wit_conversion() {
    use bindings::specforge::host::git as wit;
    let log = GitLogQuery {
        tips: vec!["a1".into()],
        hidden: vec!["b2".into()],
        paths: vec!["src".into()],
        first_parent: true,
        max: Some(3),
        oldest: true,
        grep: Some("feat".into()),
        since: Some(-1),
        until: Some(2),
        no_merges: true,
        pickaxe: Some("x".into()),
    };
    assert_eq!(convert::log_query_back((&log).into()), log);
    for head in [
        GitDiffHead::Commit("c3".into()),
        GitDiffHead::Index,
        GitDiffHead::Worktree,
    ] {
        let diff = GitDiffQuery {
            base: Some("d4".into()),
            head,
            paths: vec!["a".into()],
            patch: true,
            hunks: true,
            renames: false,
        };
        assert_eq!(convert::diff_query_back((&diff).into()), diff);
    }
    let signed = |seconds| wit::Signature {
        name: "n".into(),
        seconds,
    };
    let answer = GitLogAnswer::from(wit::LogAnswer {
        count: Some(1),
        commits: vec![wit::LogCommit {
            sha: "a1".into(),
            parents: vec!["b2".into()],
            author: signed(1),
            committer: signed(2),
            subject: "s".into(),
            body: "b".into(),
            trailers: vec![wit::Trailer {
                token: "Gsd-Task".into(),
                values: vec!["1".into(), "2".into()],
            }],
        }],
        truncated: true,
    });
    let commit = &answer.commits[0];
    assert_eq!((answer.count, answer.truncated), (Some(1), true));
    assert_eq!((commit.author.seconds, commit.committer.seconds), (1, 2));
    assert_eq!(commit.parents, ["b2"]);
    assert_eq!(
        commit.trailers,
        [GitTrailer {
            token: "Gsd-Task".into(),
            values: vec!["1".into(), "2".into()],
        }]
    );
    let diff = GitDiffAnswer::from(wit::DiffAnswer {
        files: vec![wit::FileChange {
            path: "b".into(),
            old_path: Some("a".into()),
            status: wit::Change::Renamed,
            added: 1,
            removed: 2,
            binary: false,
            truncated: true,
            withheld: true,
            patch: Some("p".into()),
            hunks: Some(vec![wit::Hunk {
                before_start: 1,
                before_end: 2,
                after_start: 3,
                after_end: 4,
            }]),
        }],
        truncated: true,
    });
    assert_eq!(
        diff,
        GitDiffAnswer {
            files: vec![GitFileChange {
                path: "b".into(),
                old_path: Some("a".into()),
                status: GitChange::Renamed,
                added: 1,
                removed: 2,
                binary: false,
                truncated: true,
                withheld: true,
                patch: Some("p".into()),
                hunks: Some(vec![GitHunk {
                    before_start: 1,
                    before_end: 2,
                    after_start: 3,
                    after_end: 4,
                }]),
            }],
            truncated: true,
        }
    );
}

#[test]
fn a_request_has_no_place_for_what_only_the_host_sets() {
    let asked = HttpRequest {
        method: "GET".into(),
        url: "https://example.test/".into(),
        allow_private: specforge_protocol_types::Private::Any,
        secret: Some(specforge_protocol_types::AttachedSecret {
            header: "authorization".into(),
            env: "A_TOKEN".into(),
            scheme: None,
            hosts: vec!["example.test".into()],
        }),
        ..HttpRequest::default()
    };
    let back = convert::request_back((&asked).into());
    assert!(back.allow_private.is_never());
    assert_eq!(back.secret, None);
}

#[specforge_test(
    behavior = "call_host_ports_from_the_sdk",
    verify = "a port called outside any host is answered denied(call-kind)"
)]
fn a_port_called_outside_any_host_is_denied_its_call_kind() {
    assert!(matches!(
        fs::stat("project", "a"),
        Err(HostError::Denied(Denial::CallKind(_)))
    ));
    assert!(matches!(
        git::locate(),
        Err(HostError::Denied(Denial::CallKind(_)))
    ));
}

#[derive(Default)]
struct Fake {
    asked: RefCell<Vec<String>>,
}

impl InProcessHost for Fake {
    fn http_send(&self, request: &HttpRequest) -> Result<HttpResponse, HostError> {
        self.asked.borrow_mut().push(request.url.clone());
        Ok(HttpResponse {
            status: 200,
            ..HttpResponse::default()
        })
    }
}

#[test]
fn a_served_host_answers_the_guest() {
    let fake = Rc::new(Fake::default());
    let request = HttpRequest {
        url: "https://example.test/".into(),
        ..HttpRequest::default()
    };
    let answer = in_process::serve(Rc::clone(&fake) as Rc<dyn InProcessHost>, || {
        http::send(&request)
    });
    assert_eq!(answer.unwrap().status, 200);
    assert_eq!(*fake.asked.borrow(), ["https://example.test/"]);
    // A port the fake does not give, and no host at all once the call is over.
    let unserved = in_process::serve(Rc::clone(&fake) as Rc<dyn InProcessHost>, git::branch);
    assert!(matches!(
        unserved,
        Err(HostError::Denied(Denial::CallKind(_)))
    ));
    assert!(http::send(&request).is_err());
    assert_eq!(fake.asked.borrow().len(), 1);
}

/// A host that answers a log and a diff, and records what it was asked.
#[derive(Default)]
struct History {
    asked: RefCell<Vec<String>>,
}

impl InProcessHost for History {
    fn git_log(&self, query: &GitLogQuery) -> Result<GitLogAnswer, HostError> {
        self.asked
            .borrow_mut()
            .push(format!("log {:?}", query.tips));
        Ok(GitLogAnswer {
            count: Some(0),
            ..GitLogAnswer::default()
        })
    }
    fn git_diff(&self, query: &GitDiffQuery) -> Result<GitDiffAnswer, HostError> {
        self.asked
            .borrow_mut()
            .push(format!("diff {:?}", query.base));
        Ok(GitDiffAnswer::default())
    }
}

#[test]
fn a_served_host_answers_a_guests_log_and_diff() {
    let host = Rc::new(History::default());
    let (log, diff) = in_process::serve(Rc::clone(&host) as Rc<dyn InProcessHost>, || {
        let log = git::log(&GitLogQuery {
            tips: vec!["a1".into()],
            ..GitLogQuery::default()
        });
        let diff = git::diff(&GitDiffQuery {
            base: None,
            head: GitDiffHead::Worktree,
            paths: Vec::new(),
            patch: false,
            hunks: true,
            renames: false,
        });
        (log, diff)
    });
    assert_eq!(log.unwrap().count, Some(0));
    assert_eq!(diff.unwrap(), GitDiffAnswer::default());
    assert_eq!(*host.asked.borrow(), ["log [\"a1\"]", "diff None"]);
    // no host: a pure call's answer
    assert!(matches!(
        git::log(&GitLogQuery::default()),
        Err(HostError::Denied(Denial::CallKind(_)))
    ));
}
