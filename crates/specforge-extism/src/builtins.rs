use crate::ExtismRuntime;

static PRODUCT_WASM: &[u8] = include_bytes!(
    "../../../extensions/product/wasm/specforge_ext_product.wasm"
);
static SOFTWARE_WASM: &[u8] = include_bytes!(
    "../../../extensions/software/wasm/specforge_ext_software.wasm"
);
static GOVERNANCE_WASM: &[u8] = include_bytes!(
    "../../../extensions/governance/wasm/specforge_ext_governance.wasm"
);
static FORMAL_WASM: &[u8] = include_bytes!(
    "../../../extensions/formal/wasm/specforge_ext_formal.wasm"
);

pub const BUILTIN_EXTENSIONS: &[(&str, &[u8])] = &[
    ("@specforge/product", PRODUCT_WASM),
    ("@specforge/software", SOFTWARE_WASM),
    ("@specforge/governance", GOVERNANCE_WASM),
    ("@specforge/formal", FORMAL_WASM),
];

/// Load only the builtin Wasm extensions whose names appear in `requested`.
///
/// Non-builtin names (e.g. custom `.wasm` paths) are silently skipped.
pub fn load_builtins_for(runtime: &ExtismRuntime, requested: &[String]) -> Result<(), String> {
    for (name, wasm_bytes) in BUILTIN_EXTENSIONS {
        if requested.iter().any(|r| r == *name) {
            runtime.load_module_bytes(name, wasm_bytes)?;
        }
    }
    Ok(())
}

/// Load all builtin Wasm extensions. Used by tests only.
pub fn load_builtins(runtime: &ExtismRuntime) -> Result<(), String> {
    for (name, wasm_bytes) in BUILTIN_EXTENSIONS {
        runtime.load_module_bytes(name, wasm_bytes)?;
    }
    Ok(())
}
