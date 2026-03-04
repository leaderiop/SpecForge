use std::path::PathBuf;

use clap::Args;
use specforge_coverage::{
    EntityMapConfig, SpecForgeReport, map_tests_to_entities, parse_junit_xml, parse_libtest_json,
};

use crate::pipeline;

#[derive(Args)]
pub struct CollectRustArgs {
    /// Path to JUnit XML file (e.g., from cargo nextest)
    #[arg(long)]
    pub junit: Option<PathBuf>,

    /// Path to libtest JSON output file (--format json)
    #[arg(long)]
    pub libtest: Option<PathBuf>,

    /// Output path for specforge-report.json
    #[arg(long, default_value = "specforge-report.json")]
    pub output: PathBuf,

    /// Path to spec files. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Strict mode: fail on unknown entity IDs
    #[arg(long)]
    pub strict: bool,
}

pub fn run(args: CollectRustArgs) -> i32 {
    if args.junit.is_none() && args.libtest.is_none() {
        eprintln!("specforge: collect rust requires at least --junit or --libtest");
        return 1;
    }

    // Run pipeline to get spec graph
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    // Parse input files
    let mut all_raw = Vec::new();

    if let Some(junit_path) = &args.junit {
        match std::fs::read_to_string(junit_path) {
            Ok(xml) => match parse_junit_xml(&xml) {
                Ok(results) => {
                    eprintln!(
                        "specforge: parsed {} test(s) from JUnit XML",
                        results.len()
                    );
                    all_raw.extend(results);
                }
                Err(e) => {
                    eprintln!("specforge: error parsing JUnit XML: {e}");
                    return 1;
                }
            },
            Err(e) => {
                eprintln!(
                    "specforge: error reading {}: {e}",
                    junit_path.display()
                );
                return 1;
            }
        }
    }

    if let Some(libtest_path) = &args.libtest {
        match std::fs::read_to_string(libtest_path) {
            Ok(json) => match parse_libtest_json(&json) {
                Ok(results) => {
                    eprintln!(
                        "specforge: parsed {} test(s) from libtest JSON",
                        results.len()
                    );
                    all_raw.extend(results);
                }
                Err(e) => {
                    eprintln!("specforge: error parsing libtest JSON: {e}");
                    return 1;
                }
            },
            Err(e) => {
                eprintln!(
                    "specforge: error reading {}: {e}",
                    libtest_path.display()
                );
                return 1;
            }
        }
    }

    // Map tests to entities
    let config = EntityMapConfig {
        strict: args.strict,
        ..Default::default()
    };
    let map_result = map_tests_to_entities(&all_raw, &result.graph, &config);

    for warning in &map_result.warnings {
        eprintln!("specforge: warning: {warning}");
    }
    for error in &map_result.errors {
        eprintln!("specforge: error: {error}");
    }

    if !map_result.errors.is_empty() {
        return 1;
    }

    // Build report
    let report = SpecForgeReport {
        schema_version: "1.0".to_string(),
        timestamp: None,
        adapter: Some("rust".to_string()),
        entities: map_result.entities,
    };

    // Write report
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&args.output, &json) {
                eprintln!(
                    "specforge: error writing {}: {e}",
                    args.output.display()
                );
                return 1;
            }
            let entity_count = report.entities.len();
            let test_count: usize = report.entities.iter().map(|e| e.tests.len()).sum();
            eprintln!(
                "specforge: wrote {} ({entity_count} entities, {test_count} tests)",
                args.output.display()
            );
        }
        Err(e) => {
            eprintln!("specforge: error serializing report: {e}");
            return 1;
        }
    }

    if map_result.unmapped_count > 0 {
        eprintln!(
            "specforge: {} test(s) could not be mapped to entities",
            map_result.unmapped_count
        );
    }

    0
}
