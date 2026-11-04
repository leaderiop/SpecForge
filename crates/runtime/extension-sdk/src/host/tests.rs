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
    use bindings::specforge::host::{fs, git, process, types};
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
        GitSized::from(git::FileSize {
            path: "a".into(),
            size: Some(1),
        }),
        GitSized {
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

#[test]
fn a_request_has_no_place_for_what_only_the_host_sets() {
    let asked = HttpRequest {
        method: "GET".into(),
        url: "https://example.test/".into(),
        allow_private: true,
        secret: Some(specforge_protocol_types::AttachedSecret {
            header: "authorization".into(),
            env: "A_TOKEN".into(),
            scheme: None,
            hosts: vec!["example.test".into()],
        }),
        ..HttpRequest::default()
    };
    let back = convert::request_back((&asked).into());
    assert!(!back.allow_private);
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
