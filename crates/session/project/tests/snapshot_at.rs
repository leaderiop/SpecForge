//! A project's graph at a git revision (ADR 0376 D1, D2, D4): read through the git port with no checkout, built with
//! the current environment, keyed by the revision it was named by, and served the fact stores its ledger records
//! there, never collected.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use specforge_effects::testing::{FakeProcess, MemoryConsent};
use specforge_effects::{EffectsMode, SharedGit};
use specforge_git::testing::Repo;
use specforge_project::CompiledProject;
use specforge_project::harness::{LEDGER_FILE, Ledger, Line};
use specforge_project::snapshot::{
    AtError, Comparison, FactsAt, NoteAbout, Side, Snapshot, SnapshotKey, compare,
};
use specforge_project::testing::{COLLECTED_EXTENSIONS, Collected, TestProject, stored};
use specforge_protocol_types::DeltaBase;
use specforge_registry::{Disk, PlainDisk};
use specforge_test::prelude::*;

const FIRST: &str = "thing x \"X\" {\n  note \"first\"\n}\n\nthing y \"Y\" {\n}\n";
const SECOND: &str = "thing x \"X\" {\n  note \"second\"\n}\n\nthing z \"Z\" {\n}\n";

/// The git port a surface installs.
fn gix() -> SharedGit {
    Arc::new(specforge_git::Gix::default())
}

/// A project of `a.spec` committed as [`FIRST`], then as [`SECOND`]: the project, compiled with no extension and
/// the real git port, and the two commits.
fn two_commits() -> (TestProject, CompiledProject, String, String) {
    let project = TestProject::new().files(&[("a.spec", FIRST)]);
    let repo = Repo::init(project.root());
    let first = repo.commit_all("first");
    repo.write("a.spec", SECOND);
    let second = repo.commit_all("second");
    let project = project.with_git(gix());
    let compiled = project.compile_bare();
    (project, compiled, first, second)
}

/// The value `note` of node `id` in `graph`, as the source wrote it.
fn note(graph: &specforge_graph::Graph, id: &str) -> Option<String> {
    let node = graph.node(id)?;
    let entry = node
        .fields
        .entries()
        .iter()
        .find(|e| e.key.as_str() == "note")?;
    Some(format!("{entry:?}"))
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_revision_holds_the_graph_its_commit_wrote() {
    let (_project, compiled, first, _) = two_commits();
    let env = compiled.environment();
    let at = Snapshot::at_revision(env, &PlainDisk, &first).unwrap();
    assert_eq!(
        at.key,
        SnapshotKey::Revision {
            name: first.clone(),
            sha: specforge_effects::git::Sha::new(first.clone()),
        }
    );
    assert!(at.graph.node("x").is_some(), "x at the first commit");
    assert!(at.graph.node("y").is_some(), "y at the first commit");
    assert!(at.graph.node("z").is_none(), "z is not written yet");
    assert!(note(&at.graph, "x").unwrap().contains("first"));
    // the working tree holds the second commit's
    let now = Snapshot::of_project(&compiled);
    assert!(now.graph.node("y").is_none());
    assert!(now.graph.node("z").is_some());
    assert!(note(&now.graph, "x").unwrap().contains("second"));
    assert!(matches!(now.key, SnapshotKey::WorkingTree { .. }));
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_revision_by_name_resolves_to_its_commit() {
    let (_project, compiled, _, second) = two_commits();
    let at = Snapshot::at_revision(compiled.environment(), &PlainDisk, "HEAD").unwrap();
    let SnapshotKey::Revision { name, sha } = &at.key else {
        panic!("a revision: {:?}", at.key)
    };
    assert_eq!(name, "HEAD");
    assert_eq!(sha.as_str(), second);
    assert!(at.graph.node("z").is_some());
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_unknown_revision_is_refused() {
    let (_project, compiled, _, _) = two_commits();
    let env = compiled.environment();
    assert_eq!(
        Snapshot::at_revision(env, &PlainDisk, "no-such-rev").unwrap_err(),
        AtError::UnknownRevision("no-such-rev".to_string())
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_option_like_revision_is_an_unknown_revision() {
    let (_project, compiled, _, _) = two_commits();
    let env = compiled.environment();
    // the git port resolves it in process: an option is a name no commit has, never an argument to a program
    assert_eq!(
        Snapshot::at_revision(env, &PlainDisk, "--upload-pack=x").unwrap_err(),
        AtError::UnknownRevision("--upload-pack=x".to_string())
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_project_outside_a_repository_has_no_revision() {
    let project = TestProject::new()
        .files(&[("a.spec", FIRST)])
        .with_git(gix());
    let compiled = project.compile_bare();
    assert_eq!(
        Snapshot::at_revision(compiled.environment(), &PlainDisk, "HEAD").unwrap_err(),
        AtError::NotARepository
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn no_git_port_is_unreadable() {
    let project = TestProject::new().files(&[("a.spec", FIRST)]);
    Repo::init(project.root()).commit_all("c");
    let compiled = project.compile_bare();
    assert!(matches!(
        Snapshot::at_revision(compiled.environment(), &PlainDisk, "HEAD"),
        Err(AtError::Unreadable(_))
    ));
}

const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const STORE: &str = ".specforge/facts/0123456789abcdef0123456789abcdef01234567/node.json";

/// `node`'s facts: two tests of `a.test.js`.
fn two_tests() -> Value {
    json!([
        {"kind": "test", "id": "js:a.test.js#adds", "title": "adds", "fields": {"file": "a.test.js", "status": "passed"}},
        {"kind": "test", "id": "js:a.test.js#subtracts", "fields": {"file": "a.test.js", "status": "failed"}},
    ])
}

/// A project enabling the collected fixture (`node` writes `test` facts), whose store of [`two_tests`] under
/// [`KEY`] and the ledger line recording it are committed: the project, its runtime, the commit.
fn collected_at_a_commit() -> (
    TestProject,
    Arc<specforge_wasm::testing::InProcessRuntime>,
    String,
) {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file("a.spec", FIRST);
    stored(project.root(), "node", KEY, two_tests());
    let commit = Repo::init(project.root()).commit_all("collected");
    (
        project.with_git(gix()),
        Arc::new(Collected::new().runtime()),
        commit,
    )
}

/// How many `test` facts `snapshot`'s graph holds.
fn tests_in(snapshot: &Snapshot) -> usize {
    snapshot
        .graph
        .nodes()
        .iter()
        .filter(|n| n.kind.raw.as_str() == "test")
        .count()
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_revision_serves_the_store_its_ledger_records() {
    let (project, runtime, commit) = collected_at_a_commit();
    let compiled = project.compile_in(runtime);
    let at = Snapshot::at_revision(compiled.environment(), &PlainDisk, &commit).unwrap();
    assert_eq!(
        at.facts.get("node"),
        Some(&FactsAt::Served),
        "{:?}",
        at.facts
    );
    assert_eq!(tests_in(&at), 2);
    assert!(
        at.graph.node("x").is_some(),
        "the specs are served beside the facts"
    );
    // the working tree serves the same store
    let now = Snapshot::of_project(&compiled);
    assert_eq!(
        now.facts.get("node"),
        Some(&FactsAt::Served),
        "{:?}",
        now.facts
    );
    assert_eq!(tests_in(&now), 2);
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_store_changed_since_the_revision_is_not_served_there() {
    let (project, runtime, commit) = collected_at_a_commit();
    let edited = project.read(STORE).replacen("failed", "passed", 1);
    project.write(STORE, edited);
    let compiled = project.compile_in(runtime);
    let at = Snapshot::at_revision(compiled.environment(), &PlainDisk, &commit).unwrap();
    let Some(FactsAt::Unrecorded(w191)) = at.facts.get("node") else {
        panic!("unrecorded: {:?}", at.facts)
    };
    assert_eq!(w191.code, "W191", "{w191:?}");
    assert_eq!(tests_in(&at), 0);
    // the working tree says the same of it
    let now = Snapshot::of_project(&compiled);
    assert!(
        matches!(now.facts.get("node"), Some(FactsAt::Unrecorded(d)) if d.code == "W191"),
        "{:?}",
        now.facts
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_store_the_disk_no_longer_holds_was_not_collected() {
    let (project, runtime, commit) = collected_at_a_commit();
    project.remove(STORE);
    let compiled = project.compile_in(runtime);
    let at = Snapshot::at_revision(compiled.environment(), &PlainDisk, &commit).unwrap();
    assert_eq!(
        at.facts.get("node"),
        Some(&FactsAt::NotCollected),
        "{:?}",
        at.facts
    );
    assert_eq!(tests_in(&at), 0);
    let now = Snapshot::of_project(&compiled);
    assert_eq!(
        now.facts.get("node"),
        Some(&FactsAt::NotCollected),
        "{:?}",
        now.facts
    );
}

/// The plain disk, recording every path it reads.
#[derive(Default)]
struct Recording(Mutex<Vec<PathBuf>>);

impl Disk for Recording {
    fn exists(&self, path: &Path) -> bool {
        PlainDisk.exists(path)
    }
    fn list(&self, dir: &Path) -> Option<Vec<String>> {
        PlainDisk.list(dir)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.0.lock().unwrap().push(path.to_path_buf());
        PlainDisk.read(path)
    }
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_ledger_line_naming_no_store_is_never_made_a_path() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file("a.spec", FIRST);
    // a file each hostile line would reach, were it made a path
    project.write("x.json", "{}");
    project.write(".specforge/facts/c.json", "{}");
    project.write(&format!(".specforge/facts/{KEY}/.c.json"), "{}");
    let mut ledger = Ledger::default();
    for (collector, key) in [
        ("node", "../../x"),
        ("../c", KEY),
        (".c", KEY),
        ("node", ""),
    ] {
        ledger.insert(Line::facts(collector, key, b"{}"));
    }
    ledger.insert(Line::stale("node", "../../x", b"{}", "file", "a", "b"));
    project.write(LEDGER_FILE, ledger.canonical());
    let commit = Repo::init(project.root()).commit_all("hostile");
    let project = project.with_git(gix());
    let compiled = project.compile_in(Arc::new(Collected::new().runtime()));
    let disk = Recording::default();
    let at = Snapshot::at_revision(compiled.environment(), &disk, &commit).unwrap();
    let facts = project
        .root()
        .canonicalize()
        .unwrap()
        .join(".specforge/facts");
    let read = disk.0.lock().unwrap().clone();
    let outside: Vec<&PathBuf> = read
        .iter()
        .filter(|p| {
            !p.starts_with(&facts) && !p.starts_with(project.root().join(".specforge/facts"))
        })
        .collect();
    assert!(outside.is_empty(), "read outside the stores: {outside:?}");
    assert!(
        read.is_empty(),
        "no line names a store, so nothing is read: {read:?}"
    );
    // the ledger at the revision held every hostile line, and each was refused; `unit` is loaded and named by none
    assert_eq!(
        at.facts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["../c", ".c", "node", "unit"]
    );
    for (collector, at) in &at.facts {
        assert_eq!(at, &FactsAt::NotCollected, "{collector}");
    }
    assert_eq!(tests_in(&at), 0);
}

#[specforge_test(requirement = "FEATURE-142")]
fn loading_at_a_revision_runs_no_collector_and_no_program() {
    let (project, runtime, commit) = collected_at_a_commit();
    let process = Arc::new(FakeProcess::new());
    let ports = specforge_effects::Ports {
        process: process.clone(),
        ..specforge_effects::testing::ports()
    };
    let env = project.environment_in(runtime.clone());
    let effects = specforge_project::effects::of(
        EffectsMode::Online,
        &ports,
        &env,
        &project.project_root(),
        &MemoryConsent::default(),
    );
    let env = env.with_effects(effects).with_git(gix());
    let before = runtime.calls().len();
    let at = Snapshot::at_revision(&env, &PlainDisk, &commit).unwrap();
    assert_eq!(at.facts.get("node"), Some(&FactsAt::Served));
    let called: Vec<String> = runtime.calls()[before..]
        .iter()
        .map(|c| c.export.clone())
        .collect();
    assert!(called.is_empty(), "the extensions were called: {called:?}");
    assert!(process.requests().is_empty(), "{:?}", process.requests());
}

/// The facts of a collector one side serves and the other did not collect are no change: the side that lacks them
/// is named in a note, and the specs still compare.
#[specforge_test(requirement = "FEATURE-142")]
fn a_collector_one_side_lacks_is_a_note_and_no_change() {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&COLLECTED_EXTENSIONS)
        .installing_enabled()
        .file("a.spec", FIRST);
    let repo = Repo::init(project.root());
    let bare = repo.commit_all("no facts");
    project.write("a.spec", SECOND);
    stored(project.root(), "node", KEY, two_tests());
    let collected = repo.commit_all("collected");
    let project = project.with_git(gix());
    let compiled = project.compile_in(Arc::new(Collected::new().runtime()));
    let env = compiled.environment();
    let from = Snapshot::at_revision(env, &PlainDisk, &bare).unwrap();
    let to = Snapshot::at_revision(env, &PlainDisk, &collected).unwrap();
    assert_eq!(tests_in(&to), 2);
    let forms = env.registries().field_forms();
    let Comparison { delta, notes } = compare(&from, &to, forms, DeltaBase::None);
    let changed: Vec<(&str, &str)> = delta
        .payload()
        .nodes
        .iter()
        .map(|n| (n.kind.as_str(), n.id.as_str()))
        .collect();
    assert_eq!(changed, [("thing", "x"), ("thing", "y"), ("thing", "z")]);
    let [note] = notes.as_slice() else {
        panic!("{notes:?}")
    };
    assert_eq!(
        (note.side, note.about, note.subject.as_str()),
        (Side::From, NoteAbout::FactsNotCollected, "node")
    );
    assert_eq!(note.message, format!("facts not collected at {bare}"));
    // the other way round, the note is on the other side
    let back = compare(&to, &from, forms, DeltaBase::None);
    assert_eq!(back.notes.len(), 1);
    assert_eq!(back.notes[0].side, Side::To);
    assert!(back.delta.payload().nodes.iter().all(|n| n.kind == "thing"));
}

/// A store both sides hold and neither serves (W191 at the revision and in the working tree) is a note on each side,
/// carrying its diagnostic; served on both, a collector needs none.
#[specforge_test(requirement = "FEATURE-142")]
fn an_unrecorded_store_is_a_note_on_each_side_that_holds_it() {
    let (project, runtime, commit) = collected_at_a_commit();
    let compiled = project.compile_in(runtime.clone());
    let forms = compiled.environment().registries().field_forms();
    let at = Snapshot::at_revision(compiled.environment(), &PlainDisk, &commit).unwrap();
    let served = compare(
        &at,
        &Snapshot::of_project(&compiled),
        forms,
        DeltaBase::None,
    );
    assert!(served.notes.is_empty(), "{:?}", served.notes);
    assert!(served.delta.is_empty(), "{:?}", served.delta.payload());

    let edited = project.read(STORE).replacen("failed", "passed", 1);
    project.write(STORE, edited);
    let compiled = project.compile_in(runtime);
    let at = Snapshot::at_revision(compiled.environment(), &PlainDisk, &commit).unwrap();
    let now = Snapshot::of_project(&compiled);
    let Comparison { notes, .. } = compare(&at, &now, forms, DeltaBase::None);
    let sides: Vec<(Side, NoteAbout)> = notes.iter().map(|n| (n.side, n.about)).collect();
    assert_eq!(
        sides,
        [
            (Side::From, NoteAbout::FactsUnrecorded),
            (Side::To, NoteAbout::FactsUnrecorded)
        ]
    );
    assert!(
        notes.iter().all(|n| n.message.starts_with("fact store '")),
        "{notes:?}"
    );
}
