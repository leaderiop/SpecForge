pub mod report;
pub mod merge;
pub mod coverage;
pub mod validate;
pub mod format;
pub mod matrix;

pub use report::{SpecForgeReport, EntityResult, TestResult, TestStatus};
pub use merge::{MergedReport, discover_and_merge};
pub use coverage::{CoverageLevel, EntityCoverage, CoverageSummary, compute_coverage};
pub use validate::validate_report_ids;
pub use format::{format_text, format_json};
pub use matrix::render_traceability_matrix;
