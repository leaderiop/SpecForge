//! The infer prompt over a served environment, through `prompts/get`: its
//! scopes, guides, plan paging and refusals.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::{InferenceConfig, ProjectConfig, SourceSpan, Sym};
use specforge_graph::{EntityId, EntityKind, FieldMap, Node};
use specforge_mcp::McpServer;
use specforge_registry::{ManifestEntityKind, ManifestField, ManifestV2};
use specforge_test::prelude::*;

/// The infer prompt's reply for `arguments`.
fn infer(server: &mut McpServer, arguments: Value) -> Value {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "prompts/get",
        "params": {"name": "specforge://prompts/infer", "arguments": arguments}});
    serde_json::from_str(&server.handle_message(&request.to_string()).unwrap()).unwrap()
}

/// The payload of a rendered prompt: its second user message's JSON.
fn payload(reply: &Value) -> Value {
    let text = reply["result"]["messages"][1]["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("the prompt renders: {reply}"));
    serde_json::from_str(text).unwrap()
}

/// An initialized server serving the test extension, which declares
/// `kind_name` with `guide`.
fn make_state_with_kind(kind_name: &str, guide: Option<&str>) -> McpServer {
    let mut server = McpServer::new();
    server.handle_message(
        &json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}}).to_string(),
    );
    serve(
        &mut server,
        vec![test_manifest(kind_name, guide)],
        ProjectConfig::default(),
    );
    server
}

/// Serve the test extension's `manifests` with `config`, over the graph
/// already served.
fn serve(server: &mut McpServer, manifests: Vec<ManifestV2>, config: ProjectConfig) {
    let state = server.state_mut();
    let mut env = specforge_project::Environment::empty();
    env.registries.manifests = manifests;
    env.registries.extension_info = vec![("@specforge/test".to_string(), "1.0.0".to_string())];
    env.config = config;
    let graph = state.graph().clone();
    state.serve_session(specforge_project::ProjectSession::from_graph(
        Arc::new(env),
        graph,
        Vec::new(),
    ));
}

fn make_node(id: &str, kind: &str, file: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: None,
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        methods: Vec::new(),
    }
}

/// A served project rooted at a directory with `count` Rust sources, and
/// an extension that analyzes `.rs` files.
fn plan_state_with_sources(count: usize) -> (McpServer, tempfile::TempDir) {
    let mut server = make_state_with_kind("behavior", Some("guide text"));
    let mut manifest = test_manifest("behavior", Some("guide text"));
    manifest.analyzer_contributions = vec![specforge_registry::AnalyzerContribution {
        language: "rust".to_string(),
        file_extensions: vec![".rs".to_string()],
        excluded_dirs: vec![],
        scan_export: String::new(),
        classify_export: String::new(),
        map_export: String::new(),
        description: None,
    }];
    serve(&mut server, vec![manifest], ProjectConfig::default());
    let dir = tempfile::TempDir::new().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    for i in 0..count {
        std::fs::write(src.join(format!("mod_{i:02}.rs")), "fn stub() {}\n").unwrap();
    }
    crate::support::serve_in_memory_at(server.state_mut(), dir.path());
    (server, dir)
}

fn plan_payload(server: &mut McpServer, arguments: Value) -> Value {
    payload(&infer(server, arguments))
}

fn test_manifest(kind_name: &str, guide: Option<&str>) -> ManifestV2 {
    ManifestV2 {
        name: "@specforge/test".to_string(),
        version: "1.0.0".to_string(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: Default::default(),
        entity_kinds: vec![ManifestEntityKind {
            name: kind_name.to_string(),
            keyword: kind_name.to_string(),
            description: Some(format!("A test {} entity", kind_name)),
            testable: false,
            singleton: false,
            supports_verify: false,
            allowed_verify_kinds: vec![],
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            fields: vec![ManifestField {
                name: "description".to_string(),
                field_type: "string".to_string(),
                required: false,
                description: Some("A description".to_string()),
                edge: None,
                target_kind: None,
                file_reference: false,
                default_value: None,
                enum_values: vec![],
                inverse_of: None,
                normative: false,
                exempts_obligations: false,
                headline: false,
                derived_from: None,
                proof_role: None,
            }],
            incremental: None,
            has_body_parser: false,
            open_fields: false,
            contract_target: false,
            declares_types: false,
            lifecycle_field: None,
            inference_guide: guide.map(|s| s.to_string()),
        }],
        edge_types: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        fields: vec![],
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: None,
        peer_dependencies: vec![],
        sandbox_policy: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        theme_color: None,
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![],
        surfaces: None,
    }
}

#[test]
fn overview_returns_installed_extensions() {
    let mut state = make_state_with_kind("behavior", Some("Look for public functions"));
    let resp = infer(&mut state, json!({}));
    let content: Value = payload(&resp);
    assert_eq!(content["installed_extensions"][0], "@specforge/test");
}

#[test]
fn overview_includes_inference_guide_from_extension() {
    let mut state = make_state_with_kind("behavior", Some("Look for public functions"));
    let resp = infer(&mut state, json!({}));
    let content: Value = payload(&resp);
    let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
    assert!(guide.contains("Look for public functions"));
}

#[test]
fn overview_appends_project_override() {
    let mut state = make_state_with_kind("behavior", Some("Look for public functions"));
    let config = ProjectConfig {
        inference: InferenceConfig {
            global: Some("This is a Rust project".to_string()),
            kinds: {
                let mut m = HashMap::new();
                m.insert(
                    "behavior".to_string(),
                    "In our codebase, behaviors are in use_cases/".to_string(),
                );
                m
            },
            density_threshold: None,
        },
        ..Default::default()
    };
    let manifests = vec![test_manifest("behavior", Some("Look for public functions"))];
    serve(&mut state, manifests, config);
    let resp = infer(&mut state, json!({}));
    let content: Value = payload(&resp);
    let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
    assert!(guide.contains("Look for public functions"));
    assert!(guide.contains("Project-specific"));
    assert!(guide.contains("use_cases/"));
    assert_eq!(content["project_conventions"], "This is a Rust project");
}

#[test]
fn kind_scope_returns_existing_ids() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    state.state_mut().edit_graph(|graph| {
        graph.add_node(make_node("my_behavior", "behavior", "test.spec"));
    });
    let resp = infer(&mut state, json!({"scope": "kind:behavior"}));
    let content: Value = payload(&resp);
    let ids = content["existing_entity_ids"].as_array().unwrap();
    assert!(ids.contains(&Value::from("my_behavior")));
}

#[test]
fn kind_scope_includes_example() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "kind:behavior"}));
    let content: Value = payload(&resp);
    let example = content["example"].as_str().unwrap();
    assert!(example.contains("behavior example_behavior"));
}

#[test]
fn kind_scope_is_case_insensitive() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    state.state_mut().edit_graph(|graph| {
        graph.add_node(make_node("my_behavior", "behavior", "test.spec"));
    });
    let resp = infer(&mut state, json!({"scope": "kind:Behavior"}));
    let content: Value = payload(&resp);
    let ids = content["existing_entity_ids"].as_array().unwrap();
    assert!(ids.contains(&Value::from("my_behavior")));
}

#[test]
fn unknown_scope_prefix_returns_overview() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "unknown:value"}));
    let content: Value = payload(&resp);
    assert!(content.get("installed_extensions").is_some());
}

#[test]
fn empty_kind_scope_returns_error() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "kind:"}));
    assert_eq!(resp["error"]["code"], -32602, "{resp}");
    assert_eq!(resp["error"]["data"]["argument"], "scope");
}

#[test]
fn unknown_kind_returns_error() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "kind:nonexistent"}));
    assert_eq!(resp["error"]["code"], -32602, "{resp}");
    let text = resp["error"]["message"].as_str().unwrap();
    assert!(
        text.contains("nonexistent"),
        "Error should name the unknown kind: {text}"
    );
}

#[test]
fn unknown_kind_names_the_closest_installed_kind() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "kind:behaviour"}));
    let data = &resp["error"]["data"];
    assert_eq!(data["message"], "unknown entity kind 'behaviour'", "{resp}");
    assert_eq!(data["code"], "invalid_input");
    assert_eq!(data["argument"], "scope");
    assert_eq!(data["data"]["suggestion"], "did you mean 'behavior'?");
}

#[test]
fn empty_file_scope_returns_error() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "file:"}));
    assert_eq!(resp["error"]["code"], -32602, "{resp}");
    assert_eq!(resp["error"]["data"]["argument"], "scope");
}

#[test]
fn overview_with_no_inference_guide() {
    let mut state = make_state_with_kind("behavior", None);
    let resp = infer(&mut state, json!({}));
    let content: Value = payload(&resp);
    let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
    assert_eq!(guide, "");
}

#[test]
fn plan_scope_returns_kind_priorities() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    state.state_mut().edit_graph(|graph| {
        graph.add_node(make_node("my_behavior", "behavior", "test.spec"));
    });
    let resp = infer(&mut state, json!({"scope": "plan"}));
    let content: Value = payload(&resp);
    let priorities = content["plan"]["kind_priorities"].as_array().unwrap();
    assert!(!priorities.is_empty());
    assert_eq!(priorities[0]["kind"], "behavior");
    assert_eq!(priorities[0]["existing_count"], 1);
}

#[test]
fn plan_scope_respects_target_directory() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(
        &mut state,
        json!({"scope": "plan", "target_spec_directory": "specs/"}),
    );
    let content: Value = payload(&resp);
    assert_eq!(content["plan"]["target_spec_directory"], "specs/");
}

#[test]
fn plan_scope_includes_progress() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "plan"}));
    let content: Value = payload(&resp);
    assert!(content["plan"]["progress"]["files_total"].is_number());
}

#[test]
fn workflow_scope_returns_protocol() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "workflow"}));
    let instruction = resp["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{resp}"));
    assert!(instruction.contains("Start Session"));
    assert!(instruction.contains("mark_analyzed"));
    assert!(instruction.contains("End Session"));
}

#[test]
fn workflow_scope_lists_tools_and_kinds() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "workflow"}));
    let content: Value = payload(&resp);
    let tools = content["tools"].as_array().unwrap();
    assert!(tools.contains(&Value::from("specforge.infer_session")));
    assert!(tools.contains(&Value::from("specforge.infer_progress")));
    let kinds = content["installed_kinds"].as_array().unwrap();
    assert!(kinds.contains(&Value::from("behavior")));
}

#[test]
fn plan_scope_caps_file_lists_at_50() {
    let (mut state, _dir) = plan_state_with_sources(60);
    let content = plan_payload(&mut state, json!({"scope": "plan"}));
    let files = content["plan"]["unanalyzed_files"].as_array().unwrap();
    assert_eq!(
        files.len(),
        51,
        "50 files plus the trailing truncation marker"
    );
    assert!(
        files[50]
            .as_str()
            .unwrap()
            .contains("... and 10 more (use the cursor param)"),
        "marker must name the withheld count: {}",
        files[50]
    );
    assert_eq!(content["plan"]["unanalyzed_total"], 60);
    assert_eq!(content["plan"]["next_cursor"], 50);
}

#[test]
fn plan_scope_pages_remaining_files_via_cursor() {
    let (mut state, _dir) = plan_state_with_sources(60);
    let content = plan_payload(&mut state, json!({"scope": "plan", "cursor": 50}));
    let files = content["plan"]["unanalyzed_files"].as_array().unwrap();
    assert_eq!(files.len(), 10, "only the remainder is listed");
    assert!(
        content["plan"]["next_cursor"].is_null(),
        "no further page exists"
    );
}

#[test]
fn file_scope_lists_the_entities_anchored_to_the_file() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    state.state_mut().edit_graph(|graph| {
        graph.add_node(make_node("auth_login", "behavior", "auth.spec"));
    });
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge-anchors.json"),
        json!({"version": 1, "anchors": [
            {"entity_id": "auth_login", "file": "src/auth.rs", "line": 3,
             "symbol_name": "login", "item_kind": "fn", "scanner": "manual"},
            {"entity_id": "cache_get", "file": "src/cache.rs", "line": 1,
             "symbol_name": "get", "item_kind": "fn", "scanner": "manual"},
        ]})
        .to_string(),
    )
    .unwrap();
    crate::support::serve_in_memory_at(state.state_mut(), dir.path());

    let content = payload(&infer(&mut state, json!({"scope": "file:src/auth.rs"})));
    assert_eq!(content["match_mode"], "exact");
    let refs = content["existing_entities_referencing_file"]
        .as_array()
        .unwrap();
    assert_eq!(refs.len(), 1, "{content}");
    assert_eq!(refs[0]["entity_id"], "auth_login");
    assert_eq!(refs[0]["kind"], "behavior");
    assert_eq!(refs[0]["line"], 3);
    assert_eq!(refs[0]["symbol_name"], "login");

    // A directory lists the files under it; a substring is no match.
    let content = payload(&infer(&mut state, json!({"scope": "file:src"})));
    assert_eq!(content["match_mode"], "directory");
    assert_eq!(
        content["existing_entities_referencing_file"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    let content = payload(&infer(&mut state, json!({"scope": "file:e.rs"})));
    assert_eq!(content["match_mode"], "none");
}

/// A project on disk, compiled with `@specforge/software`: behavior
/// `alpha` declared in `spec/main.spec` and anchored to `src/login.rs:1`;
/// `src/other.rs` anchors nothing.
fn anchored_project() -> (McpServer, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("specforge.json"),
        json!({"name": "probe", "version": "0.1.0", "spec_root": "spec",
               "extensions": ["@specforge/software"]})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join("spec/main.spec"),
        "behavior alpha \"Alpha\" {\n  contract \"first\"\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("src/login.rs"), "pub fn login() {}\n").unwrap();
    std::fs::write(root.join("src/other.rs"), "pub fn other() {}\n").unwrap();
    std::fs::write(
        root.join("specforge-anchors.json"),
        json!({"version": 1, "anchors": [{"entity_id": "alpha", "file": "src/login.rs",
               "line": 1, "symbol_name": "login", "item_kind": "fn", "scanner": "manual"}]})
        .to_string(),
    )
    .unwrap();
    let mut server = McpServer::with_project_root(root.to_path_buf());
    server.handle_message(
        &json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}}).to_string(),
    );
    (server, dir)
}

/// `specforge.find_spec_for_source`'s structured result for `file_path`.
fn find_spec_for_source(server: &mut McpServer, file_path: &str) -> Value {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "specforge.find_spec_for_source", "arguments": {"file_path": file_path}}});
    let reply: Value =
        serde_json::from_str(&server.handle_message(&request.to_string()).unwrap()).unwrap();
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{reply}"));
    serde_json::from_str(text).unwrap()
}

fn entity_ids(entities: &Value) -> Vec<String> {
    entities
        .as_array()
        .unwrap_or_else(|| panic!("{entities}"))
        .iter()
        .map(|e| e["entity_id"].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "provide_infer_file_scope",
    verify = "file scope lists the entities find_spec_for_source finds for the same file"
)]
fn infer_file_scope_agrees_with_find_spec_for_source() {
    let (mut server, _dir) = anchored_project();
    for spelling in [
        "src/login.rs",
        "./src/login.rs",
        "src\\login.rs",
        "src",
        "login.rs",
    ] {
        let infer = payload(&infer(
            &mut server,
            json!({"scope": format!("file:{spelling}")}),
        ));
        let tool = find_spec_for_source(&mut server, spelling);
        assert_eq!(
            infer["existing_entities_referencing_file"], tool["entities"],
            "{spelling}"
        );
        assert_eq!(infer["match_mode"], tool["match_mode"], "{spelling}");
        assert_eq!(entity_ids(&tool["entities"]), ["alpha"], "{spelling}");
    }
    let tool = find_spec_for_source(&mut server, "src/login.rs");
    let alpha = &tool["entities"][0];
    assert_eq!(alpha["kind"], "behavior");
    assert_eq!(alpha["line"], 1);
    assert_eq!(alpha["symbol_name"], "login");
}

#[specforge_test(
    behavior = "provide_infer_file_scope",
    verify = "an unanchored file lists no existing entities"
)]
fn infer_file_scope_lists_nothing_for_an_unanchored_file() {
    let (mut server, _dir) = anchored_project();
    // main.spec declares alpha, but a spec file anchors no source.
    for file in ["src/other.rs", "main.spec", "spec/main.spec"] {
        let infer = payload(&infer(
            &mut server,
            json!({"scope": format!("file:{file}")}),
        ));
        assert_eq!(
            infer["existing_entities_referencing_file"],
            json!([]),
            "{file}"
        );
        assert_eq!(infer["match_mode"], "none", "{file}");
    }
}
