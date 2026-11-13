//! An MCP resource names an entity by its path, percent-encoded in the one `{entity_id}` segment (ADR 0284).

use crate::support::*;
use serde_json::{Value, json};
use specforge_project::testing::{SCOPED_EXTENSION, scoped_kinds};
use specforge_test::prelude::*;

fn serving(sources: &str) -> Served {
    TestProject::new()
        .file("a.spec", sources)
        .serve(&[TestExtension::named(SCOPED_EXTENSION).declaring(scoped_kinds)])
}

const SCOPED: &str = "milestone v1.0 \"One\" { }\nphase 03 \"Three\" { milestone v1.0 }\n\
                      decision D-01 \"Choice\" { phase v1.0/03 }\n";

fn node_ids(entity: &Value) -> Vec<&str> {
    entity["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect()
}

#[specforge_test(
    behavior = "expose_entity_as_mcp_resource",
    verify = "an entity resource names an entity by its percent-encoded path, a rooted path or a unique short form"
)]
fn an_mcp_uri_names_an_entity_by_its_encoded_path() {
    let mut server = serving(SCOPED);
    let (content, entity) = resource(&mut server, "specforge://graph/v1.0%2F03%2FD-01");
    assert_eq!(content["uri"], "specforge://graph/v1.0%2F03%2FD-01");
    assert!(node_ids(&entity).contains(&"v1.0/03/D-01"), "{entity}");
    // A short form that names one entity reads it.
    let (_, short) = resource(&mut server, "specforge://graph/D-01");
    assert!(node_ids(&short).contains(&"v1.0/03/D-01"), "{short}");

    // A rooted path names the project's entity.
    let mut server = serving(&format!("{SCOPED}decision D-01 \"Project\" {{ }}\n"));
    let (_, project) = resource(&mut server, "specforge://graph/%2FD-01");
    assert!(node_ids(&project).contains(&"D-01"), "{project}");

    // A short form naming two entities is refused, naming both.
    let mut server = serving(&format!(
        "{SCOPED}phase 04 \"Four\" {{ milestone v1.0 }}\ndecision D-01 \"Other\" {{ phase v1.0/04 }}\n"
    ));
    let refused = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://graph/D-01"}),
    );
    let message = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("v1.0/03/D-01") && message.contains("v1.0/04/D-01"),
        "{refused}"
    );
}

#[test]
fn a_path_id_written_as_is_is_refused_as_more_than_one_segment() {
    let mut server = serving(SCOPED);
    for uri in [
        "specforge://graph/v1.0/03/D-01",
        "specforge://context/v1.0/03/D-01",
    ] {
        let refused = call(&mut server, "resources/read", json!({"uri": uri}));
        let error = &refused["error"];
        assert_eq!(error["code"], json!(-32602), "{uri}: {refused}");
        assert_eq!(error["data"]["argument"], "entity_id", "{uri}: {refused}");
        let message = error["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("is more than one path segment"),
            "{uri}: {refused}"
        );
    }
}
