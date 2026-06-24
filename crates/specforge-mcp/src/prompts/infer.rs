use std::collections::HashMap;
use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let scope = args.get("scope").and_then(|v| v.as_str());

    match scope {
        Some("plan") => get_plan(state, &args, id),
        Some("workflow") => get_workflow(state, id),
        Some(s) if s.starts_with("kind:") => {
            let kind_name = s[5..].to_lowercase();
            if kind_name.is_empty() {
                return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Empty kind name in scope 'kind:'");
            }
            get_kind_scoped(state, &kind_name, id)
        }
        Some(s) if s.starts_with("file:") => {
            let file_path = &s[5..];
            if file_path.is_empty() {
                return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Empty file path in scope 'file:'");
            }
            get_file_scoped(state, file_path, id)
        }
        _ => get_overview(state, id),
    }
}

fn get_overview(state: &McpState, id: Option<Value>) -> JsonRpcResponse {
    let mut kind_counts: HashMap<String, usize> = HashMap::new();
    for node in state.graph.nodes() {
        *kind_counts.entry(node.kind.raw.to_string()).or_default() += 1;
    }

    let mut kinds_info: Vec<Value> = Vec::new();
    for manifest in &state.manifests {
        for kind in &manifest.entity_kinds {
            let keyword = kind.keyword.to_lowercase();
            let guide = build_guide_for_kind(&keyword, manifest, &state.project_config.inference);
            let fields: Vec<String> = kind.fields.iter().map(|f| {
                if f.required { format!("{}*", f.name) } else { f.name.clone() }
            }).collect();

            kinds_info.push(serde_json::json!({
                "kind": keyword,
                "extension": manifest.name,
                "description": kind.description,
                "fields": fields,
                "inference_guide": guide,
            }));
        }
    }

    let global_conventions = state.project_config.inference.global.as_deref().unwrap_or("");

    let result = serde_json::json!({
        "installed_extensions": state.extension_info.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>(),
        "existing_entities": kind_counts,
        "kinds": kinds_info,
        "project_conventions": global_conventions,
        "output_format": "Write .spec files in the spec/ directory. Use `keyword entity_id \"Title\" { fields }` syntax. Entity IDs are snake_case identifiers (letters, digits, underscores, 2-60 chars).",
        "validation": "After writing .spec files, call specforge_validate to check for errors. Fix any errors before proceeding.",
    });

    let instruction = "You are inferring spec entities from this codebase. \
        Use the inference guides below to identify entities in the code, \
        write .spec files, and validate them with specforge_validate. \
        Each kind has signals describing what to look for in code. \
        Do not duplicate entities that already exist.";

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": { "type": "text", "text": instruction }
            },
            {
                "role": "assistant",
                "content": { "type": "text", "text": result.to_string() }
            }
        ]
    }))
}

fn get_kind_scoped(state: &McpState, kind_name: &str, id: Option<Value>) -> JsonRpcResponse {
    let matched_kind = state.manifests.iter()
        .flat_map(|m| m.entity_kinds.iter().map(move |k| (m, k)))
        .find(|(_, k)| k.keyword.to_lowercase() == kind_name);

    let (manifest, kind_def) = match matched_kind {
        Some(pair) => pair,
        None => return JsonRpcResponse::error(
            id, error_codes::INVALID_PARAMS,
            format!("Unknown entity kind: '{}'. Check installed extensions.", kind_name),
        ),
    };

    let existing_ids: Vec<String> = state.graph.nodes().into_iter()
        .filter(|n| n.kind.raw == kind_name)
        .map(|n| n.id.raw.to_string())
        .collect();

    let guide = build_guide_for_kind(kind_name, manifest, &state.project_config.inference);
    let fields: Vec<Value> = kind_def.fields.iter().map(|f| {
        serde_json::json!({
            "name": f.name,
            "type": f.field_type,
            "required": f.required,
            "description": f.description,
        })
    }).collect();
    let example = build_example_for_kind(kind_name, &kind_def.fields);

    let result = serde_json::json!({
        "kind": kind_name,
        "existing_entity_ids": existing_ids,
        "fields": fields,
        "inference_guide": guide,
        "example": example,
        "validation": "After writing .spec files, call specforge_validate to check for errors. Fix any errors before proceeding.",
    });

    let instruction = format!(
        "You are inferring '{}' entities from this codebase. \
         Use the guide below. Do not duplicate the existing entity IDs listed.",
        kind_name
    );

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": { "type": "text", "text": instruction }
            },
            {
                "role": "assistant",
                "content": { "type": "text", "text": result.to_string() }
            }
        ]
    }))
}

fn get_file_scoped(state: &McpState, file_path: &str, id: Option<Value>) -> JsonRpcResponse {
    let referencing_entities: Vec<String> = state.graph.nodes().into_iter()
        .filter(|n| {
            let span_file: &str = n.source_span.file.as_str();
            span_file.contains(file_path)
        })
        .map(|n| format!("{} ({})", n.id.raw, n.kind.raw))
        .collect();

    let mut kinds_info: Vec<Value> = Vec::new();
    for manifest in &state.manifests {
        for kind in &manifest.entity_kinds {
            let keyword = kind.keyword.to_lowercase();
            let guide = build_guide_for_kind(&keyword, manifest, &state.project_config.inference);
            kinds_info.push(serde_json::json!({
                "kind": keyword,
                "inference_guide": guide,
            }));
        }
    }

    let global_conventions = state.project_config.inference.global.as_deref().unwrap_or("");

    let result = serde_json::json!({
        "file": file_path,
        "existing_entities_referencing_file": referencing_entities,
        "kinds": kinds_info,
        "project_conventions": global_conventions,
        "validation": "After writing .spec files, call specforge_validate to check for errors. Fix any errors before proceeding.",
    });

    let instruction = format!(
        "You are inferring spec entities from the file '{}'. \
         The entities listed below already reference this file — do not duplicate them. \
         Use the kind guides to identify new entities.",
        file_path
    );

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": { "type": "text", "text": instruction }
            },
            {
                "role": "assistant",
                "content": { "type": "text", "text": result.to_string() }
            }
        ]
    }))
}

fn get_plan(state: &McpState, args: &Value, id: Option<Value>) -> JsonRpcResponse {
    let target_spec_directory = args
        .get("target_spec_directory")
        .and_then(|v| v.as_str())
        .unwrap_or("spec/");

    let project_root = state.project_root.as_deref();

    let (summary, unanalyzed, stale) = match project_root {
        Some(root) => {
            let manifest = specforge_common::inference::load_inference_manifest(root)
                .unwrap_or_default();
            let analyzer_configs: Vec<specforge_common::AnalyzerConfig> = state.manifests.iter()
                .flat_map(|m| m.analyzer_contributions.iter())
                .map(|ac| specforge_common::AnalyzerConfig {
                    language: ac.language.clone(),
                    file_extensions: ac.file_extensions.clone(),
                    excluded_dirs: ac.excluded_dirs.clone(),
                })
                .collect();
            let discovery_config = specforge_common::inference::discovery::SourceDiscoveryConfig::from_analyzer_configs(&analyzer_configs);
            let source_files = specforge_common::inference::discover_source_files(root, &manifest.source_roots, &discovery_config);
            let index_map = manifest.source_index_map();

            let unanalyzed: Vec<String> = source_files
                .iter()
                .filter(|f| !index_map.contains_key(f.as_str()))
                .cloned()
                .collect();

            let (stale_entries, _deleted) =
                specforge_common::inference::detect_stale_entries(root, &manifest);
            let summary = manifest.compute_summary(source_files.len());
            (summary, unanalyzed, stale_entries)
        }
        None => {
            let summary = specforge_common::InferenceSummary {
                files_total: 0,
                files_analyzed: 0,
                entities_produced: 0,
            };
            (summary, Vec::new(), Vec::new())
        }
    };

    let kind_priorities: Vec<Value> = state.manifests.iter()
        .flat_map(|m| m.entity_kinds.iter().map(move |k| (m, k)))
        .map(|(m, k)| {
            let keyword = k.keyword.to_lowercase();
            let existing_count = state.graph.nodes().into_iter()
                .filter(|n| n.kind.raw == keyword.as_str())
                .count();
            serde_json::json!({
                "kind": keyword,
                "extension": m.name,
                "existing_count": existing_count,
            })
        })
        .collect();

    let result = serde_json::json!({
        "plan": {
            "target_spec_directory": target_spec_directory,
            "progress": {
                "files_total": summary.files_total,
                "files_analyzed": summary.files_analyzed,
                "entities_produced": summary.entities_produced,
            },
            "unanalyzed_files": unanalyzed,
            "stale_files": stale,
            "kind_priorities": kind_priorities,
        }
    });

    let instruction = format!(
        "Create a prioritized inference plan. There are {} unanalyzed files and {} stale files. \
         Write .spec files to '{}'. Process files with the most entity signals first. \
         Use specforge.infer_session to track progress (start → mark_analyzed per file → end). \
         After each file, call specforge.validate to check for errors.",
        unanalyzed.len(), stale.len(), target_spec_directory
    );

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": { "type": "text", "text": instruction }
            },
            {
                "role": "assistant",
                "content": { "type": "text", "text": result.to_string() }
            }
        ]
    }))
}

fn get_workflow(state: &McpState, id: Option<Value>) -> JsonRpcResponse {
    let tool_names: Vec<&str> = vec![
        "specforge.infer_session",
        "specforge.infer_progress",
        "specforge.validate",
        "specforge.query",
        "specforge.search",
        "specforge.schema",
    ];

    let installed_kinds: Vec<String> = state.manifests.iter()
        .flat_map(|m| m.entity_kinds.iter())
        .map(|k| k.keyword.to_lowercase())
        .collect();

    let result = serde_json::json!({
        "tools": tool_names,
        "installed_kinds": installed_kinds,
    });

    let workflow = "\
## Inference Workflow Protocol

### Step 1: Start Session
Call `specforge.infer_session` with `action: \"start\"` and `agent: \"<your-id>\"`.
Optionally set `source_roots` to limit scanning scope.

### Step 2: Check Progress
Call `specforge.infer_progress` to see unanalyzed files and current state.

### Step 3: For Each Source File
1. Read the source file
2. Identify entities (behaviors, types, events, etc.) using entity kind guides
3. Write a `.spec` file with the discovered entities
4. Call `specforge.validate` to check for errors — fix any before proceeding
5. Call `specforge.infer_session` with `action: \"mark_analyzed\"`, `source_file`, and `entities_produced`

### Step 4: Validate Continuously
After every 3-5 files, call `specforge.validate` to catch cross-file issues.
Use `specforge.search` to find existing entities and avoid duplicates.
Use `specforge.query` to check how new entities connect to the graph.

### Step 5: End Session
Call `specforge.infer_session` with `action: \"end\"` and the `session_id` from Step 1.
Use `status: \"completed\"` when done, or `status: \"paused\"` to resume later.

### Retry Pattern
If validation fails, fix the .spec file and re-validate. Do not skip errors.
If a file has no identifiable entities, still mark it as analyzed with an empty `entities_produced`.
";

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": { "type": "text", "text": workflow }
            },
            {
                "role": "assistant",
                "content": { "type": "text", "text": result.to_string() }
            }
        ]
    }))
}

fn build_guide_for_kind(
    kind_name: &str,
    manifest: &specforge_registry::ManifestV2,
    inference_config: &specforge_common::InferenceConfig,
) -> String {
    let extension_guide = manifest.entity_kinds.iter()
        .find(|k| k.keyword.to_lowercase() == kind_name)
        .and_then(|k| k.inference_guide.as_deref())
        .unwrap_or("");

    let project_override = inference_config.kinds.get(kind_name);

    match project_override {
        Some(override_text) if !extension_guide.is_empty() => {
            format!("{}\n\n**Project-specific:**\n{}", extension_guide, override_text)
        }
        Some(override_text) => override_text.clone(),
        None => extension_guide.to_string(),
    }
}

fn build_example_for_kind(kind_name: &str, fields: &[specforge_registry::ManifestField]) -> String {
    let required_fields: Vec<&specforge_registry::ManifestField> = fields.iter()
        .filter(|f| f.required)
        .collect();
    let optional_fields: Vec<&specforge_registry::ManifestField> = fields.iter()
        .filter(|f| !f.required)
        .take(3)
        .collect();

    let mut lines = vec![
        format!("{} example_{} \"Example Title\" {{", kind_name, kind_name),
    ];

    for f in &required_fields {
        lines.push(format!("  {} \"...\"", f.name));
    }
    for f in &optional_fields {
        match f.field_type.as_str() {
            "reference_list" => lines.push(format!("  {} [ref_1, ref_2]", f.name)),
            "string_list" => lines.push(format!("  {} [\"item1\", \"item2\"]", f.name)),
            "reference" => lines.push(format!("  {} ref_id", f.name)),
            _ => lines.push(format!("  {} \"...\"", f.name)),
        }
    }

    lines.push("}".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::McpState;
    use specforge_common::{InferenceConfig, ProjectConfig, SourceSpan, Sym};
    use specforge_graph::{EntityId, EntityKind, FieldMap, Node};
    use specforge_registry::{ManifestV2, ManifestEntityKind, ManifestField};

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
                fields: vec![
                    ManifestField {
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
                    },
                ],
                incremental: None,
                has_body_parser: false,
                open_fields: false,
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
            grammar_contributions: vec![],
            body_parser_contributions: vec![],
            ext_short: None,
            query_scope: None,
            collector_contributions: vec![],
            analyzer_contributions: vec![],
            surfaces: None,
        }
    }

    fn make_state_with_kind(kind_name: &str, guide: Option<&str>) -> McpState {
        let mut state = McpState::new();
        state.manifests = vec![test_manifest(kind_name, guide)];
        state.extension_info = vec![("@specforge/test".to_string(), "1.0.0".to_string())];
        state
    }

    fn make_node(id: &str, kind: &str, file: &str) -> Node {
        Node {
            id: EntityId { raw: Sym::new(id) },
            kind: EntityKind { raw: Sym::new(kind) },
            title: None,
            fields: FieldMap::new(),
            source_span: SourceSpan {
                file: Sym::new(file),
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
            },
        }
    }

    #[test]
    fn overview_returns_installed_extensions() {
        let state = make_state_with_kind("behavior", Some("Look for public functions"));
        let resp = get(&state, serde_json::json!({}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        assert_eq!(content["installed_extensions"][0], "@specforge/test");
    }

    #[test]
    fn overview_includes_inference_guide_from_extension() {
        let state = make_state_with_kind("behavior", Some("Look for public functions"));
        let resp = get(&state, serde_json::json!({}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
        assert!(guide.contains("Look for public functions"));
    }

    #[test]
    fn overview_appends_project_override() {
        let mut state = make_state_with_kind("behavior", Some("Look for public functions"));
        state.project_config = ProjectConfig {
            inference: InferenceConfig {
                global: Some("This is a Rust project".to_string()),
                kinds: {
                    let mut m = HashMap::new();
                    m.insert("behavior".to_string(), "In our codebase, behaviors are in use_cases/".to_string());
                    m
                },
                density_threshold: None,
            },
            ..Default::default()
        };
        let resp = get(&state, serde_json::json!({}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
        assert!(guide.contains("Look for public functions"));
        assert!(guide.contains("Project-specific"));
        assert!(guide.contains("use_cases/"));
        assert_eq!(content["project_conventions"], "This is a Rust project");
    }

    #[test]
    fn kind_scope_returns_existing_ids() {
        let mut state = make_state_with_kind("behavior", Some("guide text"));
        state.graph.add_node(make_node("my_behavior", "behavior", "test.spec"));
        let resp = get(&state, serde_json::json!({"scope": "kind:behavior"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let ids = content["existing_entity_ids"].as_array().unwrap();
        assert!(ids.contains(&Value::from("my_behavior")));
    }

    #[test]
    fn kind_scope_includes_example() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "kind:behavior"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let example = content["example"].as_str().unwrap();
        assert!(example.contains("behavior example_behavior"));
    }

    #[test]
    fn file_scope_returns_referencing_entities() {
        let mut state = make_state_with_kind("behavior", Some("guide text"));
        state.graph.add_node(make_node("auth_login", "behavior", "src/auth.rs"));
        let resp = get(&state, serde_json::json!({"scope": "file:src/auth.rs"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let refs = content["existing_entities_referencing_file"].as_array().unwrap();
        assert!(!refs.is_empty());
        assert!(refs[0].as_str().unwrap().contains("auth_login"));
    }

    #[test]
    fn kind_scope_is_case_insensitive() {
        let mut state = make_state_with_kind("behavior", Some("guide text"));
        state.graph.add_node(make_node("my_behavior", "behavior", "test.spec"));
        let resp = get(&state, serde_json::json!({"scope": "kind:Behavior"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let ids = content["existing_entity_ids"].as_array().unwrap();
        assert!(ids.contains(&Value::from("my_behavior")));
    }

    #[test]
    fn unknown_scope_prefix_returns_overview() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "unknown:value"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        assert!(content.get("installed_extensions").is_some());
    }

    #[test]
    fn empty_kind_scope_returns_error() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "kind:"}), Some(Value::from(1)));
        assert!(resp.error.is_some(), "Expected error for empty kind name");
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn unknown_kind_returns_error() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "kind:nonexistent"}), Some(Value::from(1)));
        assert!(resp.error.is_some(), "Expected error for unknown kind");
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("nonexistent"), "Error should name the unknown kind");
    }

    #[test]
    fn empty_file_scope_returns_error() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "file:"}), Some(Value::from(1)));
        assert!(resp.error.is_some(), "Expected error for empty file path");
        assert_eq!(resp.error.unwrap().code, -32602);
    }

    #[test]
    fn overview_with_no_inference_guide() {
        let state = make_state_with_kind("behavior", None);
        let resp = get(&state, serde_json::json!({}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let guide = content["kinds"][0]["inference_guide"].as_str().unwrap();
        assert_eq!(guide, "");
    }

    #[test]
    fn plan_scope_returns_kind_priorities() {
        let mut state = make_state_with_kind("behavior", Some("guide text"));
        state.graph.add_node(make_node("my_behavior", "behavior", "test.spec"));
        let resp = get(&state, serde_json::json!({"scope": "plan"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let priorities = content["plan"]["kind_priorities"].as_array().unwrap();
        assert!(!priorities.is_empty());
        assert_eq!(priorities[0]["kind"], "behavior");
        assert_eq!(priorities[0]["existing_count"], 1);
    }

    #[test]
    fn plan_scope_respects_target_directory() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "plan", "target_spec_directory": "specs/"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        assert_eq!(content["plan"]["target_spec_directory"], "specs/");
    }

    #[test]
    fn plan_scope_includes_progress() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "plan"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        assert!(content["plan"]["progress"]["files_total"].is_number());
    }

    #[test]
    fn workflow_scope_returns_protocol() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "workflow"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let instruction = messages[0]["content"]["text"].as_str().unwrap();
        assert!(instruction.contains("Start Session"));
        assert!(instruction.contains("mark_analyzed"));
        assert!(instruction.contains("End Session"));
    }

    #[test]
    fn workflow_scope_lists_tools_and_kinds() {
        let state = make_state_with_kind("behavior", Some("guide text"));
        let resp = get(&state, serde_json::json!({"scope": "workflow"}), Some(Value::from(1)));
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        let content: Value = serde_json::from_str(messages[1]["content"]["text"].as_str().unwrap()).unwrap();
        let tools = content["tools"].as_array().unwrap();
        assert!(tools.contains(&Value::from("specforge.infer_session")));
        assert!(tools.contains(&Value::from("specforge.infer_progress")));
        let kinds = content["installed_kinds"].as_array().unwrap();
        assert!(kinds.contains(&Value::from("behavior")));
    }
}
