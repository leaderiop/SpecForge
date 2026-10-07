use specforge_test_macros::test as spec;

// -- lsp_initialize -----------------------------------------------------------

#[spec(
    behavior = "lsp_initialize",
    verify = "initialize response includes semantic token legend"
)]
fn init_includes_semantic_legend() {
    let caps = specforge_lsp::server_capabilities(&["behavior", "type", "event"]);
    assert!(!caps.semantic_token_types.is_empty());
    assert!(caps.semantic_token_types.contains(&"keyword".to_string()));
}

#[test]
fn init_legend_includes_extension_types() {
    let caps = specforge_lsp::server_capabilities(&["behavior", "type"]);
    // Extension kinds should appear in the legend as "keyword" type
    assert!(caps.semantic_token_types.contains(&"keyword".to_string()));
    assert!(caps.semantic_token_types.contains(&"string".to_string()));
    assert!(caps.semantic_token_types.contains(&"property".to_string()));
}

#[spec(
    behavior = "lsp_initialize",
    verify = "initialize response advertises incremental sync"
)]
fn init_advertises_incremental_sync() {
    let caps = specforge_lsp::server_capabilities(&[]);
    assert!(caps.incremental_sync);
}

#[spec(
    behavior = "lsp_initialize",
    verify = "initialize response includes completion trigger characters"
)]
fn init_includes_completion_triggers() {
    let caps = specforge_lsp::server_capabilities(&[]);
    assert!(!caps.completion_trigger_characters.is_empty());
}

#[spec(
    behavior = "lsp_initialize",
    verify = "initialize response includes server_info with name and version"
)]
fn init_includes_server_info() {
    let info = specforge_lsp::server_info();
    assert_eq!(info.name, "specforge-lsp");
    assert!(!info.version.is_empty(), "version must be non-empty");
}

#[spec(
    behavior = "lsp_initialize",
    verify = "zero extensions produces structural-only capabilities"
)]
#[tokio::test]
async fn init_zero_extensions() {
    use crate::contracts::{STANDARD_TOKEN_TYPES, legend_of, project_with};
    use crate::session::Session;
    use std::time::Duration;

    let bare = project_with(&[]);
    let (mut session, init) = Session::start(Some(bare.path())).await;
    let caps = &init["capabilities"];

    // The structural capabilities are all there...
    assert_eq!(caps["textDocumentSync"], 2);
    for provider in [
        "hoverProvider",
        "definitionProvider",
        "referencesProvider",
        "codeActionProvider",
        "documentSymbolProvider",
        "workspaceSymbolProvider",
        "documentFormattingProvider",
        "documentRangeFormattingProvider",
    ] {
        assert_eq!(caps[provider], true, "{provider}");
    }
    assert_eq!(caps["renameProvider"]["prepareProvider"], true);
    assert_eq!(
        caps["completionProvider"]["triggerCharacters"],
        serde_json::json!([" ", "["])
    );
    // ...and nothing else: the legend is the standard LSP list, no entity
    // kind of any extension among it.
    assert_eq!(legend_of(&init), STANDARD_TOKEN_TYPES);
    assert!(
        session
            .notification_within("window/logMessage", Duration::ZERO, |p| {
                p["message"].as_str().is_some_and(|m| m.contains("loaded"))
            })
            .await
            .is_none(),
        "no extension was loaded"
    );

    // A project with extensions is offered the same capabilities.
    let extended = project_with(&["@specforge/software", "@specforge/testing"]);
    let (_session, with_extensions) = Session::start(Some(extended.path())).await;
    assert_eq!(with_extensions["capabilities"], *caps);
}

// -- lsp_shutdown -------------------------------------------------------------

/// Apply an editor buffer to the state's project session.
fn edit(state: &mut specforge_lsp::LspState, path: &str, text: &str) {
    // The buffer is the file's text: what navigation reads.
    state.open_document(&format!("file://{path}"), text);
    state.session_mut().expect("no update is running").update(
        specforge_project::SourceChange::Buffer {
            path,
            text: Some(text),
        },
    );
}

#[spec(
    behavior = "lsp_shutdown",
    verify = "shutdown releases in-memory graph"
)]
fn shutdown_clears_state() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///p/login.spec", LOGIN);
    edit(&mut state, "/p/login.spec", LOGIN);
    assert!(state.graph().node("login").is_some());
    let session = state.session().unwrap();
    assert_eq!(session.graph_diagnostics().len(), 1, "the E003");

    state.shutdown();

    assert_eq!(state.graph().node_count(), 0);
    assert_eq!(state.graph().edges().len(), 0);
    let session = state.session().unwrap();
    assert!(session.diagnostics().is_empty());
    assert!(session.diagnostic_files().is_empty());
    assert!(!state.is_open("file:///p/login.spec"));
    assert!(state.is_shutdown());
}

/// `login` references `session_limit`, which exists nowhere.
const LOGIN: &str = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";

#[test]
fn shutdown_sets_flag() {
    let mut state = specforge_lsp::LspState::new();
    state.shutdown();
    assert!(state.is_shutdown());
}

#[test]
fn requests_after_shutdown_rejected() {
    let mut state = specforge_lsp::LspState::new();
    state.shutdown();
    // Trying to open a document after shutdown should be ignored
    state.open_document("file:///a.spec", "content");
    assert!(!state.is_open("file:///a.spec"));
}

// -- shared_incremental_pipeline ----------------------------------------------

#[spec(
    behavior = "shared_incremental_pipeline",
    verify = "LSP and watch share the same graph"
)]
fn lsp_state_holds_graph() {
    let mut state = specforge_lsp::LspState::new();
    assert_eq!(state.graph().node_count(), 0);

    // The graph the LSP serves is the one owned by its project session,
    // the type `specforge watch` holds.
    let session: &specforge_project::ProjectSession = state.session().unwrap();
    assert!(std::ptr::eq(state.graph(), session.graph()));

    // A change driven through the session is what the LSP's features see.
    let limit = "invariant session_limit \"Limit\" {\n}\n";
    edit(&mut state, "/p/login.spec", LOGIN);
    edit(&mut state, "/p/limit.spec", limit);
    let nav = specforge_lsp::navigator(&state);
    let def = nav
        .definition("session_limit")
        .expect("the session's entity is navigable");
    assert_eq!(def.block.file, "/p/limit.spec");
    let with_declaration = specforge_ops::navigate::ReferenceQuery {
        include_declaration: true,
        ..Default::default()
    };
    let refs = nav.references("session_limit", with_declaration).unwrap();
    let ref_files: Vec<&str> = refs.iter().map(|r| r.span.file.as_str()).collect();
    assert_eq!(ref_files, ["/p/limit.spec", "/p/login.spec"]);
    drop(nav);

    // A session fed the same changes, as `specforge watch` feeds its own,
    // builds the same graph and reports the same diagnostics.
    let mut watch = specforge_project::ProjectSession::detached();
    for (path, text) in [("/p/login.spec", LOGIN), ("/p/limit.spec", limit)] {
        watch.update(specforge_project::SourceChange::Buffer {
            path,
            text: Some(text),
        });
    }
    let ids = |g: &specforge_graph::Graph| {
        let mut ids: Vec<String> = g.nodes().iter().map(|n| n.id.raw.to_string()).collect();
        ids.sort();
        ids
    };
    assert_eq!(ids(state.graph()), ["login", "session_limit"]);
    assert_eq!(ids(state.graph()), ids(watch.graph()));
    assert_eq!(state.graph().edges().len(), watch.graph().edges().len());
    assert_eq!(state.session().unwrap().diagnostics(), watch.diagnostics());
}

#[spec(
    behavior = "shared_incremental_pipeline",
    verify = "graph update serves all LSP features"
)]
fn graph_update_serves_all_features() {
    let mut state = specforge_lsp::LspState::new();

    // Build a graph through the shared session.
    edit(
        &mut state,
        "/p/auth.spec",
        "behavior login \"User Login\" {\n  types [token]\n}\n",
    );
    edit(
        &mut state,
        "/p/types.spec",
        "type token \"Auth Token\" {\n}\n",
    );

    // The same graph serves go-to-definition
    let nav = specforge_lsp::navigator(&state);
    assert!(
        nav.definition("token").is_ok(),
        "go-to-definition must use shared graph"
    );

    // The same graph serves find-all-references
    let refs = nav.references("token", Default::default()).unwrap();
    assert!(
        refs.iter().any(|r| r.span.file == "/p/auth.spec"),
        "find-all-references must use shared graph: {refs:?}"
    );

    // The same graph serves hover, through the inspect read view
    let facts = specforge_ops::inspect::inspect(&state.view(), "login")
        .expect("inspect must use shared graph");
    assert!(std::ptr::eq(
        facts.node,
        state.graph().node("login").unwrap()
    ));
    let hover = specforge_lsp::hover::entity(&facts, &[], false);
    assert!(
        hover.contains("`login`"),
        "hover must use shared graph: {hover}"
    );

    // The same graph serves workspace symbols and completions (one
    // ranking, over ids and titles)
    use specforge_ops::navigate::{EntityQuery, MatchScope, find_entities};
    let syms = find_entities(state.graph(), &EntityQuery::new("login", MatchScope::Names));
    assert!(!syms.is_empty(), "workspace symbols must use shared graph");
    let completions = find_entities(state.graph(), &EntityQuery::new("log", MatchScope::Names));
    assert!(!completions.is_empty(), "completions must use shared graph");
}

#[test]
fn lsp_debounces_like_watch() {
    assert_eq!(
        specforge_lsp::DEBOUNCE_WINDOW,
        specforge_watch::DEFAULT_DEBOUNCE_WINDOW
    );
}

#[spec(behavior = "lsp_shutdown", verify = "shutdown releases Wasm engines")]
fn shutdown_frees_the_wasm_runtime() {
    let project = tempfile::TempDir::new().unwrap();
    std::fs::write(
        project.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    let runtime: specforge_project::SharedRuntime =
        std::sync::Arc::new(specforge_component::ComponentRuntime::new());
    let engine = std::sync::Arc::downgrade(&runtime);
    let mut state = specforge_lsp::LspState::new();
    state.set_session(specforge_project::ProjectSession::open_with_runtime(
        project.path(),
        Some(runtime),
    ));
    assert!(state.session().unwrap().runtime().is_some());

    state.shutdown();

    assert!(state.session().unwrap().runtime().is_none());
    assert!(
        engine.upgrade().is_none(),
        "the engine is freed, not just forgotten"
    );
}

// While the session is out for an update, readers see the stand-in of its
// last complete graph: its entity snapshot is that graph's own, and a later
// stand-in never reads the memo of an earlier one.
#[spec(
    behavior = "snapshot_entities_once",
    verify = "a session's snapshot follows every update"
)]
fn a_stand_in_reads_the_snapshot_of_its_own_graph() {
    let project = tempfile::TempDir::new().unwrap();
    std::fs::write(
        project.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    std::fs::write(project.path().join("a.spec"), "behavior a \"A\" {\n}\n").unwrap();
    let mut state = specforge_lsp::LspState::new();
    state.set_session(specforge_project::ProjectSession::open_with_runtime(
        project.path(),
        None,
    ));

    let first_snapshot = std::sync::Arc::clone(state.session().unwrap().recorded().entities());
    let session = state.take_session().expect("the session is held");
    let first = state.view().entities().kind_of("a").map(str::to_string);
    assert_eq!(first.as_deref(), Some("behavior"));
    assert!(state.view().entities().kind_of("b").is_none());
    assert!(std::ptr::eq(state.view().entities(), &*first_snapshot));
    state.set_session(session);

    // The graph changes while the session is held; the next stand-in is of
    // the new graph, with a snapshot of its own.
    std::fs::write(project.path().join("b.spec"), "behavior b \"B\" {\n}\n").unwrap();
    let mut session = state.take_session().expect("the session is held");
    session.update(specforge_project::SourceChange::Disk(&[
        "b.spec".to_string()
    ]));
    state.set_session(session);
    let session = state.take_session().expect("the session is held");
    let view = state.view();
    assert_eq!(view.entities().kind_of("b"), Some("behavior"));
    assert!(!std::ptr::eq(view.entities(), &*first_snapshot));
    // It is the snapshot the session holds for that graph.
    assert!(std::ptr::eq(view.entities(), session.entities()));
}

/// The watchers derive from the session: relative to each input's
/// directory for a client with relative pattern support, absolute globs
/// otherwise, and the static set with no project.
#[test]
fn file_watchers_follow_what_the_session_is_built_from() {
    use tower_lsp::lsp_types::{GlobPattern, OneOf};

    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"w","version":"0.1.0","extensions":["@acme/local=ext/local.wasm"]}"#,
    )
    .unwrap();
    let session = specforge_project::ProjectSession::open_with_runtime(dir.path(), None);
    let root = dir.path().to_string_lossy().into_owned();

    let absolute: Vec<String> = specforge_lsp::watchers::file_watchers(&session, false)
        .into_iter()
        .map(|w| match w.glob_pattern {
            GlobPattern::String(glob) => glob,
            other => panic!("expected an absolute glob, got {other:?}"),
        })
        .collect();
    assert_eq!(
        absolute,
        vec![
            format!("{root}/**/*.spec"),
            format!("{root}/specforge.json"),
            format!("{root}/specforge.lock"),
            format!("{root}/ext/local.wasm"),
        ]
    );

    let relative: Vec<(std::path::PathBuf, String)> =
        specforge_lsp::watchers::file_watchers(&session, true)
            .into_iter()
            .map(|w| match w.glob_pattern {
                GlobPattern::Relative(pattern) => {
                    let OneOf::Right(base) = pattern.base_uri else {
                        panic!("a base URI")
                    };
                    (base.to_file_path().unwrap(), pattern.pattern)
                }
                other => panic!("expected a relative pattern, got {other:?}"),
            })
            .collect();
    assert!(
        relative.contains(&(dir.path().join("ext"), "local.wasm".to_string())),
        "{relative:?}"
    );
    assert!(
        relative.contains(&(dir.path().to_path_buf(), "**/*.spec".to_string())),
        "{relative:?}"
    );

    let detached = specforge_project::ProjectSession::detached();
    assert_eq!(
        specforge_lsp::watchers::file_watchers(&detached, true),
        specforge_lsp::watchers::default_watchers()
    );
}

/// Pin (plan 03, bugs L2 and L3): every input the session classifies has an
/// absolute watcher glob equal to its path as the checks join it, except a
/// file in a missing file's directory, which has none. T4 flips it: every
/// input is matched by a glob, spelled under the root.
#[test]
fn the_watchers_cover_what_the_session_classifies() {
    use specforge_project::{InputRole, ProjectSession};
    use tower_lsp::lsp_types::GlobPattern;

    let outside = tempfile::TempDir::new().unwrap();
    let far = tempfile::TempDir::new().unwrap();
    let module_dir = std::fs::canonicalize(far.path()).unwrap().join("mods");
    std::fs::create_dir_all(&module_dir).unwrap();
    let module = module_dir.join("ext.wasm");
    // From `spec/`, two levels up is the parent of the root: the sibling
    // temp directory `outside`.
    let outside_name = outside.path().file_name().unwrap().to_string_lossy();
    let reference = format!("../../{outside_name}/guide.md");
    let dir = crate::session::docref_project(&format!(
        "gadget gadget_one \"G\" {{\n  docs [\"{reference}\", \"missing/sub.md\"]\n}}\n"
    ));
    let root = dir.path();
    std::fs::write(
        root.join("specforge.json"),
        serde_json::json!({
            "name": "p",
            "version": "0.1.0",
            "spec_root": "spec",
            "extensions": [
                "@specforge/software",
                "@sdk/docref=ext/docref.wasm",
                format!("@acme/far={}", module.display()),
            ],
        })
        .to_string(),
    )
    .unwrap();
    let session = ProjectSession::open(root);

    let globs: Vec<String> = specforge_lsp::watchers::file_watchers(&session, false)
        .into_iter()
        .map(|w| match w.glob_pattern {
            GlobPattern::String(glob) => glob,
            other => panic!("expected an absolute glob, got {other:?}"),
        })
        .collect();
    let spec_root = root.join("spec");
    let inputs = [
        root.join("specforge.json"),
        root.join("specforge.lock"),
        root.join("ext/docref.wasm"),
        module,
        spec_root.join(&reference),
        spec_root.join("missing/sub.md"),
    ];
    for path in &inputs {
        assert_ne!(
            session.classify(path),
            InputRole::Unrelated,
            "{}",
            path.display()
        );
        assert!(
            globs.contains(&path.display().to_string()),
            "{} not in {globs:?}",
            path.display()
        );
    }

    // A file created beside a missing referenced file changes E016's
    // suggestion: the session classifies it, and no glob reports it.
    let sibling = spec_root.join("missing/x.md");
    assert_eq!(session.classify(&sibling), InputRole::CheckInput);
    assert!(
        !globs
            .iter()
            .any(|g| g.contains("/missing/") && !g.ends_with("/missing/sub.md")),
        "{globs:?}"
    );
}
