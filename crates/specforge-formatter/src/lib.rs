mod comments;
pub mod config;
pub mod diff;
pub mod discover;
pub mod engine;
pub mod rules;

pub use config::{load_config, FormatConfig};
pub use diff::{unified_diff, FormatDiff};
pub use discover::discover_targets;
pub use engine::{compute_edits, format_range, format_source, FormatResult, TextEdit};
