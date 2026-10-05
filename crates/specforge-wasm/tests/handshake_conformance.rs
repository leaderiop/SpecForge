//! Fixture conformance for the extension handshake/describe protocol
//! (audit C7-07): every builtin's pinned wire answers (`handshake.json` and
//! `describe_*.json` under `crates/specforge-component/tests/declarations/`,
//! the exact bytes each vendored blob answers) must deserialize against the
//! shared Rust protocol types, and the handshake's critical fields must be
//! present — a truncated handshake now FAILS deserialization instead of
//! silently yielding empty contribution flags / no peer dependencies / no
//! sandbox limits.

use std::fs;
use std::path::{Path, PathBuf};

use specforge_protocol_types::{ContributionFlags, DescribeResponse, HandshakeResponse};

/// Handshake fields the host requires on the wire. Drift between this list
/// and `HandshakeResponse`'s required fields fails the conformance test.
const REQUIRED_HANDSHAKE_KEYS: &[&str] = &[
    "protocol_version",
    "name",
    "version",
    "contribution_flags",
    "peer_dependencies",
    "sandbox_policy",
];

fn declarations_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../specforge-component/tests/declarations")
}

/// Every pinned declaration (one directory per builtin, and greet).
fn extension_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(declarations_dir())
        .expect("the pinned declarations exist")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("handshake.json").is_file())
        .collect();
    dirs.sort();
    assert!(
        dirs.len() >= 10,
        "expected every builtin's pinned declaration"
    );
    dirs
}

#[test]
fn every_builtin_handshake_fixture_parses_against_protocol_types() {
    for dir in extension_dirs() {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let path = dir.join("handshake.json");
        let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("declarations/{name}/handshake.json missing or unreadable: {e}")
        });

        let value: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("declarations/{name}/handshake.json is not JSON: {e}"));

        let missing: Vec<&str> = REQUIRED_HANDSHAKE_KEYS
            .iter()
            .copied()
            .filter(|key| !value.get(key).is_some())
            .collect();
        assert!(
            missing.is_empty(),
            "declarations/{name}/handshake.json drifted: missing required handshake field(s) \
             {missing:?} — the host now REJECTS handshakes without them (C7-07). \
             Fix the fixture (it is the source of truth the guest blob embeds)."
        );

        let response: HandshakeResponse = serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!(
                "declarations/{name}/handshake.json no longer deserializes as \
                     HandshakeResponse: {e}"
            )
        });
        assert!(
            !response.protocol_version.is_empty() && !response.name.is_empty(),
            "declarations/{name}/handshake.json has empty protocol_version/name"
        );
    }
}

#[test]
fn every_builtin_describe_fixture_parses_against_protocol_types() {
    for dir in extension_dirs() {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let mut describe_fixtures: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("declarations/{name} unreadable: {e}"))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .is_some_and(|f| f.to_string_lossy().starts_with("describe_"))
            })
            .collect();
        describe_fixtures.sort();
        assert!(
            !describe_fixtures.is_empty(),
            "declarations/{name} has no describe_*.json fixtures"
        );

        for path in describe_fixtures {
            let file = path.file_name().unwrap().to_string_lossy().to_string();
            let raw = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("declarations/{name}/{file} unreadable: {e}"));
            let response: DescribeResponse = serde_json::from_str(&raw).unwrap_or_else(|e| {
                panic!("declarations/{name}/{file} no longer deserializes as DescribeResponse: {e}")
            });
            // `items` is a raw JSON array of typed descriptors.
            assert!(
                response.items.is_array(),
                "declarations/{name}/{file} items must be a JSON array"
            );
        }
    }
}

#[test]
fn truncated_handshake_missing_contribution_flags_fails_deserialization() {
    let dir = extension_dirs().remove(0);
    let raw = fs::read_to_string(dir.join("handshake.json")).expect("fixture exists");
    let mut value: serde_json::Value = serde_json::from_str(&raw).expect("fixture is JSON");
    value
        .as_object_mut()
        .expect("handshake is an object")
        .remove("contribution_flags");

    let truncated = serde_json::to_string(&value).expect("re-serialize");
    let err = serde_json::from_str::<HandshakeResponse>(&truncated)
        .expect_err("handshake missing contribution_flags must FAIL deserialization (C7-07)");
    assert!(
        err.to_string().contains("contribution_flags"),
        "deserialization error should name the missing critical field, got: {err}"
    );
}

#[test]
fn every_contribution_flag_defaults_false_when_absent() {
    // Per-flag `default` stays: individual flags are genuinely optional
    // (absent = extension does not contribute that category). Only the
    // handshake envelope fields became required.
    let flags: ContributionFlags =
        serde_json::from_str("{}").expect("empty object deserializes to all-false flags");
    assert!(!flags.entities && !flags.analyzers && !flags.validators);
}

#[test]
fn handshake_with_all_fields_round_trips() {
    let raw = r#"{
        "protocol_version": "1.0.0",
        "name": "@acme/test",
        "version": "0.1.0",
        "contribution_flags": { "entities": true },
        "peer_dependencies": [],
        "sandbox_policy": { "max_execution_ms": 5000 }
    }"#;
    let response: HandshakeResponse = serde_json::from_str(raw).expect("full handshake parses");
    assert_eq!(response.name, "@acme/test");
    assert!(response.contribution_flags.entities);
    assert_eq!(
        response.sandbox_policy.and_then(|p| p.max_execution_ms),
        Some(5000)
    );
}
