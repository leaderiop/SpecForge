//! The `syntax` port (ADR 0345 D1): source parsed by the host's reader inside the call's fs grant, git's at a revision.

use std::sync::Arc;

use specforge_effects::testing::{FakeFs, FakeGit, FakeHttp, FakeProcess, MemoryConsent};
use specforge_effects::{EffectJournal, EffectsCeiling, Grant, Ports, Provenance, Roots};
use specforge_extension_sdk::host::{self, HostError};
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::{
    NotParsed, Operation, SyntaxAnswer, SyntaxFiles, SyntaxOutcome, SyntaxRequest, SyntaxSource,
};
use specforge_source::{Deadline, SourceReader, SyntaxCache};
use specforge_wasm::testing::counted_reader;
use specforge_wasm::{Limits, PortCall, PortSession};

use super::{EXT, SHA, lent_over};

const QUERY: &str = "(function_item name: (identifier) @name) @definition.function";

fn request(source: SyntaxSource, files: SyntaxFiles) -> SyntaxRequest {
    SyntaxRequest {
        source,
        files,
        language: "rust".to_string(),
        query: QUERY.to_string(),
    }
}

fn project(paths: &[&str]) -> SyntaxRequest {
    request(
        SyntaxSource::Worktree {
            root: "project".to_string(),
        },
        SyntaxFiles::Paths(paths.iter().map(|p| p.to_string()).collect()),
    )
}

/// A call of the extension `ports.rs` serves, over `fs` and `git`.
fn call_over(fs: FakeFs, git: FakeGit) -> PortCall {
    lent_over(FakeHttp::new(), FakeProcess::new(), fs, git)
        .call
        .with_syntax(counted_reader())
}

fn session(call: &PortCall, limits: Limits) -> PortSession {
    PortSession::effectful(EXT, Operation::Collect, Some(call.clone()), limits)
}

/// Each file's function names, or its reason for not being parsed.
fn shown(answer: &SyntaxAnswer) -> Vec<(String, Vec<String>)> {
    answer
        .files
        .iter()
        .map(|file| {
            let what = match &file.outcome {
                SyntaxOutcome::Parsed { matches, .. } => matches
                    .iter()
                    .flat_map(|m| m.captures.iter().map(|c| c.text.clone()))
                    .collect(),
                SyntaxOutcome::NotParsed { reason } => vec![reason.name().to_string()],
            };
            (file.path.clone(), what)
        })
        .collect()
}

fn owned(rows: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
    rows.iter()
        .map(|(path, what)| {
            (
                path.to_string(),
                what.iter().map(|w| w.to_string()).collect(),
            )
        })
        .collect()
}

#[specforge_test_macros::test(
    behavior = "parse_source_through_the_syntax_import",
    verify = "a collector's syntax request inside its grant is parsed"
)]
fn a_collectors_syntax_request_inside_its_grant_is_parsed() {
    let call = call_over(
        FakeFs::new()
            .with_file("/project/src/a.rs", "fn a() {}\nfn b() {}\n")
            .with_file("/project/src/c.rs", "fn c() {}\n"),
        FakeGit::none(),
    );
    let answer = session(&call, Limits::CEILING)
        .syntax_parse(project(&["src/a.rs", "src/c.rs"]))
        .unwrap();
    assert_eq!(
        shown(&answer),
        owned(&[("src/a.rs", &["a", "b"]), ("src/c.rs", &["c"])])
    );
    assert_eq!(
        (answer.parsed, answer.cached, answer.truncated),
        (2, 0, false)
    );
    assert!(
        answer.files[0]
            .digest
            .as_deref()
            .unwrap()
            .starts_with("sha256:")
    );
    let work = call.take_syntax_work();
    assert_eq!((work.parsed, work.bytes), (2, 30));
    assert_eq!(call.take_syntax_work().parsed, 0);
    assert!(call.take_denials().is_empty());
}

#[test]
fn a_glob_is_walked_inside_the_grant() {
    let call = call_over(
        FakeFs::new()
            .with_file("/project/src/a.rs", "fn a() {}\n")
            .with_file("/project/src/deep/b.rs", "fn b() {}\n")
            .with_file("/project/src/c.ts", "function c() {}\n")
            .with_file("/project/other/d.rs", "fn d() {}\n"),
        FakeGit::none(),
    );
    let glob = request(
        SyntaxSource::Worktree {
            root: "project".to_string(),
        },
        SyntaxFiles::Glob("src/**/*.rs".to_string()),
    );
    let answer = session(&call, Limits::CEILING).syntax_parse(glob).unwrap();
    assert_eq!(
        shown(&answer),
        owned(&[("src/a.rs", &["a"]), ("src/deep/b.rs", &["b"])])
    );
}

#[specforge_test_macros::test(
    behavior = "parse_source_through_the_syntax_import",
    verify = "a syntax request outside the grant is answered denied(scope) and reported as E090"
)]
fn a_syntax_request_outside_the_grant_is_denied_scope_and_e090() {
    // An extension whose fs capability stats and nothing else.
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Stat])
            .why("look at the project"),
    );
    let ports = Ports {
        fs: Arc::new(FakeFs::new().with_file("/project/src/a.rs", "fn a() {}\n")),
        ..specforge_effects::testing::ports()
    };
    let roots = Roots::new("/project", "/project/spec");
    let grant = Grant::decide(
        &c.declaration(),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    let call = PortCall::online(&ports, EffectJournal::detached(), roots, grant, Vec::new())
        .with_syntax(counted_reader());
    let refused = session(&call, Limits::CEILING).syntax_parse(project(&["src/a.rs"]));
    assert!(
        matches!(&refused, Err(HostError::Denied(host::Denial::Scope(_)))),
        "{refused:?}"
    );
    let denials = call.take_denials();
    assert_eq!(denials.len(), 1, "{denials:?}");
    assert_eq!(denials[0].code, "E090");
}

#[test]
fn a_pure_call_asking_syntax_is_denied_call_kind() {
    let refused = PortSession::pure().syntax_parse(project(&["src/a.rs"]));
    assert!(
        matches!(&refused, Err(HostError::Denied(host::Denial::CallKind(_)))),
        "{refused:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "parse_source_through_the_syntax_import",
    verify = "a missing, a withheld and a too large file are each answered not_parsed"
)]
fn a_missing_a_withheld_and_a_too_large_file_are_each_not_parsed() {
    let huge = vec![b' '; (64 << 20) + 1];
    let call = call_over(
        FakeFs::new()
            .with_file("/project/src/a.rs", "fn a() {}\n")
            .with_file("/project/.env", "fn secret() {}\n")
            .with_file("/project/src/huge.rs", huge),
        FakeGit::none(),
    );
    let answer = session(&call, Limits::CEILING)
        .syntax_parse(project(&["src/a.rs", "src/gone.rs", ".env", "src/huge.rs"]))
        .unwrap();
    assert_eq!(
        shown(&answer),
        owned(&[
            ("src/a.rs", &["a"]),
            ("src/gone.rs", &["missing"]),
            (".env", &["denied"]),
            ("src/huge.rs", &["too_large"]),
        ])
    );
    // The file too large to parse is a W196; a missing or withheld one is the guest's to know.
    let denials = call.take_denials();
    assert_eq!(denials.len(), 1, "{denials:?}");
    assert_eq!(denials[0].code, "W196");
    assert!(
        denials[0]
            .message
            .contains("1 source files were not parsed: `src/huge.rs` (too_large)"),
        "{denials:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "parse_source_through_the_syntax_import",
    verify = "a revision's files are parsed from the commit's blobs"
)]
fn a_revision_is_parsed_from_the_commits_blobs() {
    let dir = std::env::temp_dir();
    let mut git = FakeGit::repo(&dir, SHA, Vec::new());
    git.blobs = vec![
        ("src/a.rs".into(), b"fn at_the_commit() {}\n".to_vec()),
        ("src/b.ts".into(), b"function b() {}\n".to_vec()),
        (".env".into(), b"fn secret() {}\n".to_vec()),
    ];
    git.listed = vec![("src/big.rs".into(), Some(32 << 20))];
    let call = call_over(FakeFs::new(), git);
    let at = |files| {
        request(
            SyntaxSource::Revision {
                commit: SHA.to_string(),
            },
            files,
        )
    };
    let answer = session(&call, Limits::CEILING)
        .syntax_parse(at(SyntaxFiles::Paths(vec![
            "src/a.rs".into(),
            ".env".into(),
            "src/big.rs".into(),
            "src/gone.rs".into(),
        ])))
        .unwrap();
    assert_eq!(
        shown(&answer),
        owned(&[
            ("src/a.rs", &["at_the_commit"]),
            (".env", &["denied"]),
            ("src/big.rs", &["too_large"]),
            ("src/gone.rs", &["missing"]),
        ])
    );
    let globbed = session(&call, Limits::CEILING)
        .syntax_parse(at(SyntaxFiles::Glob("src/*.rs".into())))
        .unwrap();
    assert_eq!(
        shown(&globbed),
        owned(&[
            ("src/a.rs", &["at_the_commit"]),
            ("src/big.rs", &["too_large"])
        ])
    );
    let not_a_commit = session(&call, Limits::CEILING).syntax_parse(request(
        SyntaxSource::Revision {
            commit: "HEAD~1".into(),
        },
        SyntaxFiles::Paths(vec!["src/a.rs".into()]),
    ));
    assert!(
        matches!(
            &not_a_commit,
            Err(HostError::Denied(host::Denial::Scope(_)))
        ),
        "{not_a_commit:?}"
    );
}

/// `n` functions, one a line.
fn functions(n: usize) -> String {
    (0..n).map(|i| format!("fn f{i}() {{}}\n")).collect()
}

#[specforge_test_macros::test(
    behavior = "parse_source_through_the_syntax_import",
    verify = "a syntax answer past the cap names the rest capped and is not truncated"
)]
fn a_syntax_answer_past_the_cap_names_the_rest_capped_and_is_not_truncated() {
    // A 1 MiB guest's answer cap is 256 KiB; each file's matches encode to about 100 KiB.
    let call = call_over(
        FakeFs::new()
            .with_file("/project/a.rs", functions(400))
            .with_file("/project/b.rs", functions(400))
            .with_file("/project/c.rs", functions(400))
            .with_file("/project/d.rs", functions(1)),
        FakeGit::none(),
    );
    let small = Limits {
        execution_ms: 1_000,
        memory_mb: 1,
    };
    let answer = session(&call, small)
        .syntax_parse(project(&["a.rs", "b.rs", "c.rs", "d.rs"]))
        .unwrap();
    // Only a glob's short walk truncates an answer: a capped file names itself.
    assert!(!answer.truncated);
    let reasons: Vec<Option<NotParsed>> = answer
        .files
        .iter()
        .map(|file| match file.outcome {
            SyntaxOutcome::NotParsed { reason } => Some(reason),
            SyntaxOutcome::Parsed { .. } => None,
        })
        .collect();
    assert_eq!(
        reasons,
        [None, None, Some(NotParsed::Capped), Some(NotParsed::Capped)]
    );
    // The answer counts the files it returns; the call's work is every parse made.
    assert_eq!((answer.parsed, answer.cached), (2, 0));
    assert_eq!(call.take_syntax_work().parsed, 4);
    // Capped files are the guest's to ask again, not a W196.
    assert!(call.take_denials().is_empty());
}

#[specforge_test_macros::test(
    behavior = "parse_source_through_the_syntax_import",
    verify = "files past the deadline are one W196 and broken files one I211 per call"
)]
fn files_past_the_deadline_are_one_w196_and_broken_files_one_i211_per_call() {
    let reader = SourceReader::new(SyntaxCache::detached()).with_deadline(Deadline::Progress(0));
    let call = call_over(
        FakeFs::new()
            .with_file("/project/large1.rs", functions(6000))
            .with_file("/project/large2.rs", functions(6000))
            .with_file("/project/broken1.rs", "fn a( { }\n")
            .with_file("/project/broken2.rs", "fn b() { let = ; }\n"),
        FakeGit::none(),
    )
    .with_syntax(Arc::new(reader));
    for pair in [["large1.rs", "broken1.rs"], ["large2.rs", "broken2.rs"]] {
        session(&call, Limits::CEILING)
            .syntax_parse(project(&pair))
            .unwrap();
    }
    let notes = call.take_denials();
    let codes: Vec<&str> = notes.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes.len(), 2, "{notes:?}");
    assert_eq!(codes, ["W196", "I211"], "{notes:?}");
    assert!(
        notes[0].message.contains(
            "2 source files were not parsed: `large1.rs` (deadline), `large2.rs` (deadline)"
        ),
        "{notes:?}"
    );
    assert!(
        notes[1]
            .message
            .contains("2 source files parsed with syntax errors: `broken1.rs` ("),
        "{notes:?}"
    );
    assert!(call.take_denials().is_empty());
}

#[test]
fn an_unknown_language_and_a_bad_query_fail_with_their_class() {
    let call = call_over(FakeFs::new(), FakeGit::none());
    let failed_class =
        |request: SyntaxRequest| match session(&call, Limits::CEILING).syntax_parse(request) {
            Err(HostError::Failed(failure)) => (failure.class, failure.message),
            other => panic!("{other:?}"),
        };
    let (class, message) = failed_class(SyntaxRequest {
        language: "cobol".into(),
        ..project(&["a.rs"])
    });
    assert_eq!(class, "no_grammar");
    assert!(message.contains("rust, typescript, tsx"), "{message}");
    let (class, message) = failed_class(SyntaxRequest {
        query: "(function_item) @name".into(),
        ..project(&["a.rs"])
    });
    assert_eq!(class, "invalid_query");
    assert!(message.contains("1:1"), "{message}");
}
