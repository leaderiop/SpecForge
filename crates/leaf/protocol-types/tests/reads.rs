//! File reads on the wire (protocol 1.11.0, ADR 0293): the declaration of `reads` and `reads_tests`, the `files` of an
//! input, and that none of them is written while absent.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::{Value, json};
use specforge_protocol_types::{
    CommandDescriptor, CommandInput, CompilerPassDescriptor, DeclaredCategory, DescribeResponse,
    ExtensionDeclaration, FileFact, FileFacts, FileRead, HandshakeResponse, PROTOCOL_VERSION,
    PassInput, UnknownKey,
};

fn round_trip<T>(value: &T) -> Value
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let written = serde_json::to_value(value).unwrap();
    let read: T = serde_json::from_value(written.clone()).unwrap();
    assert_eq!(&read, value);
    written
}

#[test]
fn file_read_round_trips_and_stays_off_the_wire_when_absent() {
    let read = FileRead {
        name: "modified".into(),
        field: Some("plan.files_modified".into()),
        text: true,
        ..Default::default()
    };
    assert_eq!(
        round_trip(&read),
        json!({"name": "modified", "field": "plan.files_modified", "text": true})
    );
    let glob = FileRead {
        name: "sources".into(),
        glob: Some("src/**/*.rs".into()),
        ..Default::default()
    };
    assert_eq!(
        round_trip(&glob),
        json!({"name": "sources", "glob": "src/**/*.rs"})
    );

    // An input with no reads writes no `files`; one with reads writes the facts under the read's name.
    assert!(
        serde_json::to_value(PassInput::default())
            .unwrap()
            .get("files")
            .is_none()
    );
    assert!(
        serde_json::to_value(CommandInput::<()>::default())
            .unwrap()
            .get("files")
            .is_none()
    );
    let mut input = PassInput::default();
    input.files.insert(
        "modified".into(),
        FileFacts {
            facts: vec![FileFact {
                path: "src/lib.rs".into(),
                entity: Some("pl1".into()),
                exists: true,
                bytes: Some(5),
                lines: Some(1),
                digest: Some("sha256:ab".into()),
                text: Some("hello".into()),
                error: None,
            }],
            truncated: false,
        },
    );
    let written = round_trip(&input);
    assert_eq!(
        written["files"]["modified"],
        json!({"facts": [{"path": "src/lib.rs", "entity": "pl1", "exists": true, "bytes": 5, "lines": 1,
                          "digest": "sha256:ab", "text": "hello"}]})
    );
}

#[test]
fn a_pass_declaring_reads_and_reads_tests_round_trips() {
    let pass = CompilerPassDescriptor {
        name: "audit".into(),
        phase: Some("check".into()),
        reads: vec![FileRead {
            name: "files".into(),
            glob: Some("src/**".into()),
            ..Default::default()
        }],
        reads_tests: true,
        ..Default::default()
    };
    let written = round_trip(&pass);
    assert_eq!(written["reads_tests"], true);
    assert_eq!(written["reads"][0]["glob"], "src/**");
    // A pass that declares neither writes neither.
    let plain = serde_json::to_value(CompilerPassDescriptor::default()).unwrap();
    assert!(plain.get("reads").is_none() && plain.get("reads_tests").is_none());
}

#[test]
fn a_command_declaring_reads_round_trips() {
    let command = CommandDescriptor {
        id: "verify".into(),
        reads: vec![FileRead {
            name: "plans".into(),
            field: Some("plan.files_modified".into()),
            base: Some("spec".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let written = round_trip(&command);
    assert_eq!(written["reads"][0]["base"], "spec");
    assert!(
        serde_json::to_value(CommandDescriptor::default())
            .unwrap()
            .get("reads")
            .is_none()
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a describe item key the protocol does not define produces W138"
)]
fn an_unknown_key_in_a_read_is_w138() {
    let handshake = HandshakeResponse {
        protocol_version: PROTOCOL_VERSION.to_string(),
        name: "@acme/reads".to_string(),
        version: "0.1.0".to_string(),
        ..Default::default()
    };
    let mut unknown = Vec::new();
    ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            let items = match category {
                DeclaredCategory::Passes => json!([
                    {"name": "audit", "reads": [{"name": "files", "glob": "src/**", "txet": true}]},
                ]),
                DeclaredCategory::Surfaces => json!([{"commands": [
                    {"id": "go", "title": "Go", "description": "Go", "export": "cmd__go",
                     "reads": [{"name": "files", "field": "plan.files", "bsae": "spec"}]},
                ]}]),
                _ => json!([]),
            };
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items,
            })
        },
        |key| unknown.push(key),
    )
    .unwrap();
    let keys: Vec<(&str, &str)> = unknown
        .iter()
        .map(|UnknownKey { category, key, .. }| (*category, key.as_str()))
        .collect();
    assert_eq!(
        keys,
        [("surfaces", "bsae"), ("passes", "txet")],
        "{unknown:?}"
    );
}
