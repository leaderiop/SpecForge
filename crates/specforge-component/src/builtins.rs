#[cfg(feature = "testing")]
use crate::ComponentRuntime;
#[cfg(feature = "testing")]
use specforge_common::ExtensionEntry;

static PRODUCT_WASM: &[u8] =
    include_bytes!("../../../extensions/product/wasm/specforge_ext_product.wasm");
static SOFTWARE_WASM: &[u8] =
    include_bytes!("../../../extensions/software/wasm/specforge_ext_software.wasm");
static GOVERNANCE_WASM: &[u8] =
    include_bytes!("../../../extensions/governance/wasm/specforge_ext_governance.wasm");
static FORMAL_WASM: &[u8] =
    include_bytes!("../../../extensions/formal/wasm/specforge_ext_formal.wasm");
static TESTING_WASM: &[u8] =
    include_bytes!("../../../extensions/testing/wasm/specforge_ext_testing.wasm");
static CARGO_TEST_WASM: &[u8] =
    include_bytes!("../../../extensions/cargo-test/wasm/specforge_ext_cargo_test.wasm");
static VITEST_WASM: &[u8] =
    include_bytes!("../../../extensions/vitest/wasm/specforge_ext_vitest.wasm");
static RUST_WASM: &[u8] = include_bytes!("../../../extensions/rust/wasm/specforge_ext_rust.wasm");
static TYPESCRIPT_WASM: &[u8] =
    include_bytes!("../../../extensions/typescript/wasm/specforge_ext_typescript.wasm");

pub const BUILTIN_EXTENSIONS: &[(&str, &[u8])] = &[
    ("@specforge/product", PRODUCT_WASM),
    ("@specforge/software", SOFTWARE_WASM),
    ("@specforge/governance", GOVERNANCE_WASM),
    ("@specforge/formal", FORMAL_WASM),
    ("@specforge/testing", TESTING_WASM),
    ("@specforge/cargo-test", CARGO_TEST_WASM),
    ("@specforge/vitest", VITEST_WASM),
    ("@specforge/rust", RUST_WASM),
    ("@specforge/typescript", TYPESCRIPT_WASM),
];

/// Test support: puts the builtin Wasm extensions `requested`
/// (`specforge.json` entries, read by [`ExtensionEntry`]: a legacy
/// `name@version` names the builtin too) names into a runtime outside the
/// extension load (`Installed::load`).
///
/// Other entries (installed extensions, `.wasm` files) are skipped.
#[cfg(feature = "testing")]
pub fn load_builtins_for(runtime: &ComponentRuntime, requested: &[String]) -> Result<(), String> {
    for (name, wasm_bytes) in BUILTIN_EXTENSIONS {
        if requested
            .iter()
            .any(|entry| ExtensionEntry::parse(entry) == ExtensionEntry::Named(name))
        {
            runtime.load_module_bytes(name, wasm_bytes)?;
        }
    }
    Ok(())
}

/// Test support: puts all builtin Wasm extensions into a runtime outside the
/// extension load (`Installed::load`).
#[cfg(feature = "testing")]
pub fn load_builtins(runtime: &ComponentRuntime) -> Result<(), String> {
    for (name, wasm_bytes) in BUILTIN_EXTENSIONS {
        runtime.load_module_bytes(name, wasm_bytes)?;
    }
    Ok(())
}
