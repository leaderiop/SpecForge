mod formal;
mod governance;
mod product;
mod software;

pub use formal::FormalExtension;
pub use governance::GovernanceExtension;
pub use product::ProductExtension;
pub use software::SoftwareExtension;

use specforge_wasm::BuiltinRuntime;

/// Create a `BuiltinRuntime` with all built-in extensions registered.
pub fn default_runtime() -> BuiltinRuntime {
    BuiltinRuntime::new()
        .with_extension("@specforge/product", Box::new(ProductExtension))
        .with_extension("@specforge/software", Box::new(SoftwareExtension))
        .with_extension("@specforge/governance", Box::new(GovernanceExtension))
        .with_extension("@specforge/formal", Box::new(FormalExtension))
}
