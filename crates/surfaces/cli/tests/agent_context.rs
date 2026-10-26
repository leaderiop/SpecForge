//! The context export and the context prompt carry the same members of every entity (ADR 0236).

use serde_json::{Value, json};
use specforge_test_macros::test as specforge_test;

use crate::support::TestProject;
use crate::support::pairing::{mcp_calls, served};

/// Every entity of `project`: its context resource (depth 0) against its context prompt.
fn assert_one_agent_context(project: TestProject) {
    let mut served = served(project);
    let listed = mcp_calls(
        &mut served,
        &[json!({"name": "specforge.list", "arguments": {}})],
    )
    .pop()
    .unwrap();
    let ids: Vec<String> = listed["entities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] != "spec") // a spec's id (`todo-app`) is no entity ID
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.len() > 1, "{listed}");
    for id in &ids {
        let calls = [
            json!({"method": "resources/read", "params": {"uri": format!("specforge://context/{id}?depth=0")}}),
            json!({"method": "prompts/get", "params": {"name": "specforge://prompts/context", "arguments": {"entity_id": id}}}),
        ];
        let [export, prompt]: [Value; 2] = mcp_calls(&mut served, &calls).try_into().unwrap();
        let node = &export["nodes"][0];
        assert!(!node.is_null(), "{export}");
        let payload: Value =
            serde_json::from_str(prompt["messages"][1]["content"]["text"].as_str().unwrap())
                .unwrap();
        // Every member the prompt carries is in the node once: lifted (a string) or under `fields`.
        let fields = payload["fields"].as_object().unwrap();
        for (name, value) in fields {
            match node["fields"].get(name) {
                Some(kept) => assert_eq!(kept, value, "{id}.{name}: {node}"),
                None => assert!(
                    node[name].is_string(),
                    "{id}: `{name}` neither kept nor lifted: {node}"
                ),
            }
        }
        for name in node["fields"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, _)| k)
        {
            assert!(
                fields.contains_key(name),
                "{id}: `{name}` only in the export"
            );
        }
        assert_eq!(node.get("annotations"), payload.get("annotations"), "{id}");
        assert_eq!(node.get("methods"), payload.get("methods"), "{id}");
        let stated: Vec<String> = node["verify"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|o| {
                format!(
                    "{} {}",
                    o["kind"].as_str().unwrap_or(""),
                    o["text"].as_str().unwrap()
                )
            })
            .collect();
        assert_eq!(
            serde_json::to_value(&stated).unwrap(),
            payload["verify_expectations"],
            "{id}"
        );
    }
}

#[specforge_test(
    behavior = "export_agent_context_format",
    verify = "the context export and the context prompt carry the same members of every entity"
)]
fn the_context_export_and_prompt_carry_the_same_members() {
    assert_one_agent_context(TestProject::copy_of(
        specforge_repo_root::repo_root().join("examples/todo-app"),
    ));
    let project = TestProject::new().raw_config(
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software","@specforge/formal"]}"#,
    );
    project.write(
        "app.spec",
        "behavior save \"Save\" {\n  contract \"The system MUST save\"\n  requires {\n    open \"the store is open\"\n  }\n  ensures {\n    saved \"the record is on disk\"\n  }\n}\nevent stored \"Stored\" {\n  channel  \"store.saved\"\n  category domain\n}\n",
    );
    assert_one_agent_context(project);
}
