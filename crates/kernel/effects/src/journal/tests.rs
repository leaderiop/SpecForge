use super::*;
use crate::{
    AttachedSecret, EnvVar, FsOp, FsRequest, Header, HttpRequest, ProcessRequest, Unreachable,
};

fn http(url: &str) -> EffectRequest {
    EffectRequest::Http(HttpRequest {
        method: "GET".into(),
        url: url.into(),
        body_cap: 1024,
        ..HttpRequest::default()
    })
}

fn answer(class: OutcomeClass) -> Answer {
    Answer {
        outcome: Outcome {
            class,
            digest: String::new(),
        },
        payload: None,
    }
}

fn at(ms: u64) -> UnixMillis {
    UnixMillis(ms)
}

#[test]
fn a_record_is_written_atomically_and_read_back_by_its_request_digest() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let (path, record) = journal
        .keep(&request, &answer(OutcomeClass::Ok), at(1_000))
        .unwrap()
        .expect("journaled");
    assert!(path.is_file());
    assert!(!path.with_extension("json.tmp").exists(), "no temp left");
    assert_eq!(record.request_digest, RecordDigest::of(&request));
    assert_eq!(
        journal.lookup(&record.request_digest, at(2_000)),
        Lookup::Fresh(Kept {
            record: record.clone(),
            payload: None
        })
    );
    assert_eq!(record.ttl, Some(HTTP_SUCCESS_TTL));
}

#[test]
fn only_an_http_failure_the_table_names_is_journaled_an_hour() {
    let request = http("https://example.org/");
    let ttl = |class| record_of(&request, &answer(class), at(0)).map(|record| record.ttl);
    assert_eq!(ttl(OutcomeClass::Status(404)), Some(Some(HTTP_FAILURE_TTL)));
    assert_eq!(ttl(OutcomeClass::Status(503)), Some(Some(HTTP_FAILURE_TTL)));
    assert_eq!(
        ttl(OutcomeClass::Unreachable(Unreachable::Dns)),
        Some(Some(HTTP_FAILURE_TTL))
    );
    for class in [
        OutcomeClass::Denied,
        OutcomeClass::Deadline,
        OutcomeClass::Capped,
        OutcomeClass::Status(304),
        OutcomeClass::Redirect(crate::Redirected::Limit),
    ] {
        assert_eq!(ttl(class), None, "{class} is not journaled");
    }
    let run = EffectRequest::Process(ProcessRequest::default());
    assert_eq!(
        record_of(&run, &answer(OutcomeClass::Exit(1)), at(0)).map(|r| r.ttl),
        Some(None),
        "a process run is kept for audit"
    );
    assert_eq!(record_of(&run, &answer(OutcomeClass::Denied), at(0)), None);
}

#[test]
fn a_record_past_its_ttl_is_expired_not_fresh() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let (_, record) = journal
        .keep(&request, &answer(OutcomeClass::Ok), at(0))
        .unwrap()
        .unwrap();
    let end = at(HTTP_SUCCESS_TTL.as_millis() as u64);
    assert_eq!(expires(&record), Some(end));
    assert!(matches!(
        journal.lookup(&record.request_digest, at(end.0 - 1)),
        Lookup::Fresh(_)
    ));
    assert!(matches!(
        journal.lookup(&record.request_digest, end),
        Lookup::Expired(_)
    ));
    // A process run is journaled for audit but never answers fresh again.
    let run = EffectRequest::Process(ProcessRequest::default());
    let (_, record) = journal
        .keep(&run, &answer(OutcomeClass::Ok), at(0))
        .unwrap()
        .expect("journaled");
    assert!(matches!(
        journal.lookup(&record.request_digest, at(0)),
        Lookup::Expired(_)
    ));
}

#[test]
fn an_undecodable_record_is_missing_and_the_next_record_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let digest = RecordDigest::of(&request);
    let path = journal.path(&digest).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{ not json").unwrap();
    assert_eq!(journal.lookup(&digest, at(0)), Lookup::Missing);
    journal
        .keep(&request, &answer(OutcomeClass::Ok), at(0))
        .unwrap();
    assert!(matches!(journal.lookup(&digest, at(0)), Lookup::Fresh(_)));
}

#[test]
fn fs_and_git_requests_are_never_journaled() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let read = EffectRequest::Fs(FsRequest {
        root: "project".into(),
        path: "a.txt".into(),
        op: FsOp::Read { max_bytes: 1 },
    });
    assert_eq!(retention(&read, OutcomeClass::Ok), Retention::Never);
    assert_eq!(
        journal
            .keep(&read, &answer(OutcomeClass::Ok), at(0))
            .unwrap(),
        None
    );
    assert!(!dir.path().join(JOURNAL_DIR).exists());
    // A detached journal writes nothing either, and every lookup misses.
    let request = http("https://example.org/");
    let detached = EffectJournal::detached();
    assert_eq!(
        detached
            .keep(&request, &answer(OutcomeClass::Ok), at(0))
            .unwrap(),
        None
    );
    assert_eq!(
        detached.lookup(&RecordDigest::of(&request), at(0)),
        Lookup::Missing
    );
}

#[test]
fn the_journal_lives_under_specforge_effects() {
    let journal = EffectJournal::at(Path::new("/p"));
    let hex = "ab".repeat(32);
    let digest = RecordDigest::parse(&format!("sha256:{hex}")).unwrap();
    assert_eq!(
        journal.path(&digest).unwrap(),
        Path::new("/p/.specforge/effects/journal/ab").join(format!("{hex}.json"))
    );
}

#[test]
fn a_record_digest_is_sha256_and_hex_only() {
    let hex = "0f".repeat(32);
    assert!(RecordDigest::parse(&format!("sha256:{hex}")).is_some());
    for text in [
        "../x",
        "sha256:../x",
        hex.as_str(),
        &format!("sha256:{}", "0F".repeat(32)),
        &format!("sha256:{}/..", "0f".repeat(31)),
        &format!("sha256:{hex}0"),
    ] {
        assert_eq!(RecordDigest::parse(text), None, "{text}");
    }
    // A record file that names another digest does not decode.
    assert!(serde_json::from_str::<RecordDigest>("\"../x\"").is_err());
}

#[test]
fn a_record_names_neither_the_secret_a_grant_attached_nor_its_header() {
    let dir = tempfile::tempdir().unwrap();
    let request = EffectRequest::Http(HttpRequest {
        method: "GET".into(),
        url: "https://example.org/api".into(),
        secret: Some(AttachedSecret {
            header: "X-Attached-Key".into(),
            env: "ATTACHED_ENV".into(),
            scheme: Some("Bearer".into()),
            hosts: vec!["example.org".into()],
        }),
        ..HttpRequest::default()
    });
    let (path, _) = EffectJournal::at(dir.path())
        .keep(&request, &answer(OutcomeClass::Ok), at(0))
        .unwrap()
        .unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    for name in ["ATTACHED_ENV", "X-Attached-Key"] {
        assert!(!written.contains(name), "{name} in {written}");
    }
}

#[test]
fn a_record_holds_no_secret() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = EffectRequest::Http(HttpRequest {
        method: "POST".into(),
        url: "https://example.org/api".into(),
        headers: vec![
            Header::sensitive("Authorization", "Bearer s3cr3t-t0ken"),
            Header::plain("Accept", "application/json"),
        ],
        body: Some(b"password=hunter2".to_vec()),
        body_cap: 1024,
        ..HttpRequest::default()
    });
    let (path, _) = journal
        .keep(&request, &answer(OutcomeClass::Ok), at(0))
        .unwrap()
        .unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    for secret in ["s3cr3t-t0ken", "hunter2"] {
        assert!(!written.contains(secret), "{secret} in {written}");
    }
    assert!(written.contains("Authorization") && written.contains("application/json"));
    assert!(written.contains(&Blob::of(b"password=hunter2").sha256));
    let run = EffectRequest::Process(ProcessRequest {
        argv: vec!["deploy".into()],
        env: vec![EnvVar::set("API_TOKEN", "env-s3cr3t")],
        stdin: Some(b"stdin-s3cr3t".to_vec()),
        ..ProcessRequest::default()
    });
    let (path, _) = journal
        .keep(&run, &answer(OutcomeClass::Ok), at(0))
        .unwrap()
        .unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(!written.contains("env-s3cr3t") && !written.contains("stdin-s3cr3t"));
    assert!(written.contains("API_TOKEN"));
}

#[cfg(unix)]
#[test]
fn journal_files_and_directories_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let ok = Answer {
        payload: Some(b"<html>".to_vec()),
        ..answer(OutcomeClass::Ok)
    };
    let (path, _) = journal.keep(&request, &ok, at(0)).unwrap().unwrap();
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(&path.with_extension("payload")), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
    assert_eq!(mode(&dir.path().join(JOURNAL_DIR)), 0o700);
}

#[test]
fn a_payload_is_kept_beside_its_record_and_checked_when_read() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let ok = Answer {
        payload: Some(b"{\"ok\":true}".to_vec()),
        ..answer(OutcomeClass::Ok)
    };
    let (path, record) = journal.keep(&request, &ok, at(0)).unwrap().unwrap();
    let Lookup::Fresh(kept) = journal.lookup(&record.request_digest, at(1)) else {
        panic!("fresh");
    };
    assert_eq!(kept.answer(), ok);
    // A payload that is not the one the record names is a miss.
    std::fs::write(path.with_extension("payload"), b"tampered").unwrap();
    assert_eq!(
        journal.lookup(&record.request_digest, at(1)),
        Lookup::Missing
    );
    // A payload over the request's cap is not kept; the record is.
    let big = Answer {
        payload: Some(vec![b'x'; 2048]),
        ..answer(OutcomeClass::Ok)
    };
    let (path, record) = journal.keep(&request, &big, at(0)).unwrap().unwrap();
    assert_eq!(record.payload, None);
    let Lookup::Fresh(kept) = journal.lookup(&record.request_digest, at(1)) else {
        panic!("fresh");
    };
    assert_eq!(kept.payload, None);
    assert!(path.is_file());
}

#[test]
fn a_loose_roots_journal_is_the_users() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let loose = ProjectRoot::of(project.path()).unwrap();
    assert!(!loose.is_configured());
    assert_eq!(
        EffectJournal::of(&loose, Some(home.path())),
        EffectJournal::at(home.path())
    );
    assert_eq!(EffectJournal::of(&loose, None), EffectJournal::detached());
    std::fs::write(project.path().join("specforge.json"), "{}").unwrap();
    let configured = ProjectRoot::of(project.path()).unwrap();
    assert_eq!(
        EffectJournal::of(&configured, Some(home.path())),
        EffectJournal::at(project.path())
    );
    // A loose root's run writes into the user's journal, never into the directory it ran in.
    let journal = EffectJournal::of(&loose, Some(home.path()));
    journal
        .keep(
            &http("https://example.org/"),
            &answer(OutcomeClass::Ok),
            at(0),
        )
        .unwrap()
        .unwrap();
    assert!(home.path().join(JOURNAL_DIR).is_dir());
    assert!(!project.path().join(".specforge").exists());
}

#[test]
fn a_record_dated_in_the_future_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let (_, record) = journal
        .keep(&request, &answer(OutcomeClass::Ok), at(5_000))
        .unwrap()
        .unwrap();
    assert_eq!(
        journal.lookup(&record.request_digest, at(4_999)),
        Lookup::Missing
    );
    assert!(matches!(
        journal.lookup(&record.request_digest, at(5_000)),
        Lookup::Fresh(_)
    ));
}

/// Windows sets no mode, so it cannot tell a committed record this way.
#[cfg(unix)]
#[test]
fn a_record_or_payload_not_private_to_this_user_is_missing() {
    use std::os::unix::fs::{PermissionsExt as _, symlink};

    let dir = tempfile::tempdir().unwrap();
    let journal = EffectJournal::at(dir.path());
    let request = http("https://example.org/");
    let ok = Answer {
        payload: Some(b"body".to_vec()),
        ..answer(OutcomeClass::Ok)
    };
    let (path, record) = journal.keep(&request, &ok, at(0)).unwrap().unwrap();
    let digest = record.request_digest;
    let chmod = |path: &Path, mode| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    assert!(matches!(journal.lookup(&digest, at(1)), Lookup::Fresh(_)));

    // What a git checkout writes.
    chmod(&path, 0o644);
    assert_eq!(journal.lookup(&digest, at(1)), Lookup::Missing);
    chmod(&path, 0o600);
    let payload = path.with_extension("payload");
    chmod(&payload, 0o644);
    assert_eq!(journal.lookup(&digest, at(1)), Lookup::Missing);
    chmod(&payload, 0o600);
    assert!(matches!(journal.lookup(&digest, at(1)), Lookup::Fresh(_)));

    // A link a checkout can also write, to a private file that reads.
    let moved = path.with_extension("moved");
    std::fs::rename(&path, &moved).unwrap();
    symlink(&moved, &path).unwrap();
    assert_eq!(journal.lookup(&digest, at(1)), Lookup::Missing);
}
