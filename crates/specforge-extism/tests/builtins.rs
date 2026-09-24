use specforge_extism::{ExtismRuntime, builtins};
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

#[test]
fn load_all_builtins() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).expect("failed to load builtins");
}

#[test]
fn product_handshake() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).unwrap();

    let result = runtime.call_export("@specforge/product", "__handshake", &[]);
    let WasmCallResult::Ok(bytes) = result else {
        panic!("handshake failed: {:?}", result);
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["name"], "@specforge/product");
}

#[test]
fn software_describe_entities() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).unwrap();

    let input = br#"{"category":"entities"}"#;
    let result = runtime.call_export("@specforge/software", "__describe", input);
    let WasmCallResult::Ok(bytes) = result else {
        panic!("describe failed: {:?}", result);
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["category"], "entities");
    assert!(!value["items"].as_array().unwrap().is_empty());
}

#[test]
fn governance_handshake() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).unwrap();

    let result = runtime.call_export("@specforge/governance", "__handshake", &[]);
    let WasmCallResult::Ok(bytes) = result else {
        panic!("handshake failed: {:?}", result);
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["name"], "@specforge/governance");
}

#[test]
fn formal_describe_edges() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).unwrap();

    let input = br#"{"category":"edges"}"#;
    let result = runtime.call_export("@specforge/formal", "__describe", input);
    let WasmCallResult::Ok(bytes) = result else {
        panic!("describe failed: {:?}", result);
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["category"], "edges");
}

// SDK adoption: the formal wasm twin is authored with the extension SDK —
// its handshake must still report the extracted builtin contract.
#[test]
fn formal_handshake_from_sdk_authored_twin() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).unwrap();

    let result = runtime.call_export("@specforge/formal", "__handshake", &[]);
    let WasmCallResult::Ok(bytes) = result else {
        panic!("handshake failed: {:?}", result);
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["name"], "@specforge/formal");
    assert_eq!(value["version"], "1.0.0");
    assert_eq!(value["contribution_flags"]["entities"], true);
    assert_eq!(value["contribution_flags"]["validators"], true);
    let peers = value["peer_dependencies"].as_array().unwrap();
    assert!(
        peers.iter().any(|p| p["name"] == "@specforge/software"),
        "peer dependency on @specforge/software must survive the migration"
    );
}

// SDK migration: every builtin twin is now SDK-authored; pin each handshake
// contract (peer dependencies, sandbox policy, contribution flags).
#[test]
fn builtin_handshakes_survive_sdk_migration() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).unwrap();

    let expectations = [
        (
            "@specforge/software",
            &[("@specforge/product", false)][..],
            true,
        ),
        (
            "@specforge/governance",
            &[("@specforge/software", false), ("@specforge/product", true)][..],
            false,
        ),
        ("@specforge/product", &[][..], false),
    ];

    for (name, peers, has_sandbox) in expectations {
        let result = runtime.call_export(name, "__handshake", &[]);
        let WasmCallResult::Ok(bytes) = result else {
            panic!("handshake failed for {name}");
        };
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["name"], name, "handshake name for {name}");
        assert_eq!(
            value["contribution_flags"]["entities"], true,
            "{name}: entities flag"
        );
        assert_eq!(
            value["contribution_flags"]["validators"], true,
            "{name}: validators flag"
        );
        let peer_deps = value["peer_dependencies"].as_array().unwrap();
        assert_eq!(
            peer_deps.len(),
            peers.len(),
            "{name}: peer dependency count ({peer_deps:?})"
        );
        for (peer_name, optional) in peers {
            let found = peer_deps
                .iter()
                .find(|p| p["name"] == *peer_name)
                .unwrap_or_else(|| panic!("{name}: missing peer {peer_name}"));
            assert_eq!(found["optional"], *optional, "{name} peer {peer_name}");
        }
        assert_eq!(
            value["sandbox_policy"].is_null(),
            !has_sandbox,
            "{name}: sandbox policy presence"
        );
    }
}
