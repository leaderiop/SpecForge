pub mod atexit;
pub mod guard;
pub mod registry;
pub mod report;
pub mod slugify;

/// Re-export the proc macro so users can write `#[specforge::test(...)]`
/// when they `use specforge_test::prelude::*`.
pub mod prelude {
    pub use specforge_test_macros::test as specforge_test;
}

/// Private API consumed by proc macro expansion. Not for direct use.
#[doc(hidden)]
pub mod __private {
    pub use crate::guard::TestGuard;
    pub use crate::slugify::slugify_verify_description;
}
