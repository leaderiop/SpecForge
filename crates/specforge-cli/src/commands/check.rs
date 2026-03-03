use crate::formatter;
use crate::pipeline;
use clap::Args;
use specforge_common::Severity;
use std::path::PathBuf;

#[derive(Args)]
pub struct CheckArgs {
    /// Path to check (directory or file). Defaults to current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Treat warnings as errors
    #[arg(long)]
    pub strict: bool,
}

/// Run the check command. Returns exit code (0 = success, 1 = errors found).
pub fn run(args: CheckArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    formatter::print_diagnostics(&result.diagnostics, &result.sources);

    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity() == Severity::Error);
    let has_warnings = result
        .diagnostics
        .iter()
        .any(|d| d.severity() == Severity::Warning);

    if has_errors || (args.strict && has_warnings) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use crate::pipeline;
    use std::fs;

    #[test]
    fn discover_spec_files_sorted() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.spec"), "").unwrap();
        fs::write(dir.path().join("a.spec"), "").unwrap();
        fs::write(dir.path().join("c.txt"), "").unwrap();

        let files = pipeline::discover_spec_files(dir.path());
        assert_eq!(files.len(), 2);
        assert!(files[0].to_string_lossy().contains("a.spec"));
        assert!(files[1].to_string_lossy().contains("b.spec"));
    }
}
