mod comments;
pub mod config;
pub mod diff;
pub mod discover;
pub mod engine;
pub mod rules;

pub use config::{FormatConfig, load_config};
pub use diff::{FormatDiff, unified_diff};
pub use discover::discover_targets;
pub use engine::{FormatResult, TextEdit, compute_edits, format_range, format_source};
