//! A source-code path a request names (infer_session's `source_file` and `source_roots`,
//! find_spec_for_source's `file_path`) is read from the project root into its key by one reader
//! (ADR 0253).

use serde_json::json;
use specforge_installed::Installed;
use specforge_ops::OpErrorKind;
use specforge_ops::read::infer::{
    self, Recorded, SessionField, SessionRequest, SessionStep, session,
};
use specforge_ops::read::navigate::{FileAnchorsField, FileMatch, anchors_of_file};
use specforge_project::{Environment, ExtensionLoad};

use crate::view_support::{Assembled, Viewed, config_read};

/// A project with `src/tiny.rs`, its manifest unwritten.
fn project() -> Assembled {
    let project = Assembled::assembling(|root| {
        Environment::assemble(
            config_read(root, json!({})),
            Installed::unread(root),
            ExtensionLoad::NoRuntime,
        )
    })
    .source("behavior a \"A\" {\n}\n");
    std::fs::create_dir_all(project.root().join("src")).unwrap();
    std::fs::write(project.root().join("src/tiny.rs"), "pub fn a() {}\n").unwrap();
    project
}

fn step(
    project: &Assembled,
    step: SessionStep<'_>,
) -> Result<infer::SessionOutcome, specforge_ops::Refusal<SessionField>> {
    session(
        &project.view(),
        &SessionRequest {
            step,
            caller: specforge_ops::testing::caller(),
        },
    )
}

fn mark(
    project: &Assembled,
    source_file: &str,
) -> Result<infer::SessionOutcome, specforge_ops::Refusal<SessionField>> {
    step(
        project,
        SessionStep::MarkAnalyzed {
            source_file,
            entities: &[],
        },
    )
}

fn start(
    project: &Assembled,
    root: &str,
) -> Result<infer::SessionOutcome, specforge_ops::Refusal<SessionField>> {
    step(
        project,
        SessionStep::Start {
            agent: None,
            source_roots: Some(&[root.to_string()]),
        },
    )
}

const LEAVES: &str = "a source file is named from the project root";

#[specforge_test_macros::test(
    behavior = "read_a_named_source_path",
    verify = "every spelling of a source-code path under the project root reads as its key"
)]
fn an_absolute_source_path_under_the_root_is_its_key() {
    let project = project();
    let file = project.root().join("src/tiny.rs");
    let marked = mark(&project, file.to_str().unwrap()).unwrap();
    match marked.recorded {
        Recorded::Marked { source_file, .. } => assert_eq!(source_file, "src/tiny.rs"),
        other => panic!("{other:?}"),
    }
    let src = project.root().join("src");
    start(&project, src.to_str().unwrap()).unwrap();
    let text = std::fs::read_to_string(project.root().join("specforge-infer.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(manifest["source_roots"], json!(["src"]));
}

#[specforge_test_macros::test(
    behavior = "read_a_named_source_path",
    verify = "a source-code path whose key lies outside the project root is refused in the named spec file's sentence"
)]
fn a_source_path_that_leaves_the_root_is_refused_in_one_sentence() {
    let project = project();
    for named in ["../x.rs", "/elsewhere/x.rs"] {
        let message = format!("'{named}' lies outside the project root: {LEAVES}");
        for refused in [mark(&project, named), start(&project, named)] {
            let refusal = refused.unwrap_err();
            assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput, "{named}");
            assert_eq!(refusal.error.code, "not_a_source_path", "{named}");
            assert_eq!(refusal.error.message, message);
        }
    }
    // A `..` that stays inside is applied where it stands (plan 04's mint).
    let marked = mark(&project, "src/../src/tiny.rs").unwrap();
    assert!(matches!(
        marked.recorded,
        Recorded::Marked { ref source_file, .. } if source_file == "src/tiny.rs"
    ));
}

#[specforge_test_macros::test(
    behavior = "mark_source_file_analyzed",
    verify = "mark refuses a directory as invalid input on source_file"
)]
fn a_marked_directory_is_invalid_input_on_source_file() {
    let project = project();
    let refusal = mark(&project, "src").unwrap_err();
    assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
    assert_eq!(refusal.subject, Some(SessionField::SourceFile));
}

#[cfg(unix)]
#[test]
fn a_marked_file_behind_a_link_that_leaves_is_refused_where_it_lands() {
    let project = project();
    let outside = tempfile::TempDir::new().unwrap();
    std::fs::write(outside.path().join("o.rs"), "pub fn o() {}\n").unwrap();
    std::os::unix::fs::symlink(outside.path(), project.root().join("src2")).unwrap();
    let refusal = mark(&project, "src2/o.rs").unwrap_err();
    assert_eq!(refusal.error.code, "outside_root");
    assert!(
        refusal
            .error
            .message
            .starts_with("'src2/o.rs' resolves to "),
        "{}",
        refusal.error.message
    );
    assert!(
        refusal
            .error
            .message
            .ends_with(", which bounds the request"),
        "{}",
        refusal.error.message
    );
}

#[specforge_test_macros::test(
    behavior = "read_a_named_source_path",
    verify = "every spelling of a source-code path under the project root reads as its key"
)]
fn the_anchors_read_their_query_from_the_project_root() {
    let project = project();
    project.write(
        "specforge-anchors.json",
        json!({"version": 1, "anchors": [{"entity_id": "alpha", "file": "src/a.rs", "line": 1,
               "symbol_name": "alpha", "item_kind": "fn", "scanner": "manual"}]})
        .to_string(),
    );
    let found = |query: &str| anchors_of_file(&project.view(), query).map(|f| f.match_mode);
    let file = project.root().join("src/a.rs");
    let dir = project.root().join("src");
    assert_eq!(found(file.to_str().unwrap()).unwrap(), FileMatch::Exact);
    assert_eq!(found(dir.to_str().unwrap()).unwrap(), FileMatch::Directory);
    for outside in ["../x.rs", "/elsewhere/a.rs", ""] {
        let refusal = anchors_of_file(&project.view(), outside).unwrap_err();
        assert_eq!(
            refusal.subject,
            Some(FileAnchorsField::FilePath),
            "{outside:?}"
        );
        assert_eq!(refusal.error.code, "not_a_source_path", "{outside:?}");
    }
}

#[specforge_test_macros::test(
    behavior = "mark_source_file_analyzed",
    verify = "mark replaces the entry of its file whatever spelling recorded it"
)]
#[ignore = "flipped by T3"]
fn marking_a_file_replaces_its_entry_recorded_absolute() {
    let project = project();
    let recorded = project.root().join("src/tiny.rs");
    project.write(
        "specforge-infer.json",
        json!({"version": 1, "source_roots": ["src"], "source_index": [
            {"path": recorded.to_str().unwrap(), "content_hash": "00", "entities_produced": [], "analyzed_at": "2026-10-01T00:00:00Z"}]})
        .to_string(),
    );
    start(&project, "src").unwrap();
    mark(&project, "src/tiny.rs").unwrap();
    let text = std::fs::read_to_string(project.root().join("specforge-infer.json")).unwrap();
    let written: serde_json::Value = serde_json::from_str(&text).unwrap();
    let paths: Vec<&str> = written["source_index"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["src/tiny.rs"]);
}
