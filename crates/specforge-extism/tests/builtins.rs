use specforge_extism::{builtins, ExtismRuntime};
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
    assert!(value["items"].as_array().unwrap().len() > 0);
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
