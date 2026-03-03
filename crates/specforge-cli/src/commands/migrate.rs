use crate::pipeline;
use clap::Args;
use specforge_common::FormatVersion;
use std::path::PathBuf;

#[derive(Args)]
pub struct MigrateArgs {
    /// Path to the spec project
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Preview changes without writing files
    #[arg(long)]
    pub dry_run: bool,
}

/// A migration transforms spec file content from one version to the next.
struct Migration {
    from: FormatVersion,
    to: FormatVersion,
    transform: fn(&str) -> String,
}

/// Registry of all known migrations.
fn migration_registry() -> Vec<Migration> {
    // Currently empty — only v1.0 exists.
    // Future migrations (e.g., v1.0 → v2.0: remove `verify e2e`) will be added here.
    vec![]
}

/// Build a chain of migrations from `current` to `target`.
fn build_migration_chain(
    registry: &[Migration],
    current: &FormatVersion,
    target: &FormatVersion,
) -> Option<Vec<usize>> {
    if current == target {
        return Some(Vec::new());
    }

    let mut chain = Vec::new();
    let mut version = current.clone();

    while version != *target {
        let next = registry.iter().position(|m| m.from == version);
        match next {
            Some(idx) => {
                version = registry[idx].to.clone();
                chain.push(idx);
            }
            None => return None, // No migration path
        }
    }

    Some(chain)
}

/// Run the migrate command. Returns exit code.
pub fn run(args: MigrateArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let current = &result.config.version;
    let target = FormatVersion::CURRENT;

    match current.cmp(&target) {
        std::cmp::Ordering::Equal => {
            println!(
                "specforge: already at latest version ({})",
                FormatVersion::CURRENT
            );
            0
        }
        std::cmp::Ordering::Greater => {
            eprintln!(
                "specforge: spec format version {} is newer than compiler version {}",
                result.config.version,
                FormatVersion::CURRENT
            );
            1
        }
        std::cmp::Ordering::Less => {
            let registry = migration_registry();
            let chain = build_migration_chain(&registry, current, &target);

            match chain {
                Some(indices) if indices.is_empty() => {
                    println!(
                        "specforge: already at latest version ({})",
                        FormatVersion::CURRENT
                    );
                    0
                }
                Some(indices) => {
                    let spec_files = pipeline::discover_spec_files(&args.path);
                    let mut migrated_count = 0;

                    for file_path in &spec_files {
                        let content = match std::fs::read_to_string(file_path) {
                            Ok(c) => c,
                            Err(e) => {
                                eprintln!(
                                    "specforge: error reading {}: {e}",
                                    file_path.display()
                                );
                                return 1;
                            }
                        };

                        let mut transformed = content.clone();
                        for &idx in &indices {
                            transformed = (registry[idx].transform)(&transformed);
                        }

                        if transformed != content {
                            migrated_count += 1;
                            if args.dry_run {
                                println!(
                                    "  would migrate: {}",
                                    file_path.display()
                                );
                            } else {
                                if let Err(e) = std::fs::write(file_path, &transformed) {
                                    eprintln!(
                                        "specforge: error writing {}: {e}",
                                        file_path.display()
                                    );
                                    return 1;
                                }
                                println!("  migrated: {}", file_path.display());
                            }
                        }
                    }

                    if args.dry_run {
                        println!(
                            "specforge: dry run — {} file(s) would be migrated from {} to {}",
                            migrated_count, current, target
                        );
                    } else {
                        println!(
                            "specforge: migrated {} file(s) from {} to {}",
                            migrated_count, current, target
                        );
                    }
                    0
                }
                None => {
                    eprintln!(
                        "specforge: no migration path from {} to {}",
                        current, target
                    );
                    1
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_no_chain() {
        let registry = migration_registry();
        assert!(registry.is_empty());
    }

    #[test]
    fn chain_same_version() {
        let registry = Vec::new();
        let v1 = FormatVersion::CURRENT;
        let chain = build_migration_chain(&registry, &v1, &v1);
        assert_eq!(chain, Some(Vec::new()));
    }
}
