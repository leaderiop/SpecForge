use crate::pipeline;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct CoverageArgs {
    /// Path to project root (default: current directory)
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Minimum coverage percentage to pass (overrides config threshold)
    #[arg(long)]
    pub min: Option<u32>,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,

    /// Show per-entity details
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Clone, Debug)]
pub enum OutputFormat {
    Text,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err(format!("unknown format: {s} (expected text or json)")),
        }
    }
}

/// Run the coverage command. Returns exit code.
pub fn run(args: CoverageArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Determine project root for report discovery
    let project_root = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());

    // Discover and merge reports
    let merged =
        specforge_coverage::discover_and_merge(&result.config.coverage.test_dirs, &project_root);

    // Compute coverage
    let summary =
        specforge_coverage::compute_coverage(&result.graph, &result.files, &merged);

    // Optionally validate report IDs
    if result.config.coverage.fail_on_unknown_ids {
        let unknown = specforge_coverage::validate_report_ids(&merged, &result.graph);
        if !unknown.is_empty() {
            eprintln!(
                "specforge: unknown entity IDs in report: {}",
                unknown.join(", ")
            );
            return 1;
        }
    }

    // Output
    match args.format {
        OutputFormat::Text => {
            print!("{}", specforge_coverage::format_text(&summary, args.verbose));
        }
        OutputFormat::Json => {
            println!("{}", specforge_coverage::format_json(&summary));
        }
    }

    // Gate on threshold
    let threshold = args.min.or(result.config.coverage.threshold);
    if let Some(min) = threshold {
        if !summary.meets_threshold(min) {
            eprintln!(
                "specforge: coverage {}% is below threshold {}%",
                summary.percentage(),
                min,
            );
            return 1;
        }
    }

    0
}
