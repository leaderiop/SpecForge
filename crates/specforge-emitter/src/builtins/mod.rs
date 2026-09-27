mod formal;
mod governance;
mod product;
mod rust;
mod software;
mod typescript;

pub use formal::FormalExtension;
pub use governance::GovernanceExtension;
pub use product::ProductExtension;
pub use rust::RustExtension;
pub use software::SoftwareExtension;
pub use typescript::TypeScriptExtension;

use specforge_wasm::BuiltinRuntime;

/// All known builtin extension names.
pub const KNOWN_BUILTINS: &[&str] = &[
    "@specforge/product",
    "@specforge/software",
    "@specforge/governance",
    "@specforge/formal",
    "@specforge/rust",
    "@specforge/typescript",
];

/// Create a `BuiltinRuntime` containing only the requested extensions.
///
/// MIGRATION ORACLE — `#[doc(hidden)]`, retained solely for the Phase 0
/// parity harness (`crates/specforge-extism/tests/parity.rs`) until the
/// native tier is deleted in Phase 7 of `.plugin/migration-plan.md`.
/// Production code must not call this: all surfaces build runtimes via
/// `specforge_extism::project_runtime`.
#[doc(hidden)]
pub fn runtime_for_extensions(names: &[String]) -> BuiltinRuntime {
    let mut runtime = BuiltinRuntime::new();
    for name in names {
        match name.as_str() {
            "@specforge/product" => {
                runtime = runtime.with_extension(name, Box::new(ProductExtension));
            }
            "@specforge/software" => {
                runtime = runtime.with_extension(name, Box::new(SoftwareExtension));
            }
            "@specforge/governance" => {
                runtime = runtime.with_extension(name, Box::new(GovernanceExtension));
            }
            "@specforge/formal" => {
                runtime = runtime.with_extension(name, Box::new(FormalExtension));
            }
            "@specforge/rust" => {
                runtime = runtime.with_extension(name, Box::new(RustExtension));
            }
            "@specforge/typescript" => {
                runtime = runtime.with_extension(name, Box::new(TypeScriptExtension));
            }
            _ => {}
        }
    }
    runtime
}
