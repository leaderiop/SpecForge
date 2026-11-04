//! The files family: what each checker plans for its text and finds in canned answers, as the host would send them.

use serde_json::json;
use specforge_protocol_types::{OutcomeClass, OutcomeData, Probe, ProbeOutcome, Root, StatEntry};

use super::{e103_at, judged, planned};
use crate::families::files::split_glob as glob_parts;

fn answer(class: OutcomeClass, data: OutcomeData) -> ProbeOutcome {
    ProbeOutcome {
        probe: Probe::FsStat {
            root: Root::Project,
            path: String::new(),
        },
        class,
        data,
        observed_at: 0,
        from_cache: false,
    }
}

fn stat(entry: Option<StatEntry>, size: u64, near: Option<&str>) -> OutcomeData {
    OutcomeData::Stat {
        entry,
        size,
        near: near.map(str::to_string),
    }
}

fn missing(near: Option<&str>) -> ProbeOutcome {
    answer(OutcomeClass::Missing, stat(None, 0, near))
}

fn found(entry: StatEntry, size: u64) -> ProbeOutcome {
    answer(OutcomeClass::Ok, stat(Some(entry), size, None))
}

fn read(bytes: &[u8], truncated: bool) -> ProbeOutcome {
    answer(
        OutcomeClass::Ok,
        OutcomeData::Bytes {
            bytes: bytes.to_vec(),
            length: bytes.len() as u64,
            truncated,
        },
    )
}

fn codes(findings: &[specforge_protocol_types::CheckFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.code.as_str()).collect()
}

#[test]
fn files_file_stats_under_its_root_and_names_the_hosts_sibling() {
    let plan = planned("file", "docs/a.md", json!({"root": "spec"}));
    assert_eq!(
        plan.probes,
        [Probe::FsStat {
            root: Root::Spec,
            path: "docs/a.md".into()
        }]
    );
    let default = planned("file", "a.md", json!({}));
    assert!(matches!(
        &default.probes[0],
        Probe::FsStat {
            root: Root::Project,
            ..
        }
    ));

    let gone = judged("file", "docs/a.md", json!({}), missing(Some("docs/b.md")));
    assert_eq!(codes(&gone), ["E016"]);
    assert_eq!(gone[0].at, Some([0, 9]));
    assert_eq!(gone[0].help.as_deref(), Some("did you mean 'docs/b.md'?"));
    assert!(judged("file", "a.md", json!({}), found(StatEntry::File, 3)).is_empty());
    assert_eq!(
        codes(&judged("file", "d", json!({}), found(StatEntry::Dir, 0))),
        ["E016"]
    );
}

#[test]
fn files_dir_wants_a_directory() {
    assert!(judged("dir", "d", json!({}), found(StatEntry::Dir, 0)).is_empty());
    assert_eq!(
        codes(&judged("dir", "d", json!({}), found(StatEntry::File, 1))),
        ["E016"]
    );
    assert_eq!(
        codes(&judged("dir", "d", json!({}), missing(None))),
        ["E016"]
    );
}

#[test]
fn files_an_answer_not_performed_is_not_judged() {
    assert!(judged(
        "file",
        "a.md",
        json!({}),
        answer(OutcomeClass::Denied, OutcomeData::None)
    )
    .is_empty());
    assert!(planned("file", "", json!({})).probes.is_empty());
}

#[test]
fn files_glob_walks_its_literal_directories() {
    assert_eq!(glob_parts("docs/*.md"), ("docs", "*.md"));
    assert_eq!(glob_parts("a/b/**/x.md"), ("a/b", "**/x.md"));
    assert_eq!(glob_parts("*.md"), ("", "*.md"));
    assert_eq!(glob_parts("docs/readme.md"), ("docs", "readme.md"));
    let plan = planned("glob", "*.md", json!({"root": "spec"}));
    assert_eq!(
        plan.probes,
        [Probe::FsWalk {
            root: Root::Spec,
            path: "<spec>".into(),
            glob: "*.md".into()
        }]
    );
    assert_eq!(e103_at("glob", "a/../b", json!({})), [2, 4]);
}

#[test]
fn files_glob_counts_its_matches() {
    let walked = |entries: &[&str], truncated| {
        answer(
            OutcomeClass::Ok,
            OutcomeData::Walk {
                entries: entries.iter().map(|e| e.to_string()).collect(),
                truncated,
            },
        )
    };
    assert_eq!(
        codes(&judged("glob", "*.md", json!({}), walked(&[], false))),
        ["E016"]
    );
    assert!(judged("glob", "*.md", json!({}), walked(&["a.md"], false)).is_empty());
    let over = judged(
        "glob",
        "*.md",
        json!({"max": 1}),
        walked(&["a.md", "b.md"], false),
    );
    assert_eq!(codes(&over), ["W176"]);
    assert!(judged("glob", "*.md", json!({"min": 3}), walked(&["a"], true)).is_empty());
    assert!(judged("glob", "*.md", json!({"min": 0}), missing(None)).is_empty());
}

#[test]
fn files_size_holds_its_bounds() {
    let args = json!({"min": 2, "max": 4});
    assert!(judged("size", "a", args.clone(), found(StatEntry::File, 3)).is_empty());
    assert_eq!(
        codes(&judged(
            "size",
            "a",
            args.clone(),
            found(StatEntry::File, 1)
        )),
        ["W176"]
    );
    assert_eq!(
        codes(&judged(
            "size",
            "a",
            args.clone(),
            found(StatEntry::File, 5)
        )),
        ["W176"]
    );
    assert_eq!(codes(&judged("size", "a", args, missing(None))), ["E016"]);
}

#[test]
fn files_digest_compares_the_pin() {
    let pin = "a.bin@sha256:".to_string() + &"ab".repeat(32);
    let plan = planned("digest", &pin, json!({}));
    assert_eq!(
        plan.probes,
        [Probe::FsDigest {
            root: Root::Project,
            path: "a.bin".into()
        }]
    );
    let digest = |sha256: &str| {
        answer(
            OutcomeClass::Ok,
            OutcomeData::Digest {
                sha256: sha256.into(),
                length: 1,
            },
        )
    };
    assert!(judged("digest", &pin, json!({}), digest(&"ab".repeat(32))).is_empty());
    let other = judged("digest", &pin, json!({}), digest(&"cd".repeat(32)));
    assert_eq!(codes(&other), ["E096"]);
    assert_eq!(other[0].at, Some([13, 77]));
    let gone = judged("digest", &pin, json!({}), missing(None));
    assert_eq!((codes(&gone), gone[0].at), (vec!["E016"], Some([0, 5])));
    assert_eq!(e103_at("digest", "a.bin@sha256:12", json!({})), [13, 15]);
}

#[test]
fn files_encoding_reads_utf8_the_bom_and_line_ends() {
    assert!(judged("encoding", "a", json!({}), read(b"a\nb\n", false)).is_empty());
    assert_eq!(
        codes(&judged("encoding", "a", json!({}), read(b"\xff", false))),
        ["W176"]
    );
    assert_eq!(
        codes(&judged(
            "encoding",
            "a",
            json!({}),
            read(b"\xEF\xBB\xBFa", false)
        )),
        ["W176"]
    );
    assert!(judged(
        "encoding",
        "a",
        json!({"bom": true}),
        read(b"\xEF\xBB\xBFa", false)
    )
    .is_empty());
    let crlf = read(b"a\r\nb\r\n", false);
    assert_eq!(
        codes(&judged(
            "encoding",
            "a",
            json!({"line_ends": "lf"}),
            crlf.clone()
        )),
        ["W176"]
    );
    assert!(judged("encoding", "a", json!({"line_ends": "crlf"}), crlf).is_empty());
    // A read cut inside `é`.
    assert!(judged("encoding", "a", json!({}), read(b"a\xC3", true)).is_empty());
}

#[test]
fn files_line_range_reads_the_lines() {
    let plan = planned("line_range", "a.rs:2-3", json!({}));
    assert!(matches!(&plan.probes[0], Probe::FsRead { path, .. } if path == "a.rs"));
    assert!(judged("line_range", "a.rs:2-3", json!({}), read(b"1\n2\n3", false)).is_empty());
    let past = judged(
        "line_range",
        "a.rs:2-4",
        json!({}),
        read(b"1\n2\n3\n", false),
    );
    assert_eq!((codes(&past), past[0].at), (vec!["W176"], Some([5, 8])));
    assert!(judged("line_range", "a.rs:2-9", json!({}), read(b"1\n2\n", true)).is_empty());
    assert_eq!(
        codes(&judged("line_range", "a.rs:1", json!({}), missing(None))),
        ["E016"]
    );
    assert_eq!(e103_at("line_range", "a.rs:x", json!({})), [5, 6]);
}

#[test]
fn files_carry_their_switch_and_scope() {
    let declaration = super::declaration();
    let files: Vec<_> = declaration
        .checkers
        .iter()
        .filter(|checker| checker.when == ["families.files"])
        .collect();
    assert_eq!(files.len(), 7);
    assert!(files.iter().all(|checker| checker.scopes == ["files"]));
    assert!(files
        .iter()
        .all(|checker| checker.root_arg.as_deref() == Some("root")));
}
