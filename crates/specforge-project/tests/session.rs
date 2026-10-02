use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_project::{CompiledProject, ProjectSession, SourceChange};
use specforge_test::prelude::*;
use tempfile::TempDir;

const CONFIG: &str =
    r#"{"name":"s","version":"0.1.0","extensions":["@specforge/software","@specforge/testing"]}"#;

fn project(config: &str, files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), config).unwrap();
    for (path, text) in files {
        write(dir.path(), path, text);
    }
    dir
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Diagnostics as a multiset of their full JSON (code, severity, message,
/// span, suggestion): order-independent, nothing else dropped.
fn diagnostic_set(diagnostics: &[Diagnostic]) -> BTreeMap<String, usize> {
    let mut set = BTreeMap::new();
    for d in diagnostics {
        *set.entry(serde_json::to_string(d).unwrap()).or_default() += 1;
    }
    set
}

/// Every node (id, kind, file, title, fields) and edge of a graph.
fn graph_contents(graph: &Graph) -> (Vec<String>, Vec<String>) {
    let mut nodes: Vec<String> = graph
        .nodes()
        .iter()
        .map(|n| {
            format!(
                "{} {} {} {:?} {}",
                n.id.raw,
                n.kind.raw,
                n.source_span.file,
                n.title,
                serde_json::to_string(&n.fields).unwrap()
            )
        })
        .collect();
    nodes.sort();
    let mut edges: Vec<String> = graph
        .edges()
        .iter()
        .map(|e| format!("{} -{}-> {}", e.source, e.label, e.target))
        .collect();
    edges.sort();
    (nodes, edges)
}

/// The session agrees with a fresh compile of what is on disk now.
fn assert_matches_a_fresh_compile(session: &ProjectSession, root: &Path) {
    let runtime = specforge_component::project_runtime(root);
    let fresh = CompiledProject::compile(root, Some(&runtime));
    assert_eq!(
        graph_contents(session.graph()),
        graph_contents(&fresh.graph)
    );
    assert_eq!(
        diagnostic_set(&session.diagnostics()),
        diagnostic_set(&fresh.diagnostics())
    );
}

fn w113(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|d| d.code == "W113")
        .map(|d| d.message.clone())
        .collect()
}

fn changed(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|p| p.to_string()).collect()
}

/// Edits that add and remove imports, a duplicate ID that moves between
/// files, a missing import and a deleted file: after each, the session's
/// graph and diagnostics are those of a fresh compile.
#[specforge_test(
    invariant = "incremental_correctness",
    verify = "incremental recompilation produces the same graph as a full rebuild"
)]
fn every_update_leaves_what_a_fresh_compile_builds() {
    let dir = project(
        CONFIG,
        &[
            (
                "a.spec",
                "behavior alpha \"A\" {\n  category command\n  contract \"The system MUST a\"\n  verify unit \"a\"\n}\n",
            ),
            (
                "b.spec",
                "behavior beta \"B\" {\n  category command\n  contract \"The system MUST b\"\n  verify unit \"b\"\n}\n",
            ),
        ],
    );
    let root = dir.path();
    let mut session = ProjectSession::open(root);
    session.set_verify_incremental(true);
    assert_matches_a_fresh_compile(&session, root);

    let steps: &[(&str, Option<&str>)] = &[
        // An import that does not resolve (E025).
        (
            "a.spec",
            Some(
                "use \"missing\"\n\nbehavior alpha \"A\" {\n  category command\n  contract \"The system MUST a\"\n  verify unit \"a\"\n}\n",
            ),
        ),
        // A cycle a -> b -> a (W113) and a duplicate of alpha in b.spec.
        (
            "b.spec",
            Some(
                "use \"a\"\n\nbehavior alpha \"Dup\" {\n  category command\n  contract \"The system MUST dup\"\n}\nbehavior beta \"B\" {\n  category command\n  contract \"The system MUST b\"\n  verify unit \"b\"\n}\n",
            ),
        ),
        (
            "a.spec",
            Some(
                "use \"b\"\n\nbehavior alpha \"A\" {\n  category command\n  verify unit \"a\"\n}\n",
            ),
        ),
        // The first declaration goes away: b.spec's alpha takes over.
        ("a.spec", None),
        (
            "c.spec",
            Some(
                "behavior gamma \"C\" {\n  category command\n  contract \"The system MUST c\"\n}\n",
            ),
        ),
    ];
    for (path, text) in steps {
        match text {
            Some(text) => write(root, path, text),
            None => fs::remove_file(root.join(path)).unwrap(),
        }
        let update = session.update(SourceChange::Disk(&changed(&[path])));
        assert_eq!(update.verification, Some(Ok(())), "after {path}");
        assert_eq!(
            diagnostic_set(&update.diagnostics),
            diagnostic_set(&session.diagnostics())
        );
        assert_matches_a_fresh_compile(&session, root);
    }
}

/// Define blocks are reported, not registered: adding one, renaming it
/// and removing it leave what a fresh compile reports.
#[specforge_test(
    behavior = "report_define_blocks",
    verify = "an incremental rebuild reports a define block as a fresh compile does"
)]
fn an_incremental_rebuild_reports_define_blocks_as_a_fresh_compile() {
    let dir = project(
        CONFIG,
        &[(
            "a.spec",
            "behavior alpha \"A\" {\n  category command\n  contract \"The system MUST a\"\n}\n",
        )],
    );
    let root = dir.path();
    let mut session = ProjectSession::open(root);
    session.set_verify_incremental(true);

    for text in [
        Some("define user_story {\n  refs [alpha]\n}\n"),
        Some("define behavior {\n}\n"),
        None,
    ] {
        match text {
            Some(text) => write(root, "b.spec", text),
            None => fs::remove_file(root.join("b.spec")).unwrap(),
        }
        let update = session.update(SourceChange::Disk(&changed(&["b.spec"])));
        assert_eq!(update.verification, Some(Ok(())));
        assert_matches_a_fresh_compile(&session, root);
        let w143 = session
            .diagnostics()
            .iter()
            .filter(|d| d.code == "W143")
            .count();
        assert_eq!(w143, usize::from(text.is_some()), "{text:?}");
    }
}

#[specforge_test(
    behavior = "track_import_dag_incrementally",
    verify = "cycle detection re-runs after import DAG update"
)]
fn an_edit_that_closes_an_import_cycle_reports_it() {
    let dir = project(
        CONFIG,
        &[
            ("a.spec", "behavior foo \"Foo\" { contract \"x\" }\n"),
            (
                "b.spec",
                "use \"a\"\nbehavior bar \"Bar\" { contract \"y\" }\n",
            ),
        ],
    );
    let root = dir.path();
    let mut session = ProjectSession::open(root);
    assert!(w113(&session.diagnostics()).is_empty());

    write(
        root,
        "a.spec",
        "use \"b\"\nbehavior foo \"Foo\" { contract \"x\" }\n",
    );
    let update = session.update(SourceChange::Disk(&changed(&["a.spec"])));

    assert_eq!(
        w113(&update.diagnostics),
        ["circular import detected: a.spec -> b.spec"]
    );
}

#[specforge_test(
    behavior = "track_import_dag_incrementally",
    verify = "cycle detection re-runs after import DAG update"
)]
fn an_edit_that_breaks_an_import_cycle_clears_it() {
    let dir = project(
        CONFIG,
        &[
            (
                "a.spec",
                "use \"b\"\nbehavior foo \"Foo\" { contract \"x\" }\n",
            ),
            (
                "b.spec",
                "use \"a\"\nbehavior bar \"Bar\" { contract \"y\" }\n",
            ),
        ],
    );
    let root = dir.path();
    let mut session = ProjectSession::open(root);
    assert_eq!(
        w113(&session.diagnostics()),
        ["circular import detected: a.spec -> b.spec"]
    );

    write(root, "a.spec", "behavior foo \"Foo\" { contract \"x\" }\n");
    let update = session.update(SourceChange::Disk(&changed(&["a.spec"])));

    assert!(
        w113(&update.diagnostics).is_empty(),
        "{:?}",
        update.diagnostics
    );
}

// B:track_import_dag_incrementally — verify contract "requires/ensures consistency for incremental import DAG tracking"
#[specforge_test(
    behavior = "track_import_dag_incrementally",
    verify = "Track Import DAG Incrementally: incremental import DAG tracking holds — subgraph_invalidated_fired, import_dag_updated_emitted, cycle_detection_rerun"
)]
fn track_import_dag_incrementally_contract() {
    let dir = project(
        CONFIG,
        &[
            ("a.spec", "behavior foo \"Foo\" { contract \"x\" }\n"),
            ("b.spec", "behavior bar \"Bar\" { contract \"y\" }\n"),
            ("c.spec", "behavior qux \"Qux\" { contract \"z\" }\n"),
        ],
    );
    let root = dir.path();
    let mut session = ProjectSession::open(root);
    assert!(
        session
            .pipeline()
            .import_dag()
            .imports_of("b.spec")
            .is_empty()
    );

    // import_dag_updated_emitted: an added `use` becomes a DAG edge.
    write(
        root,
        "b.spec",
        "use \"a\"\nbehavior bar \"Bar\" { contract \"y\" }\n",
    );
    let update = session.update(SourceChange::Disk(&changed(&["b.spec"])));
    assert_eq!(
        session.pipeline().import_dag().imports_of("b.spec"),
        vec!["a.spec"]
    );
    assert!(w113(&update.diagnostics).is_empty(), "no cycle yet");

    // subgraph_invalidated_fired: editing a.spec invalidates its importer
    // b.spec too (and not the unrelated c.spec). The edit closes a cycle
    // a -> b -> a; cycle_detection_rerun: W113 appears on this update.
    write(
        root,
        "a.spec",
        "use \"b\"\nbehavior foo \"Foo\" { contract \"x\" }\n",
    );
    let update = session.update(SourceChange::Disk(&changed(&["a.spec"])));
    assert_eq!(update.rebuilt_files, vec!["a.spec", "b.spec"]);
    assert_eq!(
        session.pipeline().import_dag().imports_of("a.spec"),
        vec!["b.spec"]
    );
    assert_eq!(
        w113(&update.diagnostics),
        ["circular import detected: a.spec -> b.spec"]
    );

    // Removing the import deletes the edge and the re-run clears the cycle.
    write(root, "b.spec", "behavior bar \"Bar\" { contract \"y\" }\n");
    let update = session.update(SourceChange::Disk(&changed(&["b.spec"])));
    assert!(
        session
            .pipeline()
            .import_dag()
            .imports_of("b.spec")
            .is_empty()
    );
    assert!(w113(&update.diagnostics).is_empty(), "cycle must be gone");
}

/// `exclude` is relative to the spec root, and a change to an excluded
/// file leaves the session as it was.
#[specforge_test(
    behavior = "resolve_use_imports",
    verify = "files matching an exclude entry are not compiled"
)]
fn excluded_files_stay_out_of_the_compile_and_the_session() {
    let dir = project(
        r#"{"name":"s","version":"0.1.0","extensions":["@specforge/software"],"spec_root":"spec","exclude":["drafts/"]}"#,
        &[
            ("spec/main.spec", "term alpha \"Alpha\" {\n}\n"),
            (
                "spec/drafts/draft.spec",
                "term alpha \"Alpha again\" {\n}\n",
            ),
        ],
    );
    let root = dir.path();
    let runtime = specforge_component::project_runtime(root);
    let compiled = CompiledProject::compile(root, Some(&runtime));
    let files: Vec<&str> = compiled
        .resolved
        .files
        .iter()
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(files, ["main.spec"]);
    assert!(
        !compiled.diagnostics().iter().any(|d| d.code == "E002"),
        "{:?}",
        compiled.diagnostics()
    );

    let mut session = ProjectSession::open(root);
    write(root, "spec/drafts/draft.spec", "term beta \"Beta\" {\n}\n");
    let update = session.update(SourceChange::Disk(&changed(&["drafts/draft.spec"])));
    assert!(
        update.rebuilt_files.is_empty(),
        "{:?}",
        update.rebuilt_files
    );
    assert!(session.graph().node("beta").is_none());
    assert_eq!(session.file_count(), 1);
}

/// A reload picks up a changed `specforge.json`: here an extension that
/// can't load (E028), which a fresh compile reports too.
#[test]
fn a_reload_reads_the_environment_again() {
    let dir = project(CONFIG, &[("a.spec", "term alpha \"Alpha\" {\n}\n")]);
    let root = dir.path();
    let mut session = ProjectSession::open(root);
    assert!(!session.diagnostics().iter().any(|d| d.code == "E028"));

    fs::write(
        root.join("specforge.json"),
        r#"{"name":"s","version":"0.1.0","extensions":["@specforge/software","@specforge/no-such-extension"]}"#,
    )
    .unwrap();
    let update = session.reload_environment();

    assert!(update.diagnostics.iter().any(|d| d.code == "E028"));
    assert_matches_a_fresh_compile(&session, root);
}

/// A session over a graph built in memory serves that graph and the
/// diagnostics given for it, in its environment, with nothing to reload.
#[test]
fn a_session_from_a_graph_serves_it_as_given() {
    let dir = project(CONFIG, &[("a.spec", "term alpha \"Alpha\" {\n}\n")]);
    let compiled = CompiledProject::compile(dir.path(), None);
    let built = compiled.graph.clone();
    let warning = Diagnostic::warning("W001", "given");

    let mut session = ProjectSession::from_graph(
        std::sync::Arc::new(specforge_project::Environment::empty()),
        built,
        vec![warning.clone()],
    );

    assert!(session.is_detached());
    assert_eq!(
        graph_contents(session.graph()),
        graph_contents(&compiled.graph)
    );
    assert_eq!(session.diagnostics(), vec![warning.clone()]);
    let update = session.reload_environment();
    assert!(
        update.delta.added_nodes.is_empty() && update.delta.removed_nodes.is_empty(),
        "nothing on disk to reload"
    );
    assert_eq!(session.diagnostics(), vec![warning]);
}
