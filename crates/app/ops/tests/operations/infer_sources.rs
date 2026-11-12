//! The files inference covers (ADR 0113, ADR 0199): the files item facts locate, held inside the project root,
//! listed once.

use std::path::Path;

use serde_json::json;
use specforge_ops::read::infer::{lint, progress};
use specforge_project::testing::{item_declarations, item_node};
use specforge_test::prelude::*;
use tempfile::TempDir;

use crate::view_support::{Assembled, Viewed};

/// A project with `src/lib.rs`, whose graph holds an item fact there.
fn project() -> Assembled {
    let mut project = Assembled::new().declarations(item_declarations());
    project
        .graph_mut()
        .add_node(item_node("rust:p::inside", "src/lib.rs", 1));
    write(project.root(), "src/lib.rs", "pub fn inside() {}\n");
    project
}

fn write(root: &Path, name: &str, text: &str) {
    let file = root.join(name);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

/// A directory beside the project's root, holding `secret.rs`.
fn outside(project: &Assembled) -> TempDir {
    let dir = TempDir::new_in(project.root().parent().unwrap()).unwrap();
    write(dir.path(), "secret.rs", "pub fn secret_outside() {}\n");
    dir
}

fn manifest(project: &Assembled, text: serde_json::Value) {
    write(project.root(), "specforge-infer.json", &text.to_string());
}

#[cfg(unix)]
fn link(target: &Path, name: &Path) {
    std::fs::create_dir_all(name.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(target, name).unwrap();
}

#[specforge_test(
    behavior = "discover_inference_sources",
    verify = "a file two source roots reach is one source"
)]
fn a_file_two_source_roots_reach_is_one_source() {
    let project = project();
    manifest(
        &project,
        json!({"version": 1, "source_roots": ["src", "."]}),
    );
    let progress = progress(&project.view()).unwrap();
    assert_eq!(progress.summary.files_total, 1, "{progress:?}");
    assert_eq!(progress.unanalyzed, ["src/lib.rs"]);
}

#[specforge_test(
    behavior = "discover_inference_sources",
    verify = "no inference source lands outside the project root"
)]
fn a_source_root_outside_the_project_adds_no_source() {
    let project = project();
    let outside = outside(&project);
    let name = outside.path().file_name().unwrap().to_string_lossy();
    manifest(
        &project,
        json!({
            "version": 1,
            "source_roots": [format!("../{name}"), outside.path().to_string_lossy()],
        }),
    );
    let progress = progress(&project.view()).unwrap();
    assert_eq!(progress.summary.files_total, 0, "{progress:?}");
    assert!(progress.unanalyzed.is_empty(), "{progress:?}");
}

#[cfg(unix)]
#[specforge_test(
    behavior = "discover_inference_sources",
    verify = "an indexed path outside the project root is deleted, never read"
)]
fn an_indexed_path_outside_the_project_is_deleted_and_never_read() {
    let project = project();
    let outside = outside(&project);
    link(outside.path(), &project.root().join("src/ext"));
    let name = outside.path().file_name().unwrap().to_string_lossy();
    let escaping = format!("../{name}/secret.rs");
    let entry = |path: &str| {
        json!({"path": path, "content_hash": "00", "entities_produced": ["x"],
               "analyzed_at": "2026-10-01T00:00:00Z"})
    };
    manifest(
        &project,
        json!({
            "version": 1,
            "source_roots": ["src"],
            "source_index": [entry(&escaping), entry("src/ext/secret.rs")],
        }),
    );
    let progress = progress(&project.view()).unwrap();
    assert!(progress.stale.is_empty(), "{progress:?}");
    assert_eq!(
        progress.deleted,
        [escaping, "src/ext/secret.rs".to_string()]
    );
    let diagnostics = lint(&project.view());
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[specforge_test(
    behavior = "read_a_recorded_source_path",
    verify = "an index entry recorded by its absolute path under the project root is that file"
)]
fn an_index_entry_recorded_absolute_under_the_root_is_its_file() {
    let project = project();
    write(project.root(), "src/net/wire.rs", "pub fn wire() {}\n");
    let root = project.root().to_path_buf();
    let canonical = std::fs::canonicalize(&root).unwrap();
    let first = format!("{}/src/lib.rs", root.display());
    let second = format!("{}/src/net/wire.rs", canonical.display());
    manifest(
        &project,
        json!({"version": 1, "source_roots": ["src"], "source_index": [
            {"path": first, "content_hash": "00", "entities_produced": [], "analyzed_at": "2026-10-01T00:00:00Z"},
            {"path": second, "content_hash": "00", "entities_produced": [], "analyzed_at": "2026-10-01T00:00:00Z"}]}),
    );
    let progress = progress(&project.view()).unwrap();
    assert_eq!(progress.stale, [first, second]);
    assert!(progress.deleted.is_empty(), "{progress:?}");
    assert!(progress.unanalyzed.is_empty(), "{progress:?}");
    let lint = lint(&project.view());
    assert_eq!(
        lint.iter().filter(|d| d.code == "I200").count(),
        2,
        "{lint:?}"
    );
}

#[specforge_test(
    behavior = "read_a_recorded_source_path",
    verify = "a source root recorded by its absolute path under the project root is walked"
)]
fn a_source_root_recorded_absolute_under_the_root_is_walked() {
    let project = project();
    let root = project.root().join("src");
    manifest(
        &project,
        json!({"version": 1, "source_roots": [root.to_str().unwrap()]}),
    );
    let progress = progress(&project.view()).unwrap();
    assert_eq!(progress.summary.files_total, 1, "{progress:?}");
    assert_eq!(progress.unanalyzed, ["src/lib.rs"]);
}

#[specforge_test(
    behavior = "compute_inference_summary",
    verify = "files_analyzed counts the inference sources the index records"
)]
fn files_analyzed_counts_the_indexed_inference_sources() {
    let project = project();
    manifest(
        &project,
        json!({"version": 1, "source_roots": ["src"], "source_index": [
            {"path": "src/gone.rs", "content_hash": "00", "entities_produced": [], "analyzed_at": "2026-10-01T00:00:00Z"}]}),
    );
    let gone = progress(&project.view()).unwrap();
    assert_eq!(gone.summary.files_total, 1);
    assert_eq!(gone.summary.files_analyzed, 0, "{gone:?}");
    assert_eq!(gone.unanalyzed, ["src/lib.rs"]);

    let hash = "00";
    manifest(
        &project,
        json!({"version": 1, "source_roots": ["src"], "source_index": [
            {"path": "./src/lib.rs", "content_hash": hash, "entities_produced": [], "analyzed_at": "2026-10-01T00:00:00Z"},
            {"path": "src/lib.rs", "content_hash": hash, "entities_produced": [], "analyzed_at": "2026-10-01T00:00:00Z"}]}),
    );
    let twice = progress(&project.view()).unwrap();
    assert_eq!(twice.summary.files_analyzed, 1, "{twice:?}");
}

#[specforge_test(
    behavior = "discover_inference_sources",
    verify = "a source is a file an item fact locates"
)]
fn a_source_is_a_file_an_item_fact_locates() {
    let mut project = project();
    // A file no item fact locates is no source; a file only a fact locates is one.
    write(project.root(), "src/notes.rs", "pub fn unread() {}\n");
    project
        .graph_mut()
        .add_node(item_node("rust:p::gen::made", "src/gen/made.rs", 3));
    let progress = progress(&project.view()).unwrap();
    assert_eq!(progress.unanalyzed, ["src/gen/made.rs", "src/lib.rs"]);
}

#[specforge_test(
    behavior = "discover_inference_sources",
    verify = "with no item facts, progress counts no source file"
)]
fn with_no_item_facts_progress_counts_no_source_file() {
    let project = Assembled::new().declarations(item_declarations());
    write(project.root(), "src/lib.rs", "pub fn inside() {}\n");
    let progress = progress(&project.view()).unwrap();
    assert_eq!(progress.summary.files_total, 0, "{progress:?}");
    assert!(progress.unanalyzed.is_empty(), "{progress:?}");
}
