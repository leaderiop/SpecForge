//! The in-process adapter of the `WasmRuntime` port keeps the contract the
//! component runtime keeps (`specforge_wasm::testing::assert_runtime_contract`,
//! run over the component runtime in `specforge-component`'s tests).

use specforge_extension_sdk::prelude::*;
use specforge_wasm::testing::{InProcessRuntime, assert_runtime_contract};

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
