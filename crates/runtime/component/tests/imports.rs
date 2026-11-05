//! A component's imports are read before it is instantiated (ADR 0326 D1a): a host port this specforge lacks is
//! refused naming it, and so is an interface that is neither WASI nor a specforge host port.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_component::ComponentRuntime;
use specforge_component::fixtures::{importing, importing_empty};
use specforge_protocol_types::PROTOCOL_VERSION;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::runtime::WasmRuntime;

const NEWER_PORT: &str = "specforge:host/http@9.9.0";

#[specforge_test(
    behavior = "load_wasm_module",
    verify = "a component importing a host port this specforge does not provide is refused with E088 before it is instantiated, naming the port"
)]
fn a_component_importing_a_host_port_this_specforge_lacks_is_refused_naming_it() {
    let runtime = ComponentRuntime::new();

    let error = runtime
        .load(
            "@acme/newer",
            &importing(&[NEWER_PORT], PROTOCOL_VERSION),
            specforge_wasm::Provenance::third_party(&importing(&[NEWER_PORT], PROTOCOL_VERSION)),
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains(NEWER_PORT), "{error}");
    assert!(error.contains("does not provide"), "{error}");
    assert!(!runtime.loaded_names().contains(&"@acme/newer".to_string()));
}

#[specforge_test(
    behavior = "load_wasm_module",
    verify = "a component importing a host port this specforge does not provide is refused with E088 before it is instantiated, naming the port"
)]
fn a_component_importing_an_empty_host_port_instance_is_refused_too() {
    let runtime = ComponentRuntime::new();

    let loaded = runtime.load(
        "@acme/empty",
        &importing_empty(NEWER_PORT),
        specforge_wasm::Provenance::third_party(&importing_empty(NEWER_PORT)),
    );

    assert!(loaded.is_err(), "an empty instance of a host port loads");
}

#[specforge_test(
    behavior = "load_wasm_module",
    verify = "a component importing an interface that is neither WASI nor a specforge host port is refused with E028 naming it"
)]
fn a_component_importing_a_foreign_interface_is_refused_naming_it() {
    let runtime = ComponentRuntime::new();

    let error = runtime
        .load(
            "@acme/foreign",
            &importing(&["acme:x/y@1.0.0"], PROTOCOL_VERSION),
            specforge_wasm::Provenance::third_party(&importing(
                &["acme:x/y@1.0.0"],
                PROTOCOL_VERSION,
            )),
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("acme:x/y@1.0.0"), "{error}");
    assert!(
        error.contains("neither WASI nor a specforge host port"),
        "{error}"
    );
}
