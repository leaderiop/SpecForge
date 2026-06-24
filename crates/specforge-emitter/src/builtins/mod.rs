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
/// Extension names not recognized as builtins are silently skipped (they may
/// be external Wasm extensions loaded separately).
pub fn runtime_for_extensions(names: &[String]) -> BuiltinRuntime {
    let mut runtime = BuiltinRuntime::new();
    for name in names {
        match name.as_str() {
            "@specforge/product" => { runtime = runtime.with_extension(name, Box::new(ProductExtension)); }
            "@specforge/software" => { runtime = runtime.with_extension(name, Box::new(SoftwareExtension)); }
            "@specforge/governance" => { runtime = runtime.with_extension(name, Box::new(GovernanceExtension)); }
            "@specforge/formal" => { runtime = runtime.with_extension(name, Box::new(FormalExtension)); }
            "@specforge/rust" => { runtime = runtime.with_extension(name, Box::new(RustExtension)); }
            "@specforge/typescript" => { runtime = runtime.with_extension(name, Box::new(TypeScriptExtension)); }
            _ => {}
        }
    }
    runtime
}
