//! A syntax request on the real file system parses only what the fs port would read (ADR 0345 D1): a link out of the
//! root, a `..` path and a file with a second name are answered `not_parsed(denied)`.

use std::sync::Arc;

use specforge_effects::testing::{self, MemoryConsent};
use specforge_effects::{EffectJournal, EffectsCeiling, Grant, Ports, Provenance, Roots};
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::{
    SyntaxAnswer, SyntaxFiles, SyntaxOutcome, SyntaxRequest, SyntaxSource,
};
use specforge_wasm::testing::counted_reader;
use specforge_wasm::{Limits, PortCall, PortSession};

const EXT: &str = "@t/syntax";

fn request(paths: &[&str]) -> SyntaxRequest {
    SyntaxRequest {
        source: SyntaxSource::Worktree {
            root: "project".into(),
        },
        files: SyntaxFiles::Paths(paths.iter().map(|p| p.to_string()).collect()),
        language: "rust".into(),
        query: "(function_item name: (identifier) @name) @definition.function".into(),
    }
}

/// Each file's function names, or its reason for not being parsed.
fn shown(answer: &SyntaxAnswer) -> Vec<(&str, Vec<&str>)> {
    answer
        .files
        .iter()
        .map(|file| {
            let what = match &file.outcome {
                SyntaxOutcome::Parsed { matches, .. } => matches
                    .iter()
                    .flat_map(|m| m.captures.iter().map(|c| c.text.as_str()))
                    .collect(),
                SyntaxOutcome::NotParsed { reason } => vec![reason.name()],
            };
            (file.path.as_str(), what)
        })
        .collect()
}

#[cfg(unix)]
#[specforge_test_macros::test(
    behavior = "parse_source_through_the_syntax_import",
    verify = "a syntax request over a link out of its root, a hard link or a .. path parses nothing outside"
)]
fn a_syntax_request_over_a_link_a_hard_link_or_a_parent_path_parses_nothing_outside() {
    let base = tempfile::tempdir().unwrap();
    let base = base.path().canonicalize().unwrap();
    let (root, outside) = (base.join("proj"), base.join("outside"));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.rs"), "fn secret() {}\n").unwrap();
    std::fs::write(root.join("a.rs"), "fn a() {}\n").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("out")).unwrap();
    std::fs::hard_link(outside.join("secret.rs"), root.join("hard.rs")).unwrap();
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.capability(
        Capability::fs("source")
            .ops(&[FsAccess::Read, FsAccess::Walk])
            .why("read the project's source"),
    );
    let roots = Roots::new(&root, &root);
    let grant = Grant::decide(
        &c.declaration(),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        &roots,
        &MemoryConsent::approving_everything(),
    );
    let ports = Ports {
        fs: Arc::new(specforge_fs::LocalFs),
        ..testing::ports()
    };
    let call = PortCall::online(&ports, EffectJournal::detached(), roots, grant, Vec::new())
        .with_syntax(counted_reader());
    let mut session =
        PortSession::effectful(EXT, Operation::Command, Some(call.clone()), Limits::CEILING);

    let answer = session
        .syntax_parse(request(&[
            "a.rs",
            "out/secret.rs",
            "hard.rs",
            "../outside/secret.rs",
        ]))
        .unwrap();
    assert_eq!(
        shown(&answer),
        [
            ("a.rs", vec!["a"]),
            ("out/secret.rs", vec!["denied"]),
            ("hard.rs", vec!["denied"]),
            ("../outside/secret.rs", vec!["denied"]),
        ]
    );
    assert_eq!(answer.parsed, 1);
    // What the port refused is the guest's to know, not a W196.
    assert!(call.take_denials().is_empty());
}
