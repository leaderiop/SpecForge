mod add;
mod check;
mod collect;
mod doctor;
mod explain;
mod export;
mod extension_authoring;
mod extensions;
mod format;
mod init;
mod mcp;
mod migrate;
mod model;
mod pipeline;
mod product;
mod providers;
mod query;
mod remove;
mod stats;
mod trace;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "specforge", version, about = "SpecForge compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new SpecForge project
    Init {
        /// Project name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,

        /// Project version (defaults to 0.1.0)
        #[arg(long)]
        version: Option<String>,

        /// Extensions to install
        #[arg(long)]
        extensions: Vec<String>,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Validate .spec files and report diagnostics
    Check {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Promote warnings to errors
        #[arg(long)]
        strict: bool,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Export spec graph to stdout in various formats
    Export {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: graph, brief, context, or dot
        #[arg(long, default_value = "graph")]
        format: String,

        /// Scope export to subgraph reachable from this entity ID
        #[arg(long)]
        scope: Option<String>,

        /// Suppress schema embedding (keeps format_version 1.0)
        #[arg(long)]
        no_schema: bool,

        /// Request a specific schema version for the export
        #[arg(long)]
        schema_version: Option<String>,
    },
    /// Output the Graph Protocol schema
    Schema {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Filter output to a single entity kind
        #[arg(long)]
        kind: Option<String>,

        /// Publish as standalone JSON Schema (draft 2020-12)
        #[arg(long)]
        publish: bool,
    },
    /// Render the logical data model (entity kinds, fields, relationships)
    Model {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: markdown (default), mermaid, dot, json, dbml
        #[arg(long, default_value = "markdown")]
        format: String,

        /// Group entities by: extension (default), none
        #[arg(long, default_value = "extension")]
        group_by: String,

        /// Field detail level: none, keys (default), all
        #[arg(long, default_value = "keys")]
        fields: String,

        /// Filter to a single extension
        #[arg(long)]
        extension: Option<String>,

        /// Filter to specific entity kinds (comma-separated)
        #[arg(long, value_delimiter = ',')]
        kinds: Vec<String>,

        /// Root entity kind for depth-scoped output
        #[arg(long)]
        root: Option<String>,

        /// Maximum depth from root kind (requires --root)
        #[arg(long, requires = "root")]
        depth: Option<usize>,
    },
    /// Query the graph at multiple resolutions
    Query {
        /// Entity ID to query
        entity: String,

        /// Path to the spec root directory
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Number of hops from the entity (0 = entity only)
        #[arg(long, default_value = "1")]
        depth: usize,

        /// Filter results to specific entity kinds (can be repeated)
        #[arg(long)]
        kind: Vec<String>,
    },
    /// Show traceability chain for an entity
    Trace {
        /// Entity ID to trace
        entity: String,

        /// Path to the spec root directory
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Format .spec files
    Format {
        /// Explicit file or directory paths to format
        #[arg()]
        paths: Vec<String>,

        /// Path to the spec root directory
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Check formatting without modifying files (exit 1 if unformatted)
        #[arg(long)]
        check: bool,

        /// Show unified diff of formatting changes
        #[arg(long)]
        diff: bool,

        /// Read from stdin, write to stdout
        #[arg(long)]
        stdin: bool,
    },
    /// Show project statistics
    Stats {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Install an extension
    Add {
        /// Extension specifier (e.g., @scope/name@1.0.0 or ./path)
        specifier: String,

        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Remove an installed extension
    Remove {
        /// Extension name
        name: String,

        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Force removal even if other extensions depend on it
        #[arg(long)]
        force: bool,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List installed extensions
    Extensions {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List configured providers
    Providers {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Collect test results from a test runner
    Collect {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Collector name (e.g., rust, javascript). Auto-detected if omitted.
        #[arg(long)]
        collector: Option<String>,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Run health checks on installed extensions
    Doctor {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Start MCP server (JSON-RPC over stdio)
    Mcp {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Migrate .spec files between format versions
    Migrate {
        /// Path to the spec root directory
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Show unified diff without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Skip creating .spec.bak backup files
        #[arg(long)]
        no_backup: bool,

        /// Restore files from .spec.bak backups
        #[arg(long)]
        rollback: bool,

        /// Target format version (defaults to current)
        #[arg(long)]
        target_version: Option<String>,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Generate shell completions
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Explain a diagnostic code
    Explain {
        /// Diagnostic code (e.g., E001, W010)
        code: String,
    },
    /// Extension authoring commands
    Extension {
        #[command(subcommand)]
        action: ExtensionAction,
    },
    /// Product entity queries and analytics
    Product {
        #[command(subcommand)]
        action: ProductAction,
    },
}

#[derive(Subcommand)]
enum ProductAction {
    /// List features
    Features {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        offset: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List journeys
    Journeys {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List deliverables
    Deliverables {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List milestones
    Milestones {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List modules
    Modules {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List terms
    Terms {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List personas
    Personas {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List channels
    Channels {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// List releases
    Releases {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show milestone completion progress
    MilestoneCompletion {
        milestone: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show journey feature-module coverage
    JourneyCoverage {
        journey: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show feature impact analysis
    FeatureImpact {
        feature: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show features that depend on a given feature
    FeatureDependents {
        feature: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show features reachable from a persona (via journeys)
    PersonaFeatures {
        persona: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show features reachable from a channel (via journeys)
    ChannelFeatures {
        channel: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show status breakdown across all entity kinds
    BulkStatus {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Show project health score
    Health {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "human")]
        format: String,
    },
}

#[derive(Subcommand)]
enum ExtensionAction {
    /// Scaffold a new extension project
    Init {
        /// Extension name
        #[arg(long)]
        name: Option<String>,

        /// Path for the new extension
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Validate extension project structure
    Build {
        /// Path to the extension project
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Validate extension manifest against schema
    Validate {
        /// Path to the extension project
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, version, extensions, format } => {
            let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let exit_code = init::run(&path, name.as_deref(), version.as_deref(), &extensions, &format);
            std::process::exit(exit_code);
        }
        Commands::Check { path, strict, format } => {
            let exit_code = check::run(&path, strict, &format);
            std::process::exit(exit_code);
        }
        Commands::Export { path, format, scope, no_schema, schema_version } => {
            let exit_code = export::run(&path, &format, scope.as_deref(), no_schema, schema_version.as_deref());
            std::process::exit(exit_code);
        }
        Commands::Schema { path, kind, publish } => {
            let exit_code = export::run_schema(&path, kind.as_deref(), publish);
            std::process::exit(exit_code);
        }
        Commands::Model { path, format, group_by, fields, extension, kinds, root, depth } => {
            let exit_code = model::run(
                &path, &format, &group_by, &fields,
                extension.as_deref(), &kinds, root.as_deref(), depth,
            );
            std::process::exit(exit_code);
        }
        Commands::Query { entity, path, depth, kind } => {
            let exit_code = query::run(&path, &entity, depth, &kind);
            std::process::exit(exit_code);
        }
        Commands::Trace { entity, path } => {
            let exit_code = trace::run(&path, &entity);
            std::process::exit(exit_code);
        }
        Commands::Format { paths, path, check, diff, stdin } => {
            let exit_code = format::run(&path, check, diff, stdin, &paths);
            std::process::exit(exit_code);
        }
        Commands::Stats { path, format } => {
            let exit_code = stats::run(&path, &format);
            std::process::exit(exit_code);
        }
        Commands::Add { specifier, path, format } => {
            let exit_code = add::run(&specifier, &path, &format);
            std::process::exit(exit_code);
        }
        Commands::Remove { name, path, force, format } => {
            let exit_code = remove::run(&name, &path, force, &format);
            std::process::exit(exit_code);
        }
        Commands::Extensions { path, format } => {
            let exit_code = extensions::run(&path, &format);
            std::process::exit(exit_code);
        }
        Commands::Providers { path, format } => {
            let exit_code = providers::run(&path, &format);
            std::process::exit(exit_code);
        }
        Commands::Collect { path, collector, format } => {
            let exit_code = collect::run(&path, collector.as_deref(), &format);
            std::process::exit(exit_code);
        }
        Commands::Doctor { path, format } => {
            let exit_code = doctor::run(&path, &format);
            std::process::exit(exit_code);
        }
        Commands::Mcp { path } => {
            let exit_code = mcp::run(&path);
            std::process::exit(exit_code);
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "specforge", &mut std::io::stdout());
            std::process::exit(0);
        }
        Commands::Explain { code } => {
            let exit_code = explain::run(&code);
            std::process::exit(exit_code);
        }
        Commands::Migrate { path, dry_run, no_backup, rollback, target_version, format } => {
            let exit_code = migrate::run(&path, dry_run, no_backup, rollback, target_version.as_deref(), &format);
            std::process::exit(exit_code);
        }
        Commands::Product { action } => {
            let exit_code = match action {
                ProductAction::Features { path, status, priority, limit, offset, format } => {
                    product::run_list(&path, "feature", status.as_deref(), priority.as_deref(), limit, offset, &format)
                }
                ProductAction::Journeys { path, limit, format } => {
                    product::run_list(&path, "journey", None, None, limit, None, &format)
                }
                ProductAction::Deliverables { path, status, limit, format } => {
                    product::run_list(&path, "deliverable", status.as_deref(), None, limit, None, &format)
                }
                ProductAction::Milestones { path, status, limit, format } => {
                    product::run_list(&path, "milestone", status.as_deref(), None, limit, None, &format)
                }
                ProductAction::Modules { path, limit, format } => {
                    product::run_list(&path, "module", None, None, limit, None, &format)
                }
                ProductAction::Terms { path, limit, format } => {
                    product::run_list(&path, "term", None, None, limit, None, &format)
                }
                ProductAction::Personas { path, limit, format } => {
                    product::run_list(&path, "persona", None, None, limit, None, &format)
                }
                ProductAction::Channels { path, limit, format } => {
                    product::run_list(&path, "channel", None, None, limit, None, &format)
                }
                ProductAction::Releases { path, status, limit, format } => {
                    product::run_list(&path, "release", status.as_deref(), None, limit, None, &format)
                }
                ProductAction::MilestoneCompletion { milestone, path, format } => {
                    product::run_milestone_completion(&path, &milestone, &format)
                }
                ProductAction::JourneyCoverage { journey, path, format } => {
                    product::run_journey_coverage(&path, &journey, &format)
                }
                ProductAction::FeatureImpact { feature, path, format } => {
                    product::run_feature_impact(&path, &feature, &format)
                }
                ProductAction::FeatureDependents { feature, path, format } => {
                    product::run_feature_dependents(&path, &feature, &format)
                }
                ProductAction::PersonaFeatures { persona, path, format } => {
                    product::run_persona_features(&path, &persona, &format)
                }
                ProductAction::ChannelFeatures { channel, path, format } => {
                    product::run_channel_features(&path, &channel, &format)
                }
                ProductAction::BulkStatus { path, format } => {
                    product::run_bulk_status(&path, &format)
                }
                ProductAction::Health { path, format } => {
                    product::run_health(&path, &format)
                }
            };
            std::process::exit(exit_code);
        }
        Commands::Extension { action } => {
            let exit_code = match action {
                ExtensionAction::Init { name, path, format } => {
                    extension_authoring::run_init(&path, name.as_deref(), &format)
                }
                ExtensionAction::Build { path, format } => {
                    extension_authoring::run_build(&path, &format)
                }
                ExtensionAction::Validate { path, format } => {
                    extension_authoring::run_validate(&path, &format)
                }
            };
            std::process::exit(exit_code);
        }
    }
}
