//! Hot-reload primitives (hardening-plan H1 / R-5): reload swaps the plugin,
//! unload drops it, loaded_names reflects the live set.

use specforge_extism::{ExtismRuntime, builtins};
use specforge_wasm::runtime::WasmCallResult;
use specforge_wasm::runtime::WasmRuntime;

#[test]
fn reload_unload_and_names_roundtrip() {
    let runtime = ExtismRuntime::new();
    builtins::load_builtins(&runtime).expect("blobs load");

    assert!(
        runtime
            .loaded_names()
            .contains(&"@specforge/product".to_string())
    );

    // Reload = swap the entry with the same bytes; the plugin stays callable.
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/product/wasm/specforge_ext_product.wasm"
    ))
    .unwrap();
    runtime
        .reload_module_bytes("@specforge/product", &bytes)
        .expect("reload succeeds");
    let result = runtime.call_export("@specforge/product", "__handshake", &[]);
    assert!(matches!(result, WasmCallResult::Ok(_)));

    // Unload drops it: further calls trap with extension_not_found.
    assert!(runtime.unload("@specforge/product"));
    assert!(!runtime.unload("@specforge/product"));
    let result = runtime.call_export("@specforge/product", "__handshake", &[]);
    match result {
        WasmCallResult::Trap(t) => assert_eq!(t.kind, "extension_not_found"),
        WasmCallResult::Ok(_) => panic!("unloaded extension must not answer calls"),
    }
    assert!(
        !runtime
            .loaded_names()
            .contains(&"@specforge/product".to_string())
    );
}
