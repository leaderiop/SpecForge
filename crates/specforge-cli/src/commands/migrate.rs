use crate::pipeline;
use clap::Args;
use specforge_common::FormatVersion;
use std::path::PathBuf;

#[derive(Args)]
pub struct MigrateArgs {
    /// Path to the spec project
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Run the migrate command. Returns exit code.
pub fn run(args: MigrateArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    match result.config.version.cmp(&FormatVersion::CURRENT) {
        std::cmp::Ordering::Equal => {
            println!(
                "specforge: already at latest version ({})",
                FormatVersion::CURRENT
            );
            0
        }
        std::cmp::Ordering::Less => {
            eprintln!(
                "specforge: migration from {} to {} is not yet supported",
                result.config.version,
                FormatVersion::CURRENT
            );
            1
        }
        std::cmp::Ordering::Greater => {
            eprintln!(
                "specforge: spec format version {} is newer than compiler version {}",
                result.config.version,
                FormatVersion::CURRENT
            );
            1
        }
    }
}
