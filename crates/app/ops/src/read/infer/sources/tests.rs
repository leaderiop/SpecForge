use super::*;
use crate::read::infer::manifest::SourceFileEntry;
use crate::testing::Viewed;
use specforge_project::testing::{Assembled, item_declarations, item_node};
use tempfile::TempDir;

#[test]
fn content_hash_is_sha256() {
    let hash = content_hash(b"fn main() {}");
    assert_eq!(hash.len(), 64);
    assert_eq!(
        content_hash(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

fn tree(files: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for file in files {
        let path = dir.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
    }
    dir
}

/// A project whose graph holds an item fact in each of `files`.
fn located(files: &[&str]) -> Assembled {
    let mut project = Assembled::new().declarations(item_declarations());
    for (n, file) in files.iter().enumerate() {
        project
            .graph_mut()
            .add_node(item_node(&format!("rust:p::i{n}"), file, 1));
    }
    project
}

fn keys(sources: &InferenceSources) -> Vec<&str> {
    sources.keys().iter().map(SourceKey::as_str).collect()
}

#[test]
fn no_roots_is_the_project_root() {
    let project = located(&["a.rs", "src/b.rs", "src/b.rs", "../out.rs"]);
    let view = project.view();
    let sources = InferenceSources::of(&view, project.root(), &InferenceManifest::default());
    assert_eq!(keys(&sources), ["a.rs", "src/b.rs"]);
}

#[test]
fn the_sources_are_the_files_item_facts_locate_under_a_root() {
    let project = located(&["src/a.rs", "src/b/c.rs", "srcx/d.rs", "other/e.rs"]);
    let manifest = InferenceManifest {
        source_roots: vec!["src".into()],
        ..Default::default()
    };
    let view = project.view();
    let sources = InferenceSources::of(&view, project.root(), &manifest);
    assert_eq!(keys(&sources), ["src/a.rs", "src/b/c.rs"]);
}

#[test]
fn indexed_states_are_current_changed_deleted_or_unreadable() {
    let dir = tree(&["a.rs", "b.rs", "d/x.rs"]);
    std::fs::write(dir.path().join("a.rs"), "one\ntwo").unwrap();
    let mut manifest = InferenceManifest::default();
    let hash_a = content_hash(b"one\ntwo");
    for (path, hash) in [
        ("./a.rs", hash_a.as_str()),
        ("b.rs", "stale"),
        ("gone.rs", "x"),
        ("../outside.rs", "x"),
        ("/abs.rs", "x"),
        ("d", "x"),
    ] {
        manifest
            .source_index
            .push(SourceFileEntry::new(path, hash, vec![], "t"));
    }
    let states: Vec<IndexState> = ProjectFiles::at(dir.path())
        .indexed(&manifest)
        .iter()
        .map(|i| i.state)
        .collect();
    assert_eq!(
        states,
        [
            IndexState::Current { lines: Some(2) },
            IndexState::Changed { lines: Some(1) },
            IndexState::Deleted,
            IndexState::Deleted,
            IndexState::Deleted,
            IndexState::Unreadable,
        ]
    );
}
