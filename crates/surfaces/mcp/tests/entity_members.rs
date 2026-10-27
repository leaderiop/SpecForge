//! What an entity holds is written once (ADR 0222): inspect and the context prompt carry an entity's
//! methods and its fields' annotations as the graph export writes them.

use crate::support::*;
use serde_json::{Value, json};
use specforge_test::prelude::*;

const TEXT: &str = r#"type Order "Order" {
  id string @optional @readonly
  note string @since "1.2"
  address {
    street string @optional
  }
}
port order_store "Order store" {
  direction outbound
  method save(order: Order @optional, force?: bool) -> Result<void, never>
}
behavior b3 "B3" {
  contract "c"
  verify unit "tests pass"
  verify "unit tests pass"
}
"#;

fn served() -> Served {
    TestProject::new()
        .file("a.spec", TEXT)
        .serve(&[TestExtension::software()])
}

fn tool(server: &mut Served, name: &str, args: Value) -> Value {
    serde_json::from_str(&tool_text(&call_tool(server, name, args))).unwrap()
}

#[specforge_test(
    behavior = "inspect_entity_facts",
    verify = "an entity's facts carry its methods and its fields' annotations as the graph export writes them"
)]
fn inspect_answers_methods_and_annotations() {
    let mut server = served();
    for (id, key) in [("order_store", "methods"), ("Order", "annotations")] {
        let inspected = tool(&mut server, "specforge.inspect", json!({"entity_id": id}));
        let queried = tool(
            &mut server,
            "specforge.query",
            json!({"entity_id": id, "depth": 0}),
        );
        assert_eq!(
            queried["nodes"][0][key], inspected[key],
            "{id}: {inspected}"
        );
        assert!(inspected[key].is_array() || inspected[key].is_object());
    }
}

#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "the context prompt carries the entity's methods and its fields' annotations, a structural constraint's too"
)]
fn the_context_prompt_carries_methods_and_annotations() {
    let mut server = served();
    let resp = get_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "order_store", "structural_constraints": ["Order"]}),
    );
    let payload: Value = serde_json::from_str(&prompt_text(&resp)).unwrap();
    assert_eq!(payload["methods"][0]["name"], "save", "{payload}");
    assert_eq!(
        payload["structural_constraint_entities"][0]["annotations"]["id"],
        json!([{"name": "optional"}, {"name": "readonly"}]),
        "{payload}"
    );
}

fn prompt_payload(server: &mut Served, id: &str) -> Value {
    let resp = get_prompt(
        server,
        "specforge://prompts/context",
        json!({"entity_id": id}),
    );
    serde_json::from_str(&prompt_text(&resp)).unwrap()
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "inspect writes each obligation as its kind, absent for a bare verify, and its text"
)]
fn inspect_writes_each_obligation_as_its_kind_and_its_text() {
    let mut server = served();
    let want = json!([{"kind": "unit", "text": "tests pass"}, {"text": "unit tests pass"}]);
    let inspected = tool(&mut server, "specforge.inspect", json!({"entity_id": "b3"}));
    assert_eq!(inspected["verify_declarations"], want, "{inspected}");
    let payload = prompt_payload(&mut server, "b3");
    assert_eq!(payload["verify_expectations"], want, "{payload}");
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "an obligation inspect writes, given to remove_verify, removes that verify line"
)]
fn an_inspected_obligation_is_the_one_remove_verify_removes() {
    let mut server = served();
    let inspected = tool(&mut server, "specforge.inspect", json!({"entity_id": "b3"}));
    let lines = [
        "-  verify unit \"tests pass\"",
        "-  verify \"unit tests pass\"",
    ];
    for (item, line) in inspected["verify_declarations"]
        .as_array()
        .unwrap()
        .iter()
        .zip(lines)
    {
        let mut edit = json!({"op": "remove_verify", "entity_id": "b3"});
        match item {
            Value::Object(record) => edit.as_object_mut().unwrap().extend(record.clone()),
            text => edit["text"] = text.clone(),
        }
        let reply = tool(
            &mut server,
            "specforge.edit",
            json!({"edits": [edit], "dry_run": true}),
        );
        let diff = reply["diff"].as_str().unwrap_or_default();
        assert!(diff.contains(line), "{item}: {reply}");
        assert_eq!(reply["entities"], json!(["b3"]), "{item}: {reply}");
    }
}

#[specforge_test(
    behavior = "inspect_entity_facts",
    verify = "inspect, the context prompt and the graph and context exports write an entity's obligations as one record"
)]
fn every_reply_writes_an_obligation_as_the_graph_export_does() {
    let mut server = served();
    let queried = tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "b3", "depth": 0}),
    );
    let verify = &queried["nodes"][0]["verify"];
    let exported = tool(
        &mut server,
        "specforge.export",
        json!({"format": "context"}),
    );
    let in_context = exported["nodes"]
        .as_array()
        .and_then(|all| all.iter().find(|e| e["id"] == "b3"))
        .map(|e| e["verify"].clone());
    let inspected = tool(&mut server, "specforge.inspect", json!({"entity_id": "b3"}));
    let payload = prompt_payload(&mut server, "b3");
    assert_eq!(*verify, inspected["verify_declarations"], "{inspected}");
    assert_eq!(*verify, payload["verify_expectations"], "{payload}");
    assert_eq!(Some(verify.clone()), in_context, "{exported}");
}

#[specforge_test(
    behavior = "inspect_entity_facts",
    verify = "inspect writes a block list as an array of objects, and specforge.edit appends an item to it"
)]
#[ignore = "K3 T10"]
fn mcp_inspect_shows_blocks_and_edits_them() {
    use specforge_project::testing::{TYPED_EXTENSION, typed_kinds};
    let mut server = TestProject::new()
        .file(
            "a.spec",
            "milestone v1.0 \"One\" { }\nphase 02 \"Two\" { milestone v1.0 }\nphase 03 \"Three\" { milestone v1.0 }\n\
             plan 03-02 \"Plan\" {\n  phase v1.0/03\n  files_modified [\"src/a.ts\"]\n}\n\
             summary 03-02-S \"S\" {\n  plan v1.0/03/03-02\n  requires [{ phase 02 provides \"x\" }]\n}\n",
        )
        .serve(&[TestExtension::named(TYPED_EXTENSION).declaring(typed_kinds)]);

    let inspected = tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "03-02-S"}),
    );
    assert_eq!(
        inspected["fields"]["requires"],
        json!([{"phase": "02", "provides": "x"}]),
        "{inspected}"
    );

    let reply = tool(
        &mut server,
        "specforge.edit",
        json!({"edits": [{"op": "append", "entity_id": "03-02-S", "field": "requires",
            "value": {"phase": "03", "provides": "y"}}], "dry_run": true}),
    );
    let diff = reply["diff"].as_str().unwrap_or_default();
    assert!(diff.contains("+    { phase 03 provides \"y\" }"), "{reply}");
}
