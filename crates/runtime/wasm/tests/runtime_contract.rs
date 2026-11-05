//! The in-process adapter of the `WasmRuntime` port keeps the contract the
//! component runtime keeps (`specforge_wasm::testing::assert_runtime_contract`,
//! run over the component runtime in `specforge-component`'s tests).

use specforge_extension_sdk::prelude::*;
use specforge_wasm::testing::{InProcessRuntime, assert_module_contract, assert_runtime_contract};
use specforge_wasm::{Provenance, WasmRuntime};

/// An extension whose `crash` command panics.
fn crashing() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@contract/crash", "1.0.0"));
    c.command("crash", |cmd| {
        cmd.title("Crash")
            .description("Panics")
            .handler(|_| panic!("the command crashed"));
    });
    c
}

#[specforge_test_macros::test(
    behavior = "call_extension_exports",
    verify = "both runtimes report an unknown extension, an unrouted export, a guest error and a guest panic as traps"
)]
#[specforge_test_macros::test(port = "WasmRuntime", verify = "WasmRuntime contract is satisfied")]
fn the_in_process_runtime_keeps_the_runtime_contract() {
    let runtime = InProcessRuntime::new().with(crashing);
    assert_runtime_contract(&runtime, "@contract/crash", "cmd__crash");
}

#[test]
fn the_in_process_runtime_keeps_the_module_contract() {
    let bytes = b"\0asm in-process contract";
    let runtime = InProcessRuntime::new().binary(bytes, crashing);

    assert_module_contract(&runtime, bytes, "@contract/crash");
}

#[test]
fn a_binary_is_served_under_the_name_it_is_loaded_as() {
    let runtime = InProcessRuntime::new()
        .binary(b"one", crashing)
        .with(|| ContributionsBuilder::new(ExtensionMeta::new("@contract/served", "1.0.0")));

    // Bytes it was not given are no component it serves, under any name.
    assert!(
        runtime
            .load(
                "@acme/other",
                b"two",
                specforge_wasm::Provenance::third_party(b"two")
            )
            .is_err()
    );
    // The bytes it was given are served under the name they are loaded as,
    // not the one they declare.
    runtime
        .load(
            "@acme/installed",
            b"one",
            specforge_wasm::Provenance::third_party(b"one"),
        )
        .unwrap();
    assert!(runtime.unload("@acme/installed"));
    // A name it serves loads as it is, whatever the bytes.
    runtime
        .load(
            "@contract/served",
            b"anything",
            specforge_wasm::Provenance::third_party(b"anything"),
        )
        .unwrap();
    assert!(runtime.unload("@contract/served"));
    assert!(!runtime.unload("@contract/served"));
}

#[test]
fn what_it_cannot_serve_loads_in_the_runtime_beside_it() {
    use specforge_wasm::protocol::load_declaration;
    use std::sync::Arc;

    // The runtime beside it stands for the component runtime: it serves `@contract/crash` by its bytes.
    let beside = Arc::new(InProcessRuntime::new().binary(b"\0asm component", crashing));
    let runtime = InProcessRuntime::new()
        .with(|| ContributionsBuilder::new(ExtensionMeta::new("@contract/served", "1.0.0")))
        .binary(b"\0asm local", crashing)
        .beside(beside.clone());

    runtime
        .load(
            "@contract/served",
            b"anything",
            specforge_wasm::Provenance::third_party(b"anything"),
        )
        .unwrap();
    runtime
        .load(
            "@contract/crash",
            b"\0asm component",
            specforge_wasm::Provenance::third_party(b"\0asm component"),
        )
        .unwrap();
    // A load it refuses is refused beside it too.
    assert!(
        runtime
            .load(
                "@contract/none",
                b"\0asm other",
                specforge_wasm::Provenance::third_party(b"\0asm other")
            )
            .is_err()
    );

    let loaded = load_declaration(&runtime, "@contract/crash").unwrap();
    assert_eq!(loaded.declaration.name(), "@contract/crash");
    // Its calls are recorded here and answered beside it; a name served here is not sent there.
    load_declaration(&runtime, "@contract/served").unwrap();
    assert!(
        runtime
            .calls()
            .iter()
            .any(|c| c.extension == "@contract/crash")
    );
    assert!(
        beside
            .calls()
            .iter()
            .all(|c| c.extension == "@contract/crash")
    );
    assert!(runtime.rename("@contract/crash", "@contract/renamed"));
    assert!(load_declaration(&runtime, "@contract/renamed").is_ok());
    assert!(runtime.unload("@contract/renamed"));
    assert!(!beside.unload("@contract/renamed"), "unloaded beside it");

    // A reload it can serve itself takes the name back, and unloads it beside it.
    runtime
        .load(
            "@contract/crash",
            b"\0asm component",
            specforge_wasm::Provenance::third_party(b"\0asm component"),
        )
        .unwrap();
    runtime
        .load(
            "@contract/crash",
            b"\0asm local",
            specforge_wasm::Provenance::third_party(b"\0asm local"),
        )
        .unwrap();
    assert!(!beside.unload("@contract/crash"), "unloaded beside it");
    beside.clear_calls();
    load_declaration(&runtime, "@contract/crash").unwrap();
    assert!(beside.calls().is_empty(), "served in process");
}

#[test]
fn a_module_whose_bytes_spell_a_declaration_loads_under_any_name() {
    use specforge_extension_sdk::PeerDependency;
    use specforge_wasm::protocol::load_declaration;
    use specforge_wasm::testing::spelled_module;

    let peers = vec![PeerDependency {
        name: "@acme/b".to_string(),
        version: "^1.0".to_string(),
        optional: false,
    }];
    let bytes = spelled_module("@acme/a", "2.3.4", &peers);
    let runtime = InProcessRuntime::new();

    assert_eq!(
        runtime.declared_by("@acme/x", &bytes),
        Some(("2.3.4".to_string(), peers.clone()))
    );
    runtime
        .load(
            "@acme/x",
            &bytes,
            specforge_wasm::Provenance::third_party(&bytes),
        )
        .unwrap();
    let loaded = load_declaration(&runtime, "@acme/x").unwrap();
    assert_eq!(loaded.declaration.name(), "@acme/a");
    assert_eq!(loaded.declaration.version(), "2.3.4");
    assert_eq!(loaded.declaration.peers(), peers.as_slice());
    // Bytes that spell nothing are no component, and declare nothing.
    assert!(
        runtime
            .load(
                "@acme/y",
                b"\0asm other",
                specforge_wasm::Provenance::third_party(b"\0asm other")
            )
            .is_err()
    );
    assert_eq!(runtime.declared_by("@acme/y", b"\0asm other"), None);
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "an instance restarted after a trap still carries the extension's settings"
)]
fn a_restarted_instance_keeps_its_settings() {
    use serde_json::json;
    use specforge_wasm::{ExtensionCalls, ExtensionSettings};
    let runtime = InProcessRuntime::new().with(crashing);
    let settings = ExtensionSettings {
        values: json!({"level": 2}).as_object().unwrap().clone(),
        secrets: Vec::new(),
    };
    runtime.apply_settings("@contract/crash", settings.clone());
    let calls = ExtensionCalls::new(&runtime);
    // A guest panic traps; the next call is a fresh instance's and carries them.
    assert!(
        calls
            .call_mcp_tool("@contract/crash", "cmd__crash", &json!({}))
            .is_err()
    );
    let _ = calls.call_mcp_tool("@contract/crash", "mcp__again", &json!({"q": 1}));
    let last = runtime.calls().pop().unwrap();
    assert_eq!(last.input, json!({"q": 1, "settings": {"level": 2}}));
    assert_eq!(runtime.settings_of("@contract/crash"), Some(settings));
    assert_eq!(runtime.settings_of("@contract/other"), None);
    // Settings are spliced into an object input only, at its end.
    let calls = ExtensionCalls::new(&runtime);
    let _ = calls.call_mcp_tool("@contract/crash", "mcp__empty", &json!({}));
    assert_eq!(
        runtime.calls().pop().unwrap().input,
        json!({"settings": {"level": 2}})
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "every guest call but handshake and describe carries the extension's own settings and no other's"
)]
fn secret_presence_is_told_to_what_a_person_runs_not_to_a_pass() {
    use serde_json::json;
    use specforge_protocol_types::PassInput;
    use specforge_wasm::{ExtensionCalls, ExtensionSettings};
    let runtime = InProcessRuntime::new().with(crashing);
    runtime.apply_settings(
        "@contract/crash",
        ExtensionSettings {
            values: json!({"level": 2}).as_object().unwrap().clone(),
            // PATH is set in any test process.
            secrets: vec![("token".into(), "PATH".into())],
        },
    );
    let calls = ExtensionCalls::new(&runtime);
    let _ = calls.call_mcp_tool("@contract/crash", "mcp__t", &json!({}));
    assert_eq!(
        runtime.calls().pop().unwrap().input,
        json!({"settings": {"level": 2}, "secrets": {"token": true}})
    );
    let _ = calls.render_prompt(
        "@contract/crash",
        "prompt__t",
        &specforge_protocol_types::PromptInput::<specforge_protocol_types::RawGraph>::default(),
    );
    assert_eq!(
        runtime.calls().pop().unwrap().input["secrets"],
        json!({"token": true}),
        "a prompt is told too"
    );
    // A pass's result must be the same on every machine: it is told the settings, not whether a variable is set.
    let input: PassInput = serde_json::from_value(json!({"entities": []})).unwrap();
    let _ = calls.run_pass("@contract/crash", "audit", &ExtensionCalls::encode(&input));
    let carried = runtime.calls().pop().unwrap().input;
    assert_eq!(carried["settings"], json!({"level": 2}));
    assert!(carried.get("secrets").is_none(), "{carried}");
}

#[specforge_test_macros::test(
    behavior = "load_wasm_module",
    verify = "the runtime keeps the provenance a module was loaded with, across a rename, until it is unloaded"
)]
fn the_runtime_keeps_the_provenance_it_loaded_with() {
    let bytes = b"\0asm provenance";
    let runtime = InProcessRuntime::new().binary(bytes, crashing);
    assert_module_contract(&runtime, bytes, "@contract/crash");

    // Loaded again under the same name, the newer provenance replaces the older.
    runtime
        .load("@acme/x", bytes, Provenance::third_party(bytes))
        .unwrap();
    runtime
        .load("@acme/x", bytes, Provenance::builtin(bytes))
        .unwrap();
    assert_eq!(
        runtime.provenance("@acme/x"),
        Some(Provenance::builtin(bytes))
    );
    assert_eq!(runtime.provenance("@acme/never"), None);
}

#[specforge_test_macros::test(
    behavior = "grant_a_host_port_call",
    verify = "the in-process runtime answers a port call as the component runtime does"
)]
fn the_in_process_runtime_keeps_the_port_contract() {
    specforge_wasm::testing::assert_port_contract(
        &crate::calls::ports::runtime(),
        crate::calls::ports::EXT,
        "cmd__ports",
        &|| crate::calls::ports::lent().call,
        "https://example.test/a",
        "https://elsewhere.test/a",
    );
}
