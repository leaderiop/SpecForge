//! Inference gaps over a served project: the public item facts the graph holds, covered by what cites them or
//! anchors them (ADR 0199).

use crate::support::*;
use serde_json::{Value, json};
use specforge_project::testing::{item, naming, stored_items};
use specforge_test::prelude::*;

const SPEC: &str = concat!(
    "behavior compiled_project \"C\" {\n    contract \"c\"\n}\n",
    "behavior compile_project \"P\" {\n    contract \"c\"\n}\n",
    "behavior helper \"H\" {\n    contract \"c\"\n}\n",
);

const ANCHORS: &str = r#"{"version":1,"anchors":[
{"entity_id":"compiled_project","file":"src/lib.rs","line":1,"symbol_name":"CompiledProject","item_kind":"struct","scanner":"rust-source"},
{"entity_id":"compile_project","file":"src/lib.rs","line":2,"symbol_name":"compile_project","item_kind":"fn","scanner":"rust-source"}]}"#;

/// `src/lib.rs`'s two public items.
fn lib() -> Vec<Value> {
    vec![
        item(
            "rust:p::CompiledProject",
            "src/lib.rs",
            1,
            "struct",
            "public",
            "CompiledProject",
        ),
        item(
            "rust:p::compile_project",
            "src/lib.rs",
            2,
            "function",
            "public",
            "compile_project",
        ),
    ]
}

/// `src/other.rs`'s public `compile_project`.
fn other() -> Value {
    item(
        "rust:p::other::compile_project",
        "src/other.rs",
        1,
        "function",
        "public",
        "compile_project",
    )
}

/// The project of `spec` serving the item fixtures and the software kinds, its `rust-source` store holding `facts`,
/// with `anchors` as its anchors manifest.
fn project(spec: &str, anchors: Option<&str>, facts: Vec<Value>) -> TestProject {
    let mut project = TestProject::new().file("test.spec", spec);
    if let Some(anchors) = anchors {
        project = project.file("specforge-anchors.json", anchors);
    }
    if !facts.is_empty() {
        stored_items(project.root(), "rust-source", facts);
    }
    project
}

fn serve(project: TestProject) -> Served {
    project.serve(&[
        TestExtension::software(),
        TestExtension::items(),
        TestExtension::cites(),
    ])
}

fn served(anchors: Option<&str>, facts: Vec<Value>) -> Served {
    serve(project(SPEC, anchors, facts))
}

fn gaps(server: &mut Served) -> Value {
    let reply = call_tool(server, "specforge.infer_gaps", json!({}));
    serde_json::from_str(&tool_text(&reply)).unwrap()
}

/// `(file, name)` of every listed gap.
fn gap_items(server: &mut Served) -> Vec<(String, String)> {
    let gaps = gaps(server);
    gaps["by_directory"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["items"].as_array().unwrap())
        .map(|i| {
            (
                i["file"].as_str().unwrap().to_string(),
                i["name"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "an item an anchor names for a declared entity is covered"
)]
fn an_item_an_anchor_names_for_a_declared_entity_is_covered() {
    let mut server = served(Some(ANCHORS), lib());
    let items = gap_items(&mut server);
    assert!(
        !items.contains(&("src/lib.rs".into(), "CompiledProject".into())),
        "{items:?}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "an anchored entity covers only the items its anchors name"
)]
fn an_anchored_entity_covers_only_the_items_its_anchors_name() {
    let mut facts = lib();
    facts.push(other());
    let mut server = served(Some(ANCHORS), facts);
    let items = gap_items(&mut server);
    assert!(
        items.contains(&("src/other.rs".into(), "compile_project".into())),
        "{items:?}"
    );
    let document = gaps(&mut server);
    assert_eq!(document["covered_items"], 2, "{document}");
    assert_eq!(document["total_pub_items"], 3, "{document}");
    assert_eq!(document["collectors"], json!(["rust-source"]), "{document}");
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "an unusable anchors manifest refuses the gaps with E071"
)]
fn an_unusable_anchors_manifest_refuses_the_gaps() {
    let mut server = served(
        Some(r#"{"version":1,"anchors":[{"entity_id":"x"}]}"#),
        lib(),
    );
    let reply = call_tool(&mut server, "specforge.infer_gaps", json!({}));
    let error = crate::tool_errors::mcp_error(&reply);
    assert_eq!(error["code"], "schema_mismatch", "{error}");
    assert_eq!(error["diagnostic"]["code"], "E071", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .starts_with("failed to parse specforge-anchors.json: missing field `file`"),
        "{error}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "every item find_spec_for_source anchors to a declared entity is no gap"
)]
fn every_item_find_spec_for_source_anchors_to_a_declared_entity_is_no_gap() {
    let anchors = r#"{"version":1,"anchors":[
{"entity_id":"compiled_project","file":"./src/lib.rs","line":1,"symbol_name":"CompiledProject","item_kind":"struct","scanner":"s"},
{"entity_id":"compile_project","file":"src/lib.rs","line":2,"symbol_name":"compile_project","item_kind":"fn","scanner":"s"},
{"entity_id":"ghost","file":"src/lib.rs","line":3,"symbol_name":"Ghost","item_kind":"struct","scanner":"s"}]}"#;
    let mut facts = lib();
    facts.push(item(
        "rust:p::Ghost",
        "src/lib.rs",
        3,
        "struct",
        "public",
        "Ghost",
    ));
    let mut server = served(Some(anchors), facts);
    let reply = call_tool(
        &mut server,
        "specforge.find_spec_for_source",
        json!({"file_path": "src"}),
    );
    let anchored: Value = serde_json::from_str(&tool_text(&reply)).unwrap();
    let items = gap_items(&mut server);
    let mut declared = 0;
    for entity in anchored["entities"].as_array().unwrap() {
        let item = (
            "src/lib.rs".to_string(),
            entity["symbol_name"].as_str().unwrap().to_string(),
        );
        if entity.get("kind").is_some() {
            declared += 1;
            assert!(!items.contains(&item), "{item:?} in {items:?}");
        } else {
            assert!(items.contains(&item), "{item:?} not in {items:?}");
        }
    }
    assert_eq!(declared, 2, "{anchored}");
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "an entity that only shares an item's name does not cover it"
)]
fn a_same_named_entity_does_not_cover_an_item() {
    let mut facts = lib();
    facts.push(item(
        "rust:p::util::helper",
        "src/util.rs",
        1,
        "function",
        "public",
        "helper",
    ));
    let mut server = served(Some(ANCHORS), facts);
    assert_eq!(
        gap_items(&mut server),
        [("src/util.rs".to_string(), "helper".to_string())],
        "behavior helper shares the item's name and cites nothing"
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "an item an authored entity cites is covered"
)]
fn an_item_an_authored_entity_cites_is_covered() {
    let spec = format!("{SPEC}task wire \"W\" {{\n    items [\"rust:p::net::Wire\"]\n}}\n");
    let facts = vec![
        item(
            "rust:p::net::Wire",
            "src/net.rs",
            1,
            "struct",
            "public",
            "Wire",
        ),
        item(
            "rust:p::net::Plug",
            "src/net.rs",
            2,
            "struct",
            "public",
            "Plug",
        ),
    ];
    let mut server = serve(project(&spec, None, facts));
    assert_eq!(
        gap_items(&mut server),
        [("src/net.rs".to_string(), "Plug".to_string())]
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "an item that names an authored entity is covered"
)]
fn an_item_that_names_an_entity_is_covered() {
    let facts = vec![
        naming(
            item(
                "rust:p::helper",
                "src/lib.rs",
                1,
                "function",
                "public",
                "helper",
            ),
            &["helper"],
        ),
        item(
            "rust:p::other",
            "src/lib.rs",
            2,
            "function",
            "public",
            "other",
        ),
    ];
    let mut server = served(None, facts);
    assert_eq!(
        gap_items(&mut server),
        [("src/lib.rs".to_string(), "other".to_string())]
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "only public items are counted"
)]
fn an_item_no_other_package_may_name_is_no_gap() {
    let facts = vec![
        item(
            "rust:p::open",
            "src/lib.rs",
            1,
            "function",
            "public",
            "open",
        ),
        item(
            "rust:p::inner",
            "src/lib.rs",
            2,
            "function",
            "crate",
            "inner",
        ),
        item(
            "rust:p::hidden",
            "src/lib.rs",
            3,
            "function",
            "private",
            "hidden",
        ),
    ];
    let mut server = served(None, facts);
    assert_eq!(gaps(&mut server)["total_pub_items"], 1);
    assert_eq!(
        gap_items(&mut server),
        [("src/lib.rs".to_string(), "open".to_string())]
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "with no item facts, gaps count nothing and name no collector"
)]
fn with_no_item_facts_gaps_count_nothing() {
    let mut server = served(None, Vec::new());
    let document = gaps(&mut server);
    assert_eq!(
        document,
        json!({"item_facts": 0, "total_pub_items": 0, "covered_items": 0, "gap_count": 0, "approximate": false,
               "collectors": [], "by_directory": []})
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "the report is approximate when a fact store is not served"
)]
fn an_unserved_fact_store_makes_gaps_approximate() {
    let project = project(SPEC, None, lib());
    // A store no specforge-harness line records: W191, its facts unserved.
    let ledger = project.root().join("specforge-harness");
    std::fs::remove_file(&ledger).unwrap();
    let mut server = serve(project);
    let document = gaps(&mut server);
    assert_eq!(document["approximate"], true, "{document}");
    assert_eq!(document["total_pub_items"], 0, "{document}");
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "an anchor to an entity the graph does not declare covers nothing"
)]
fn an_anchor_to_an_entity_the_graph_does_not_declare_covers_nothing() {
    let anchors = r#"{"version":1,"anchors":[
{"entity_id":"ghost","file":"src/other.rs","line":1,"symbol_name":"Other","item_kind":"struct","scanner":"s"}]}"#;
    let facts = vec![item(
        "rust:p::Other",
        "src/other.rs",
        1,
        "struct",
        "public",
        "Other",
    )];
    let mut server = served(Some(anchors), facts);
    assert_eq!(
        gap_items(&mut server),
        [("src/other.rs".to_string(), "Other".to_string())]
    );
}

#[specforge_test(
    behavior = "provide_mcp_infer_gaps_tool",
    verify = "every item find_spec_for_source anchors to a declared entity is no gap"
)]
fn an_anchor_recorded_absolute_under_the_root_is_no_gap_where_find_spec_for_source_finds_it() {
    let project = project(SPEC, None, lib());
    let absolute = project.root().join("src/lib.rs");
    let anchors = ANCHORS.replace(
        "\"src/lib.rs\"",
        &format!("{:?}", absolute.to_str().unwrap()),
    );
    let mut server = serve(project.file("specforge-anchors.json", &anchors));
    let found = |server: &mut Served, path: &str| -> Value {
        let reply = call_tool(
            server,
            "specforge.find_spec_for_source",
            json!({"file_path": path}),
        );
        serde_json::from_str(&tool_text(&reply)).unwrap()
    };
    let file = found(&mut server, "src/lib.rs");
    assert_eq!(file["match_mode"], "exact", "{file}");
    assert_eq!(file["count"], 2, "{file}");
    assert_eq!(found(&mut server, "src")["match_mode"], "directory");
    assert_eq!(gap_items(&mut server), []);
}
