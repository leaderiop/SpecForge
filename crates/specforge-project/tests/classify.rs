//! What a changed path is to a project session (behavior
//! `classify_project_changes`): a source, an environment input, a check
//! input, or nothing.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use specforge_project::{InputRole, ProjectSession, SharedRuntime, UpdateKind};
use specforge_test::prelude::*;
use tempfile::TempDir;

use crate::check_passes::{SPEC, passes_extension, project as passes_project, write_cache};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A project whose `specforge.json` is `config`, opened with no runtime
/// (no extension loads; the config is still read).
fn open(config: serde_json::Value, files: &[(&str, &str)]) -> (TempDir, ProjectSession) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    for (path, text) in files {
        write(dir.path(), path, text);
    }
    let session = ProjectSession::open_with_runtime(dir.path(), None);
    (dir, session)
}

fn source(key: &str) -> InputRole {
    InputRole::Source(key.to_string())
}

#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a discovered .spec file is a source change keyed relative to the spec root"
)]
fn a_discovered_spec_file_is_a_source_keyed_from_the_spec_root() {
    let (dir, session) = open(
        json!({"name": "c", "version": "0.1.0", "spec_root": "spec"}),
        &[
            ("spec/a.spec", "term alpha \"A\" {\n}\n"),
            ("spec/sub/b.spec", "term beta \"B\" {\n}\n"),
        ],
    );
    let root = dir.path();
    let canonical_root = fs::canonicalize(root).unwrap();

    assert_eq!(
        session.classify(&root.join("spec/a.spec")),
        source("a.spec")
    );
    assert_eq!(
        session.classify(&root.join("spec/sub/b.spec")),
        source("sub/b.spec")
    );
    // The same file however the directory is spelled (a symlinked temp dir).
    assert_eq!(
        session.classify(&canonical_root.join("spec/a.spec")),
        source("a.spec")
    );
    // A deleted (or not yet written) file is keyed the same way.
    assert_eq!(
        session.classify(&root.join("spec/gone.spec")),
        source("gone.spec")
    );
    assert_eq!(
        session.source_key(&root.join("spec/sub/b.spec")),
        "sub/b.spec"
    );
    // A .spec file outside the spec root is not a source of the project.
    assert_eq!(
        session.classify(&root.join("outside.spec")),
        InputRole::Unrelated
    );

    // A batch: sorted, deduplicated keys, nothing else.
    let paths = [
        root.join("spec/sub/b.spec"),
        root.join("spec/a.spec"),
        canonical_root.join("spec/a.spec"),
        root.join("notes.txt"),
    ];
    let changes = session.changes(paths.iter().map(|p| p.as_path()));
    assert_eq!(changes.sources, vec!["a.spec", "sub/b.spec"]);
    assert!(!changes.environment && !changes.check_inputs);
}

#[specforge_test(
    behavior = "classify_project_changes",
    verify = "specforge.json, specforge.lock and a loaded extension module are environment changes"
)]
fn config_lock_and_loaded_modules_are_environment_inputs() {
    let (dir, session) = open(
        json!({
            "name": "c",
            "version": "0.1.0",
            "extensions": ["@specforge/software", "@acme/local=ext/local.wasm", "@acme/installed"]
        }),
        &[("main.spec", "")],
    );
    let root = dir.path();

    let inputs = session.environment().inputs();
    assert_eq!(inputs.config, root.join("specforge.json"));
    assert_eq!(inputs.lock, root.join("specforge.lock"));
    // A builtin has no module on disk; a local entry and an installed one do.
    assert_eq!(
        inputs.modules,
        vec![
            root.join("ext/local.wasm"),
            root.join(".specforge/extensions/@acme/installed/extension.wasm"),
        ]
    );
    for path in [
        "specforge.json",
        "specforge.lock",
        "ext/local.wasm",
        ".specforge/extensions/@acme/installed/extension.wasm",
    ] {
        assert_eq!(
            session.classify(&root.join(path)),
            InputRole::Environment,
            "{path}"
        );
    }
    let changes = session.changes([root.join("specforge.lock").as_path()]);
    assert!(changes.environment && changes.sources.is_empty());
}

#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a .wasm file no extension loads changes nothing"
)]
fn an_unloaded_wasm_changes_nothing() {
    let (dir, session) = open(
        json!({"name": "c", "version": "0.1.0", "extensions": ["@acme/local=ext/local.wasm"]}),
        &[("main.spec", "")],
    );
    let root = dir.path();

    for path in ["target/x.wasm", "ext/other.wasm", "plugin.wasm"] {
        assert_eq!(
            session.classify(&root.join(path)),
            InputRole::Unrelated,
            "{path}"
        );
    }
    let changes = session.changes([root.join("target/x.wasm").as_path()]);
    assert!(changes.is_empty());
}

#[specforge_test(
    behavior = "classify_project_changes",
    verify = "specforge-cache.json re-runs the checks without re-parsing"
)]
fn the_build_cache_reruns_the_checks_only() {
    let dir = passes_project(SPEC);
    let root = dir.path();
    let ext = Arc::new(passes_extension());
    let mut session =
        ProjectSession::open_with_runtime(root, Some(Arc::clone(&ext) as SharedRuntime));
    let cache = root.join(specforge_project::BUILD_CACHE_FILE);
    assert_eq!(
        session.environment().inputs().check_inputs,
        vec![cache.clone()]
    );
    assert!(session.diagnostics().iter().all(|d| d.code != "W144"));

    // An invalid cache: the check passes report W144 when they read it.
    write_cache(&dir, "{ not json");
    assert_eq!(session.classify(&cache), InputRole::CheckInput);
    let changes = session.changes([cache.as_path()]);
    assert!(changes.check_inputs && !changes.environment && changes.sources.is_empty());

    let update = session.apply(&changes).expect("a check input changed");
    assert_eq!(update.kind, UpdateKind::Checks);
    assert!(
        update.rebuilt_files.is_empty(),
        "{:?}",
        update.rebuilt_files
    );
    assert!(update.delta.is_empty());
    assert!(update.diagnostics.iter().any(|d| d.code == "W144"));

    // Without a check pass, the cache is no input at all.
    let (plain, session) = open(json!({"name": "c", "version": "0.1.0"}), &[("a.spec", "")]);
    let cache = plain.path().join(specforge_project::BUILD_CACHE_FILE);
    assert!(session.environment().inputs().check_inputs.is_empty());
    assert_eq!(session.classify(&cache), InputRole::Unrelated);
}

#[specforge_test(
    behavior = "classify_project_changes",
    verify = "a file a file_reference field names re-runs the checks"
)]
fn a_referenced_file_reruns_the_checks() {
    let dir = passes_project("gadget doc \"Doc\" {\n  docs [\"docs/guide.md\"]\n}\n");
    let root = dir.path();
    let ext = Arc::new(passes_extension());
    let mut session =
        ProjectSession::open_with_runtime(root, Some(Arc::clone(&ext) as SharedRuntime));
    let e016 = |session: &ProjectSession| {
        session
            .diagnostics()
            .iter()
            .filter(|d| d.code == "E016")
            .count()
    };
    assert_eq!(e016(&session), 1, "{:?}", session.diagnostics());

    write(root, "docs/guide.md", "# Guide\n");
    let guide = root.join("docs/guide.md");
    assert_eq!(session.classify(&guide), InputRole::CheckInput);
    let update = session
        .apply(&session.changes([guide.as_path()]))
        .expect("a referenced file changed");
    assert_eq!(update.kind, UpdateKind::Checks);
    assert_eq!(e016(&session), 0, "{:?}", session.diagnostics());
}

#[specforge_test(
    behavior = "classify_project_changes",
    verify = "an excluded or undiscovered .spec file changes nothing"
)]
fn excluded_and_undiscovered_spec_files_change_nothing() {
    let (dir, session) = open(
        json!({"name": "c", "version": "0.1.0", "spec_root": "spec", "exclude": ["drafts"]}),
        &[("spec/a.spec", "")],
    );
    let root = dir.path();

    for path in [
        "spec/drafts/x.spec",
        "spec/target/y.spec",
        "spec/node_modules/z.spec",
        "spec/notes.md",
        "outside/w.spec",
    ] {
        assert_eq!(
            session.classify(&root.join(path)),
            InputRole::Unrelated,
            "{path}"
        );
    }
    let paths = [root.join("spec/drafts/x.spec"), root.join("spec/notes.md")];
    assert!(
        session
            .changes(paths.iter().map(|p| p.as_path()))
            .is_empty()
    );
}

/// The directories a watcher must watch: the root, a spec root outside
/// it, and the directory of a module outside both.
#[test]
fn watch_roots_cover_every_input() {
    let outside = TempDir::new().unwrap();
    let module_dir = fs::canonicalize(outside.path()).unwrap().join("mods");
    fs::create_dir_all(&module_dir).unwrap();
    let module = module_dir.join("ext.wasm");
    let (dir, session) = open(
        json!({
            "name": "c",
            "version": "0.1.0",
            "spec_root": "spec",
            "extensions": [format!("@acme/far={}", module.display())]
        }),
        &[("spec/a.spec", "")],
    );
    let root = fs::canonicalize(dir.path()).unwrap();
    assert_eq!(session.watch_roots(), vec![root, module_dir]);
}
