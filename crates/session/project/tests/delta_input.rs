//! What a pass that declares the `delta` input is sent (ADR 0376 D6, D7). The guest is `@test/delta`: the kinds `item`
//! (whose `refs` are edges to a `note`, and whose `size` is computed) and `note`; check passes that declare the delta
//! of `item` (`items`, `twin`), of `note` (`notes`), of no kind (`empty`), or nothing (`plain`); the compute pass
//! `counted` and the guard pass `guarded` (reporting `E901`), both declaring the delta of `item`. Each payload is read
//! from the inputs the runtime recorded.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_effects::git::{Git, Located, Sha};
use specforge_effects::testing::FakeGit;
use specforge_extension_sdk::prelude::*;
use specforge_graph::Graph;
use specforge_project::CompiledProject;
use specforge_project::baseline::{Baseline, Checking};
use specforge_project::delta::Delta;
use specforge_project::testing::{TestProject, pass_input};
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@test/delta";

/// Three items, two notes, and one edge: `a` refers to `n1`.
const SPEC: &str = "item a \"A\" {\n  refs [n1]\n}\n\nitem b \"B\" {\n}\n\nitem c \"C\" {\n}\n\n\
                    note n1 \"N1\" {\n}\n\nnote n2 \"N2\" {\n}\n";

fn nothing(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

/// `@test/delta`'s kinds and its passes.
fn delta_extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.kind("item", |k| {
        k.field("refs", |f| {
            f.field_type(FieldType::ReferenceList)
                .edge("Refs")
                .target_kind("note");
        });
        k.field("size", |f| {
            f.field_type(FieldType::Integer).computed();
        });
    });
    c.kind("note", |_| {});
    c.edge("Refs", |e| {
        e.source_kind("item").target_kind("note");
    });
    for (name, kinds) in [
        ("items", &["item"][..]),
        ("twin", &["item"][..]),
        ("notes", &["note"][..]),
        ("empty", &[][..]),
    ] {
        c.pass(name, |p| {
            p.phase("check").input(Input::delta(kinds)).run(nothing);
        });
    }
    c.pass("plain", |p| {
        p.phase("check").run(nothing);
    });
    c.pass("counted", |p| {
        p.compute()
            .computes(&["item.size"])
            .input(Input::delta(&["item"]))
            .run(nothing);
    });
    c.pass("guarded", |p| {
        p.guard()
            .reports(&["E901"])
            .input(Input::delta(&["item"]))
            .run(nothing);
    });
    c
}

/// A project of `files` with `@test/delta` enabled.
fn project(files: &[(&str, &str)]) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .files(files)
}

/// The `@test/delta` runtime.
fn runtime() -> Arc<InProcessRuntime> {
    Arc::new(InProcessRuntime::new().with(delta_extension))
}

/// `files` compiled cold with `@test/delta` loaded, and the runtime it compiled in.
fn compiled(files: &[(&str, &str)]) -> (CompiledProject, Arc<InProcessRuntime>) {
    let runtime = runtime();
    let compiled = project(files).compile_in(runtime.clone());
    (compiled, runtime)
}

/// The delta `pass` was sent.
fn delta_of(runtime: &InProcessRuntime, pass: &str) -> Value {
    pass_input(runtime, pass)["delta"].clone()
}

/// The delta from the empty graph to `compiled`'s, restricted to `kinds`, as the wire carries it.
fn cold(compiled: &CompiledProject, kinds: &[&str]) -> Value {
    let delta = Delta::between(
        &Graph::default(),
        compiled.graph(),
        compiled.environment().registries().field_forms(),
        DeltaBase::None,
    );
    serde_json::to_value(delta.restricted_to(kinds)).unwrap()
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_cold_compile_sends_every_node_of_the_named_kinds_added() {
    let (compiled, runtime) = compiled(&[("a.spec", SPEC)]);
    let delta = delta_of(&runtime, "items");
    assert_eq!(delta["base"], json!({"kind": "none"}));
    let nodes = delta["nodes"].as_array().unwrap();
    let named: Vec<(&str, &str, &str)> = nodes
        .iter()
        .map(|n| {
            (
                n["kind"].as_str().unwrap(),
                n["id"].as_str().unwrap(),
                n["change"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("item", "a", "added"),
            ("item", "b", "added"),
            ("item", "c", "added")
        ]
    );
    for node in nodes {
        assert!(
            node["after"].as_str().is_some_and(|d| !d.is_empty()),
            "{node}"
        );
        assert!(node.get("before").is_none(), "{node}");
    }
    let edges = delta["added_edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1, "{delta}");
    assert_eq!(
        (&edges[0]["source"], &edges[0]["target"]),
        (&json!("a"), &json!("n1"))
    );
    assert!(delta.get("removed_edges").is_none(), "{delta}");
    // Exactly the delta the host takes of the graph, digests included.
    assert_eq!(delta, cold(&compiled, &["item"]));
    let held = compiled.delta().expect("a loaded pass declares the delta");
    assert_eq!(
        serde_json::to_value(held.restricted_to(&["item"])).unwrap(),
        delta
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_pass_is_sent_no_node_of_a_kind_it_did_not_name() {
    let (compiled, runtime) = compiled(&[("a.spec", SPEC)]);
    let items = delta_of(&runtime, "items");
    assert!(
        items["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["kind"] == "item"),
        "{items}"
    );
    let notes = delta_of(&runtime, "notes");
    let ids: Vec<&str> = notes["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["n1", "n2"]);
    assert!(
        notes["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["kind"] == "note"),
        "{notes}"
    );
    // The edge has an end of each kind: both readers are sent it.
    assert_eq!(notes["added_edges"], items["added_edges"]);
    assert_eq!(notes, cold(&compiled, &["note"]));
}

#[specforge_test(requirement = "FEATURE-142")]
fn two_passes_of_the_same_kinds_are_sent_the_same_delta() {
    let (_, runtime) = compiled(&[("a.spec", SPEC)]);
    let items = serde_json::to_string(&delta_of(&runtime, "items")).unwrap();
    let twin = serde_json::to_string(&delta_of(&runtime, "twin")).unwrap();
    assert_eq!(items, twin);
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_pass_that_declares_no_delta_is_sent_no_delta_key() {
    let (_, runtime) = compiled(&[("a.spec", SPEC)]);
    assert!(pass_input(&runtime, "plain").get("delta").is_none());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_delta_input_naming_no_kind_is_sent_the_base_and_no_node() {
    let (_, runtime) = compiled(&[("a.spec", SPEC)]);
    assert_eq!(
        delta_of(&runtime, "empty"),
        json!({"base": {"kind": "none"}})
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_project_with_no_node_of_the_kind_sends_the_base_alone() {
    let (_, runtime) = compiled(&[("a.spec", "note n1 \"N1\" {\n}\n")]);
    assert_eq!(
        delta_of(&runtime, "items"),
        json!({"base": {"kind": "none"}})
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn no_delta_is_taken_when_no_pass_declares_it() {
    let project = project(&[("a.spec", SPEC)]);
    let runtime = Arc::new(InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.kind("item", |_| {});
        c.pass("plain", |p| {
            p.phase("check").run(nothing);
        });
        c
    }));
    let compiled = project.compile_in(runtime.clone());
    assert!(compiled.delta().is_none());
    assert!(pass_input(&runtime, "plain").get("delta").is_none());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_compute_pass_is_sent_the_delta_a_check_pass_is() {
    let (compiled, runtime) = compiled(&[("a.spec", SPEC)]);
    let counted = delta_of(&runtime, "counted");
    assert_eq!(counted, delta_of(&runtime, "items"));
    assert_eq!(counted, cold(&compiled, &["item"]));
}

#[specforge_test(requirement = "FEATURE-142")]
fn the_compute_at_the_baseline_is_sent_the_commits_graph_from_none() {
    let project = project(&[("a.spec", SPEC)]);
    let runtime = runtime();
    // The commit holds one item; the working tree three.
    let sha = "c".repeat(40);
    let at_commit = "item a \"A\" {\n}\n\nnote n1 \"N1\" {\n}\n";
    let fake = FakeGit::repo(
        project.root(),
        &sha,
        vec![("a.spec".into(), at_commit.as_bytes().to_vec())],
    );
    let env = project.environment_in(runtime.clone());
    let Located::Repo(place) = Git::locate(&fake, project.root()) else {
        panic!("a repository")
    };
    Baseline::at(&env, &fake, &place, &Sha::new(sha), &Checking::of(&env)).expect("a baseline");
    let delta = delta_of(&runtime, "counted");
    assert_eq!(delta["base"], json!({"kind": "none"}));
    let ids: Vec<&str> = delta["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["a"], "{delta}");
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_guard_pass_is_sent_what_the_write_changed_against_the_project_before_it() {
    let project = project(&[("a.spec", SPEC)]);
    let runtime = runtime();
    let compiled = project.compile_in(runtime.clone());
    let edited = compiled
        .edited(&[("a.spec".to_string(), SPEC.replace("\"B\"", "\"Bee\""))].into())
        .judged_against(&compiled);
    drop(edited);
    let delta = delta_of(&runtime, "guarded");
    assert_eq!(delta["base"], json!({"kind": "previous"}));
    let nodes = delta["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1, "{delta}");
    assert_eq!(
        (&nodes[0]["kind"], &nodes[0]["id"], &nodes[0]["change"]),
        (&json!("item"), &json!("b"), &json!("modified"))
    );
    assert_eq!(nodes[0]["aspects"], json!(["title"]), "{delta}");
    assert_ne!(nodes[0]["before"], nodes[0]["after"]);
    assert!(delta.get("added_edges").is_none(), "{delta}");
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_guard_pass_a_gate_runs_is_sent_the_compiles_delta() {
    let project = project(&[("a.spec", SPEC)]);
    let runtime = runtime();
    let compiled = project.compile_in(runtime.clone());
    let code = specforge_diagnostics::qualify(EXT, "E901");
    compiled.gated_against_self(&[code], &Trigger::gate("ship", None));
    assert_eq!(delta_of(&runtime, "guarded"), cold(&compiled, &["item"]));
}

/// Two items and a note at the merge-base.
const AT_BASE: &str = "item x \"X\" {\n}\n\nitem y \"Y\" {\n}\n\nnote n1 \"N1\" {\n}\n";
/// At `HEAD`: x retitled, z and n2 added.
const AT_HEAD: &str = "item x \"Ex\" {\n}\n\nitem y \"Y\" {\n}\n\nitem z \"Z\" {\n}\n\n\
                       note n1 \"N1\" {\n}\n\nnote n2 \"N2\" {\n}\n";

/// A repository whose `main` commits [`AT_BASE`] then [`AT_HEAD`] (the working tree, clean), and whose branch `side`
/// leaves the first commit with a commit of its own: the project with the real git port, and the first commit's sha
/// (the merge-base of `side` and `HEAD`) and `HEAD`'s.
fn forked() -> (TestProject, String, String) {
    let project = project(&[("a.spec", AT_BASE)]);
    let repo = specforge_git::testing::Repo::init(project.root());
    let base = repo.commit_all("base");
    repo.write("a.spec", AT_HEAD);
    let head = repo.commit_all("head");
    repo.branch("side", &base).head_on("side");
    repo.write("b.spec", "note s \"S\" {\n}\n");
    repo.commit_all("side");
    repo.head_on("main");
    repo.delete("b.spec");
    let git: specforge_effects::SharedGit = Arc::new(specforge_git::Gix::default());
    (project.with_git(git), base, head)
}

/// How many times `pass` was called.
fn calls(runtime: &InProcessRuntime, pass: &str) -> usize {
    let export = format!("__pass_{pass}");
    runtime
        .calls()
        .iter()
        .filter(|c| c.export == export)
        .count()
}

/// The kind, id and change of each node of `delta`.
fn changes(delta: &Value) -> Vec<(String, String, String)> {
    delta["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| {
            let text = |key: &str| n[key].as_str().unwrap().to_string();
            (text("kind"), text("id"), text("change"))
        })
        .collect()
}

#[specforge_test(requirement = "FEATURE-142")]
fn since_names_the_merge_base() {
    let (project, base, _) = forked();
    let runtime = runtime();
    let compiled = CompiledProject::of_since(project.environment_in(runtime.clone()), "side")
        .expect("side is a revision");
    let items = delta_of(&runtime, "items");
    // the merge-base by its full sha, never the revision as named nor its own commit
    assert_eq!(items["base"], json!({"kind": "revision", "id": base}));
    assert_eq!(base.len(), 40);
    assert_eq!(
        compiled.delta_base(),
        DeltaBase::Revision { id: base.clone() }
    );
    let named = |kind: &str, id: &str, change: &str| (kind.into(), id.into(), change.into());
    assert_eq!(
        changes(&items),
        [named("item", "x", "modified"), named("item", "z", "added")]
    );
    assert_eq!(
        changes(&delta_of(&runtime, "notes")),
        [named("note", "n2", "added")]
    );
    assert_eq!(delta_of(&runtime, "counted"), items);
}

#[specforge_test(requirement = "FEATURE-142")]
fn since_head_is_empty() {
    let (project, _, head) = forked();
    let runtime = runtime();
    CompiledProject::of_since(project.environment_in(runtime.clone()), "HEAD").unwrap();
    assert_eq!(
        delta_of(&runtime, "items"),
        json!({"base": {"kind": "revision", "id": head}})
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn since_an_unknown_revision_checks_nothing() {
    let (project, _, _) = forked();
    let runtime = runtime();
    let refused = CompiledProject::of_since(project.environment_in(runtime.clone()), "no-such-rev");
    assert_eq!(
        refused.err(),
        Some(specforge_project::baseline::UnknownRevision(
            "no-such-rev".to_string()
        ))
    );
    assert_eq!(calls(&runtime, "items"), 0, "no pass runs");
}

#[specforge_test(requirement = "FEATURE-142")]
fn since_runs_each_pass_once() {
    let (project, base, _) = forked();
    let since = runtime();
    let compiled =
        CompiledProject::of_since(project.environment_in(since.clone()), "side").unwrap();
    for pass in ["items", "notes", "plain", "counted"] {
        assert_eq!(calls(&since, pass), 1, "{pass}");
    }
    // judged since the revision it was checked since: no pass runs again
    let judged = compiled.judged_since("side").unwrap();
    assert_eq!(calls(&since, "items"), 1);
    assert_eq!(judged.delta_base(), compiled.delta_base());
    // a compile with no base, judged since the revision: each pass once more, against the merge-base
    let runtime = runtime();
    let cold = CompiledProject::of(project.environment_in(runtime.clone()));
    assert_eq!(delta_of(&runtime, "items")["base"], json!({"kind": "none"}));
    let judged = cold.judged_since("side").unwrap();
    assert_eq!(calls(&runtime, "items"), 2);
    assert_eq!(calls(&runtime, "counted"), 2);
    assert_eq!(
        delta_of(&runtime, "items")["base"],
        json!({"kind": "revision", "id": base})
    );
    assert_eq!(
        cold.delta_base(),
        DeltaBase::None,
        "the compile keeps its own"
    );
    assert_eq!(judged.delta_base(), DeltaBase::Revision { id: base });
}

#[specforge_test(requirement = "FEATURE-142")]
fn since_loads_no_graph_at_the_merge_base_when_no_pass_reads_the_delta() {
    let project = project(&[("a.spec", AT_HEAD)]);
    let sha = "c".repeat(40);
    let fake = |files: &str| {
        let mut git = FakeGit::repo(
            project.root(),
            &sha,
            vec![("a.spec".into(), files.as_bytes().to_vec())],
        );
        git.named
            .push(("side".into(), Sha::new("d".repeat(40)), Vec::new()));
        Arc::new(git)
    };
    let plain = Arc::new(InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.kind("item", |_| {});
        c.kind("note", |_| {});
        c.pass("plain", |p| {
            p.phase("check").run(nothing);
        });
        c
    }));
    let git = fake(AT_BASE);
    let env = project.environment_in(plain).with_git(git.clone());
    let compiled = CompiledProject::of_since(env, "side").unwrap();
    assert_eq!(git.files_asked(), 0, "nothing reads history or the delta");
    assert_eq!(compiled.delta_base(), DeltaBase::None);
    // the same project with a pass that reads the delta: the merge-base is loaded once
    let git = fake(AT_BASE);
    let env = project.environment_in(runtime()).with_git(git.clone());
    let compiled = CompiledProject::of_since(env, "side").unwrap();
    assert_eq!(git.files_asked(), 1);
    assert!(matches!(compiled.delta_base(), DeltaBase::Revision { .. }));
}

/// Whether every node `delta` names is of `kind`.
fn only_of(delta: &Value, kind: &str) -> bool {
    changes(delta).iter().all(|(of, ..)| of == kind)
}

/// A session of [`SPEC`] with `@test/delta` loaded, and the runtime it runs in.
fn session() -> (
    TestProject,
    specforge_project::ProjectSession,
    Arc<InProcessRuntime>,
) {
    let project = project(&[("a.spec", SPEC)]);
    let runtime = runtime();
    let session = project.session_in(runtime.clone());
    (project, session, runtime)
}

/// The editor's buffer of `a.spec` holding `text`.
fn buffer(project: &TestProject, text: &str) -> specforge_project::Buffer {
    specforge_project::Buffer::new(project.root().join("a.spec"), text)
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_session_sends_the_changes_since_its_last_check() {
    let (project, mut session, runtime) = session();
    // the first check has no base: every item added
    let first = delta_of(&runtime, "items");
    assert_eq!(first, cold(session.project(), &["item"]));
    assert_eq!(session.project().delta_base(), DeltaBase::Previous);
    // an edit of b: b alone, against the last check
    session.hold(&[buffer(&project, &SPEC.replace("\"B\"", "\"Bee\""))]);
    let items = delta_of(&runtime, "items");
    assert_eq!(items["base"], json!({"kind": "previous"}));
    assert_eq!(
        changes(&items),
        [("item".to_string(), "b".to_string(), "modified".to_string())]
    );
    assert!(items.get("added_edges").is_none(), "{items}");
    assert_eq!(
        delta_of(&runtime, "notes"),
        json!({"base": {"kind": "previous"}})
    );
    for pass in ["items", "counted"] {
        assert!(only_of(&delta_of(&runtime, pass), "item"), "{pass}");
    }
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_unchanged_recheck_sends_an_empty_delta() {
    let (_project, mut session, runtime) = session();
    let before = calls(&runtime, "items");
    session.recheck();
    assert_eq!(calls(&runtime, "items"), before + 1);
    // the key is there, with its base and no node
    let input = pass_input(&runtime, "items");
    assert_eq!(input["delta"], json!({"base": {"kind": "previous"}}));
    assert!(session.project().delta().is_some_and(|d| d.is_empty()));
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_skipped_check_leaves_the_base_at_the_last_completed_check() {
    let (project, mut session, runtime) = session();
    // typing: b retitled with a parse error, so the checks are skipped
    let broken = SPEC
        .replace("\"B\"", "\"Bee\"")
        .replace("item c", "item c {");
    session.hold_with(
        &[buffer(&project, &broken)],
        specforge_project::CheckMode::SyntaxOnlyIfParseErrors,
    );
    assert!(
        session.project().delta().is_none(),
        "the checks were skipped"
    );
    assert_eq!(session.project().delta_base(), DeltaBase::Previous);
    // then c retitled too, and the checks run: both edits, against the last completed check
    let fixed = SPEC.replace("\"B\"", "\"Bee\"").replace("\"C\"", "\"Sea\"");
    session.hold_with(
        &[buffer(&project, &fixed)],
        specforge_project::CheckMode::SyntaxOnlyIfParseErrors,
    );
    let ids: Vec<String> = changes(&delta_of(&runtime, "items"))
        .into_iter()
        .map(|(_, id, change)| format!("{id} {change}"))
        .collect();
    assert_eq!(ids, ["b modified", "c modified"]);
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_reloaded_environment_starts_again_with_no_base() {
    let (project, mut session, runtime) = session();
    session.hold(&[buffer(&project, &SPEC.replace("\"B\"", "\"Bee\""))]);
    assert_eq!(
        delta_of(&runtime, "items")["base"],
        json!({"kind": "previous"})
    );
    session.reload_environment();
    // the reload's check: every item added again, base none
    let items = delta_of(&runtime, "items");
    assert_eq!(items, cold(session.project(), &["item"]));
    assert_eq!(items["base"], json!({"kind": "none"}));
    assert!(only_of(&items, "item"));
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_session_with_no_delta_pass_holds_no_graph() {
    let project = project(&[("a.spec", SPEC)]);
    let runtime = Arc::new(InProcessRuntime::new().with(|| {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.kind("item", |_| {});
        c.kind("note", |_| {});
        c.pass("plain", |p| {
            p.phase("check").run(nothing);
        });
        c
    }));
    let mut session = project.session_in(runtime.clone());
    assert_eq!(session.project().delta_base(), DeltaBase::None);
    session.hold(&[buffer(&project, &SPEC.replace("\"B\"", "\"Bee\""))]);
    assert_eq!(session.project().delta_base(), DeltaBase::None);
    session.recheck();
    assert_eq!(session.project().delta_base(), DeltaBase::None);
    assert!(session.project().delta().is_none());
    assert!(pass_input(&runtime, "plain").get("delta").is_none());
}
