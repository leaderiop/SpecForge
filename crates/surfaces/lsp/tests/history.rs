//! The language server and a commit (ADR 0287).

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use specforge_extension_sdk::prelude::*;
use specforge_lsp::changes::Change;
use specforge_test_macros::test as specforge_test;
use tower_lsp::lsp_types::{FileChangeType, FileEvent, Url};

use crate::recorder::last_codes;
use crate::served::Served;

fn declare(c: &mut ContributionsBuilder) {
    c.kind("thing", |k| {
        k.keyword("thing");
        k.field("origin", |f| {
            f.field_type(FieldType::String).immutable();
        });
    });
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

#[specforge_test(
    behavior = "watch_baseline_refs",
    verify = "the LSP republishes after a commit"
)]
fn the_lsp_republishes_after_a_commit() {
    let _installed =
        specforge_project::testing::history::install(Arc::new(specforge_git::GitCli::default()));
    let served = Served::new(&[("main.spec", "thing a \"A\" {\n  origin one\n}\n")])
        .extension("@t/immutable", declare);
    let root = served.root().canonicalize().unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "c"]);
    let mut served = served.open(&["main.spec"]);
    let main = served.uri("main.spec");

    let two = "thing a \"A\" {\n  origin two\n}\n";
    served.write("main.spec", two);
    served.edit("main.spec", two);
    let codes = last_codes(&served.sent(), &main).expect("main.spec is published");
    assert!(codes.contains(&"E084".to_string()), "{codes:?}");

    git(&root, &["commit", "-q", "-am", "two"]);
    let head = Url::from_file_path(root.join(".git/refs/heads/main")).unwrap();
    served.apply(Change::Watched(vec![FileEvent::new(
        head,
        FileChangeType::CHANGED,
    )]));
    let codes = last_codes(&served.sent(), &main).expect("main.spec is republished");
    assert!(!codes.contains(&"E084".to_string()), "{codes:?}");
}
