//! A call's inputs on the wire (protocol 2.0.0, ADR 0337 D2): one tagged `inputs` list, the `files` of an input
//! (ADR 0293), and that none of them is written while absent.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::{Value, json};
use specforge_protocol_types::{
    CollectorDescriptor, CommandDescriptor, CommandInput, CompilerPassDescriptor, DeclaredCategory,
    DescribeResponse, ExtensionDeclaration, FileFact, FileFacts, FileRead, GitInput,
    HandshakeResponse, HookDescriptor, Input, Inputs, PROTOCOL_VERSION, PassInput,
    PromptDescriptor, UnknownKey,
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

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a call's inputs are one tagged list on the wire, and a descriptor that declares none writes no inputs"
)]
fn inputs_round_trip_as_one_tagged_list() {
    let pass = CompilerPassDescriptor {
        name: "audit".into(),
        phase: Some("check".into()),
        inputs: vec![
            Input::files(
                "sources",
                FileRead {
                    glob: Some("src/**/*.rs".into()),
                    ..Default::default()
                },
            ),
            Input::Tests,
        ],
        ..Default::default()
    };
    assert_eq!(
        round_trip(&pass)["inputs"],
        json!([{"input": "files", "name": "sources", "glob": "src/**/*.rs"}, {"input": "tests"}])
    );
    let hook = HookDescriptor {
        id: "guard".into(),
        export: "hook__guard".into(),
        inputs: vec![
            Input::Graph,
            Input::State,
            Input::git().status().when("workflow.git"),
            Input::setting("@gsd/security", "security.injection_blocking"),
        ],
        ..Default::default()
    };
    assert_eq!(
        round_trip(&hook)["inputs"],
        json!([{"input": "graph"}, {"input": "state"},
               {"input": "git", "status": true, "when": "workflow.git"},
               {"input": "setting", "extension": "@gsd/security", "name": "security.injection_blocking"}])
    );
    let prompt = PromptDescriptor {
        inputs: vec![Input::SlashCommands, Input::git()],
        ..Default::default()
    };
    assert_eq!(
        round_trip(&prompt)["inputs"],
        json!([{"input": "slash_commands"}, {"input": "git"}])
    );
    let command = CommandDescriptor {
        id: "verify".into(),
        inputs: vec![Input::files(
            "plans",
            FileRead {
                field: Some("plan.files_modified".into()),
                root: Some("spec".into()),
                ..Default::default()
            },
        )],
        ..Default::default()
    };
    assert_eq!(round_trip(&command)["inputs"][0]["root"], "spec");
    let collector = CollectorDescriptor {
        name: "node".into(),
        export: "collect__node".into(),
        inputs: vec![Input::files(
            "manifest",
            FileRead {
                glob: Some("package.json".into()),
                text: true,
                ..Default::default()
            },
        )],
        ..Default::default()
    };
    assert_eq!(
        round_trip(&collector)["inputs"],
        json!([{"input": "files", "name": "manifest", "glob": "package.json", "text": true}])
    );
    // A descriptor that declares none writes none.
    for plain in [
        serde_json::to_value(CompilerPassDescriptor::default()).unwrap(),
        serde_json::to_value(CommandDescriptor::default()).unwrap(),
        serde_json::to_value(CollectorDescriptor::default()).unwrap(),
        serde_json::to_value(PromptDescriptor::default()).unwrap(),
        serde_json::to_value(HookDescriptor::default()).unwrap(),
    ] {
        assert!(plain.get("inputs").is_none(), "{plain}");
    }
    // The accessor every host site reads them through.
    let inputs = Inputs(&hook.inputs);
    assert!(inputs.graph() && inputs.state() && !inputs.tests() && !inputs.slash_commands());
    assert_eq!(
        inputs.git(),
        Some(GitInput {
            status: true,
            when: Some("workflow.git")
        })
    );
    assert_eq!(
        inputs.settings().collect::<Vec<_>>(),
        [("@gsd/security", "security.injection_blocking")]
    );
    assert_eq!(Inputs(&pass.inputs).files()[0].name, "sources");
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a describe item key the protocol does not define produces W138"
)]
fn an_unknown_key_in_an_input_is_w138() {
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
                    {"name": "audit", "inputs": [{"input": "files", "name": "files", "glob": "src/**", "txet": true},
                                                {"input": "git", "status": true, "when": "x", "extension": "y",
                                                 "field": "a.b", "root": "spec", "text": true}]},
                ]),
                DeclaredCategory::Surfaces => json!([{"commands": [
                    {"id": "go", "title": "Go", "description": "Go", "export": "cmd__go",
                     "inputs": [{"input": "files", "name": "files", "field": "plan.files", "bsae": "spec"}],
                     "reads_state": true},
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
        [
            ("surfaces", "reads_state"),
            ("surfaces", "bsae"),
            ("passes", "txet")
        ],
        "{unknown:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a describe item key the protocol does not define produces W138"
)]
fn an_unknown_key_in_a_collectors_input_is_w138() {
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
                DeclaredCategory::Collectors => json!([
                    {"name": "node", "input_formats": [], "export": "collect__node",
                     "inputs": [{"input": "files", "name": "manifest", "glob": "package.json", "txet": true}]},
                ]),
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
    assert_eq!(keys, [("collectors", "txet")], "{unknown:?}");
}
