// CLI pipeline — delegates to specforge_emitter::compile for the shared compilation pipeline.
// This module re-exports the shared types so existing CLI code continues to work.

pub use specforge_emitter::compile::{compile, CompilationContext};
