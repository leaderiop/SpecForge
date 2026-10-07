//! The infer prompt over a served environment, through `prompts/get`: its
//! scopes, guides, plan paging and refusals.

use crate::support::*;
use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;
use specforge_mcp::McpServer;
use specforge_test::prelude::*;

/// The infer prompt's reply for `arguments`.
fn infer(server: &mut McpServer, arguments: Value) -> Value {
    get_prompt(server, "specforge://prompts/infer", arguments)
}

/// The test extension, `@specforge/test`, declaring `kind_name` with
/// `guide` and a `description` string field.
fn test_extension(kind_name: &str, guide: Option<&str>) -> TestExtension {
    let (kind_name, guide) = (kind_name.to_string(), guide.map(str::to_string));
    TestExtension::named("@specforge/test").declaring(move |c| {
        c.kind(&kind_name, |k| {
            k.description(&format!("A test {kind_name} entity"));
            if let Some(guide) = &guide {
                k.inference_guide(guide);
            }
            k.field("description", |f| {
                f.field_type(FieldType::String).description("A description");
            });
        });
    })
}

/// An initialized server over an empty project serving the test
/// extension, which declares `kind_name` with `guide`.
fn make_state_with_kind(kind_name: &str, guide: Option<&str>) -> Served {
    TestProject::new().serve(&[test_extension(kind_name, guide)])
}

/// As [`make_state_with_kind`] for `behavior`, the project declaring the
/// behavior `my_behavior` (test.spec).
fn state_with_my_behavior() -> Served {
    TestProject::new()
        .file("test.spec", "behavior my_behavior {\n}\n")
        .serve(&[test_extension("behavior", Some("guide text"))])
}

/// A served project with `count` Rust sources under `src/`, and an
/// extension that analyzes `.rs` files.
fn plan_state_with_sources(count: usize) -> Served {
    let project = (0..count).fold(TestProject::new(), |project, i| {
        project.file(&format!("src/mod_{i:02}.rs"), "fn stub() {}\n")
    });
    project.serve(&[
        test_extension("behavior", Some("guide text")).declaring(|c| {
            c.analyzer("rust", |a| {
                a.file_extensions(&[".rs"]).scan(|_| ScanResponse {
                    items: Vec::new(),
                    language: None,
                });
            });
        }),
    ])
}

fn plan_payload(server: &mut McpServer, arguments: Value) -> Value {
    prompt_payload(&infer(server, arguments))
}

#[test]
fn overview_returns_installed_extensions() {
    let mut state = make_state_with_kind("behavior", Some("Look for public functions"));
    let resp = infer(&mut state, json!({}));
    let content: Value = prompt_payload(&resp);
    assert_eq!(content["installed_extensions"][0], "@specforge/test");
}

#[test]
fn overview_includes_inference_guide_from_extension() {
    let mut state = make_state_with_kind("behavior", Some("Look for public functions"));
    let resp = infer(&mut state, json!({}));
    let content: Value = prompt_payload(&resp);
    let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
    assert!(guide.contains("Look for public functions"));
}

#[test]
fn overview_appends_project_override() {
    let mut state = TestProject::new()
        .config(|c| {
            c["inference"] = json!({
                "global": "This is a Rust project",
                "behavior": "In our codebase, behaviors are in use_cases/",
            });
        })
        .serve(&[test_extension(
            "behavior",
            Some("Look for public functions"),
        )]);
    let resp = infer(&mut state, json!({}));
    let content: Value = prompt_payload(&resp);
    let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
    assert!(guide.contains("Look for public functions"));
    assert!(guide.contains("Project-specific"));
    assert!(guide.contains("use_cases/"));
    assert_eq!(content["project_conventions"], "This is a Rust project");
}

#[test]
fn kind_scope_returns_existing_ids() {
    let mut state = state_with_my_behavior();
    let resp = infer(&mut state, json!({"scope": "kind:behavior"}));
    let content: Value = prompt_payload(&resp);
    let ids = content["existing_entity_ids"].as_array().unwrap();
    assert!(ids.contains(&Value::from("my_behavior")));
}

#[test]
fn kind_scope_includes_example() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "kind:behavior"}));
    let content: Value = prompt_payload(&resp);
    let example = content["example"].as_str().unwrap();
    assert!(example.contains("behavior example_behavior"));
}

// Pin (plan 09 T0): flipped by T7. `tags` is registered on `behavior`
// (the extension's shared field) and the prompt leaves it out.
#[test]
fn kind_scope_lists_only_the_kinds_declared_fields() {
    let extension = test_extension("behavior", None).declaring(|c| {
        c.shared_field("tags", |f| {
            f.field_type(FieldType::StringList);
        });
    });
    let mut state = TestProject::new().serve(&[extension]);
    let resp = infer(&mut state, json!({"scope": "kind:behavior"}));
    let content: Value = prompt_payload(&resp);
    let names: Vec<&str> = content["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["description"]);
}

#[test]
fn kind_scope_is_case_insensitive() {
    let mut state = state_with_my_behavior();
    let resp = infer(&mut state, json!({"scope": "kind:Behavior"}));
    let content: Value = prompt_payload(&resp);
    let ids = content["existing_entity_ids"].as_array().unwrap();
    assert!(ids.contains(&Value::from("my_behavior")));
}

#[test]
fn unknown_scope_prefix_returns_overview() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "unknown:value"}));
    let content: Value = prompt_payload(&resp);
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
    let content: Value = prompt_payload(&resp);
    let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
    assert_eq!(guide, "");
}

#[test]
fn plan_scope_returns_kind_priorities() {
    let mut state = state_with_my_behavior();
    let resp = infer(&mut state, json!({"scope": "plan"}));
    let content: Value = prompt_payload(&resp);
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
    let content: Value = prompt_payload(&resp);
    assert_eq!(content["plan"]["target_spec_directory"], "specs/");
}

#[test]
fn plan_scope_includes_progress() {
    let mut state = make_state_with_kind("behavior", Some("guide text"));
    let resp = infer(&mut state, json!({"scope": "plan"}));
    let content: Value = prompt_payload(&resp);
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
    let content: Value = prompt_payload(&resp);
    let tools = content["tools"].as_array().unwrap();
    assert!(tools.contains(&Value::from("specforge.infer_session")));
    assert!(tools.contains(&Value::from("specforge.infer_progress")));
    let kinds = content["installed_kinds"].as_array().unwrap();
    assert!(kinds.contains(&Value::from("behavior")));
}

#[test]
fn plan_scope_caps_file_lists_at_50() {
    let mut state = plan_state_with_sources(60);
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
    let mut state = plan_state_with_sources(60);
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
    // auth_login (auth.spec) is anchored to src/auth.rs:3; cache_get, which
    // no spec declares, to src/cache.rs.
    let mut state = TestProject::new()
        .file("auth.spec", "behavior auth_login {\n}\n")
        .file("src/auth.rs", "\n\npub fn login() {}\n")
        .file(
            "specforge-anchors.json",
            &json!({"version": 1, "anchors": [
                {"entity_id": "auth_login", "file": "src/auth.rs", "line": 3,
                 "symbol_name": "login", "item_kind": "fn", "scanner": "manual"},
                {"entity_id": "cache_get", "file": "src/cache.rs", "line": 1,
                 "symbol_name": "get", "item_kind": "fn", "scanner": "manual"},
            ]})
            .to_string(),
        )
        .serve(&[test_extension("behavior", Some("guide text"))]);

    let content = prompt_payload(&infer(&mut state, json!({"scope": "file:src/auth.rs"})));
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
    let content = prompt_payload(&infer(&mut state, json!({"scope": "file:src"})));
    assert_eq!(content["match_mode"], "directory");
    assert_eq!(
        content["existing_entities_referencing_file"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    let content = prompt_payload(&infer(&mut state, json!({"scope": "file:e.rs"})));
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
    tool(
        server,
        "specforge.find_spec_for_source",
        json!({"file_path": file_path}),
    )
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
        let infer = prompt_payload(&infer(
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
        let infer = prompt_payload(&infer(
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
