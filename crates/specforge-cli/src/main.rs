mod add;
mod analyze;
mod check;
mod collect;
mod color;
mod doctor;
mod explain;
mod export;
mod extension_authoring;
mod extension_command;
mod extensions;
mod format;
mod infer_status;
mod init;
mod login;
mod mcp;
mod migrate;
mod model;
mod new;
mod outline;
mod pipeline;
mod providers;
mod publish;
mod query;
mod remove;
mod search;
mod stats;
mod trace;
mod update;
mod watch;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "specforge",
    version,
    about = "SpecForge compiler",
    after_help = "Extensions add commands: specforge <extension> <command>, e.g. `specforge product features`.\n`specforge <extension> --help` lists an extension's commands."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Human-readable vs machine-readable output for registry-style commands.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Human,
    Json,
}

/// Export graph resolutions (`specforge export --format`).
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ExportFormat {
    Graph,
    Brief,
    Context,
    Dot,
}

/// Export formats a published JSON Schema may describe (`specforge schema --format`).
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum SchemaFormat {
    Graph,
    Context,
    Brief,
}

/// Renderers for the logical data model (`specforge model --format`).
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ModelFormat {
    Markdown,
    Mermaid,
    Dot,
    Json,
    Dbml,
}

/// Renderers for the extension architecture (`specforge outline --format`).
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum OutlineFormat {
    Markdown,
    Mermaid,
    Dot,
    Json,
}

/// Entity grouping for the data model (`specforge model --group-by`).
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum GroupBy {
    Extension,
    None,
}

/// Field detail level shared by `model --fields` and `outline --fields`.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum FieldLevel {
    None,
    Keys,
    All,
}

/// Dependency visibility for `specforge outline --deps`.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum DepsLevel {
    Direct,
    Effective,
    Full,
}

/// Static analysis passes for `specforge analyze --pass`.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum AnalysisPass {
    All,
    Coverage,
    Contracts,
}

impl AnalysisPass {
    /// The pass name used to select and order built-in/extension passes.
    fn name(self) -> &'static str {
        match self {
            AnalysisPass::All => "all",
            AnalysisPass::Coverage => "coverage",
            AnalysisPass::Contracts => "contracts",
        }
    }
}
impl OutputFormat {
    /// Report diagnostics that don't stop the command (the registry
    /// configuration's E067/W140/I003): `severity[CODE]: message` on
    /// stderr in either format, so JSON stdout stays one document.
    fn eprint_diagnostics(self, diagnostics: &[specforge_common::Diagnostic]) {
        for diagnostic in diagnostics {
            eprintln!("{}", export::render_plain(diagnostic));
        }
    }

    /// An operation's failure as the JSON document every command prints:
    /// `{"error", "code", "suggestion"}`.
    fn op_error_json(error: &specforge_ops::OpError) -> serde_json::Value {
        serde_json::json!({
            "error": error.message,
            "code": error.code,
            "suggestion": error.suggestion,
        })
    }

    /// Report a failure with diagnostic `code`, as [`Self::print_op_error`].
    fn print_error(self, message: &str, code: &str) {
        self.print_op_error(&specforge_ops::OpError::new(code.to_string(), message));
    }

    /// Report an operation's failure: `{"error", "code", "suggestion"}` on
    /// stdout as JSON, or `error[CODE]: …` and a hint on stderr.
    fn print_op_error(self, error: &specforge_ops::OpError) {
        match self {
            OutputFormat::Json => {
                let output = Self::op_error_json(error);
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Human => {
                eprintln!("error[{}]: {}", error.code, error.message);
                if let Some(suggestion) = &error.suggestion {
                    eprintln!("  hint: {suggestion}");
                }
            }
        }
    }
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
        format: OutputFormat,
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
        format: OutputFormat,

        /// Enable additional lint profiles (e.g., pedantic, inferred)
        #[arg(long, value_delimiter = ',')]
        lint: Vec<String>,

        /// Record each entity's status in specforge-cache.json when the
        /// check passes (the build cache history rules compare against)
        #[arg(long)]
        cache: bool,
    },
    /// Export spec graph to stdout in various formats
    Export {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: graph, brief, context, or dot
        #[arg(long, default_value = "graph")]
        format: ExportFormat,

        /// Scope export to subgraph reachable from this entity ID
        #[arg(long)]
        scope: Option<String>,

        /// Suppress schema embedding in `graph` exports (keeps format_version 1.0)
        #[arg(long)]
        no_schema: bool,

        /// Embed the schema in `context` and `brief` exports, and in a
        /// `graph` export under --max-tokens. They leave it out by default:
        /// an agent reading the export needs the entities, and the schema is
        /// most of the bytes. Under --max-tokens an embedded schema counts
        /// toward the budget, and a budget it doesn't fit in fails (E062).
        #[arg(long, conflicts_with = "no_schema")]
        with_schema: bool,

        /// Request a specific schema version for the export
        #[arg(long)]
        schema_version: Option<String>,

        /// Token budget for the export: keeps the most central entities that
        /// fit. The schema is left out unless --with-schema is given, and
        /// then counts toward the budget. A `graph` export lists the dropped
        /// entities under `token_budget`; below one entity it is the envelope
        /// with no entities, and below even that it fails (E062). Ignored by
        /// `dot`.
        #[arg(long)]
        max_tokens: Option<usize>,
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

        /// Export format the published schema should describe
        #[arg(long, default_value = "graph")]
        format: SchemaFormat,
    },
    /// Render the logical data model (entity kinds, fields, relationships)
    Model {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: markdown (default), mermaid, dot, json, dbml
        #[arg(long, default_value = "markdown")]
        format: ModelFormat,

        /// Group entities by: extension (default), none
        #[arg(long, default_value = "extension")]
        group_by: GroupBy,

        /// Field detail level: none, keys (default), all
        #[arg(long, default_value = "keys")]
        fields: FieldLevel,

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
    /// Render the extension architecture (dependencies, enhancements, contributions)
    Outline {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: markdown (default), mermaid, dot, json
        #[arg(long, default_value = "markdown")]
        format: OutlineFormat,

        /// Detail level: none (overview only), keys (default), all (full field attribution)
        #[arg(long, default_value = "keys")]
        fields: FieldLevel,

        /// Dependency visibility: direct (declared only), effective (direct + used transitive), full (all transitive)
        #[arg(long, default_value = "direct")]
        deps: DepsLevel,
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
    /// Show traceability chain for an entity, or for every entity
    Trace {
        /// Entity ID to trace (omit to trace every entity)
        entity: Option<String>,

        /// Path to the spec root directory
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: json or human
        #[arg(long, default_value = "json")]
        format: OutputFormat,
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
        format: OutputFormat,
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
        format: OutputFormat,

        /// Accept unsigned packages (publisher verification skipped)
        #[arg(long, default_value_t = false)]
        allow_unsigned: bool,

        /// Accept key changes non-interactively (for CI)
        #[arg(long, default_value_t = false)]
        yes: bool,
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
        format: OutputFormat,
    },
    /// List installed extensions
    Extensions {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Publish an extension to the registry
    Publish {
        /// Path to the extension project
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Search for extensions in registries
    Search {
        /// Search query
        query: String,

        /// Path to the project root (for registry config)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Run static analysis passes over the compiled project
    Analyze {
        /// Analysis pass to run (all, coverage, contracts)
        pass: Option<AnalysisPass>,

        /// Project directory (defaults to current directory)
        #[arg(long)]
        path: Option<String>,

        /// Machine-readable output
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Fail (exit 1) on warnings as well as errors
        #[arg(long, default_value_t = false)]
        strict: bool,

        /// Test-results report for proof-level verdicts (default: the project's
        /// specforge-report.json, written by `specforge collect`)
        #[arg(long)]
        test_results: Option<String>,

        /// Verify declared bounds and claims with an SMT solver (z3)
        #[arg(long, default_value_t = false)]
        prove: bool,

        /// Fail with a non-zero exit when proof coverage falls below this
        /// percentage (needs test results)
        #[arg(long)]
        min: Option<f64>,
    },
    /// Watch a project and rebuild incrementally on changes
    Watch {
        /// Project directory (defaults to current directory)
        #[arg(long)]
        path: Option<String>,

        /// Emit one JSON object per rebuild cycle
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Check every incremental rebuild against a cold rebuild (slower)
        #[arg(long, default_value_t = false)]
        verify_incremental: bool,
    },
    /// Scaffold a new extension project
    New {
        /// Extension name (e.g. @you/my-ext)
        name: String,

        /// Scaffold an extension project (SDK-authored wasm)
        #[arg(long, default_value_t = false)]
        extension: bool,

        /// Directory to scaffold into
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Update installed extensions to latest compatible versions
    Update {
        /// Extension name (updates all if omitted)
        name: Option<String>,

        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,

        /// Allow new major versions (otherwise each extension stays within
        /// its locked version's caret range)
        #[arg(long, default_value_t = false)]
        major: bool,

        /// Accept unsigned packages (publisher verification skipped)
        #[arg(long, default_value_t = false)]
        allow_unsigned: bool,

        /// Accept key changes non-interactively (for CI)
        #[arg(long, default_value_t = false)]
        yes: bool,
    },
    /// Authenticate with a registry
    Login {
        /// Registry alias (defaults to "default")
        #[arg(long)]
        registry: Option<String>,

        /// Authentication token
        #[arg(long)]
        token: Option<String>,

        /// Path to the project root (for registry config)
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Remove registry credentials
    Logout {
        /// Registry alias (defaults to "default")
        #[arg(long)]
        registry: Option<String>,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// List configured providers
    Providers {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Run the project's test runners and record which entities their tests prove
    Collect {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Collector to use (e.g. cargo-test). Detected from project files if omitted.
        #[arg(long, alias = "collector", value_name = "NAME")]
        runner: Option<String>,

        /// Parse the runner's existing report instead of running it
        #[arg(long)]
        no_run: bool,

        /// Report files to parse instead of running the runner (implies --no-run)
        #[arg(long = "report", value_name = "FILE")]
        reports: Vec<PathBuf>,

        /// Run the declared test command without asking (for CI)
        #[arg(long)]
        yes: bool,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Run health checks on installed extensions
    Doctor {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
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
        format: OutputFormat,
    },
    /// Generate shell completions: the built-in commands, and the commands
    /// of the extensions the project in the current directory enables
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
    /// Show inference progress (analyzed files, gaps, stale entries)
    InferStatus {
        /// Path to the project root
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,

        /// Show unanalyzed files grouped by directory
        #[arg(long)]
        gaps: bool,

        /// Show stale and deleted files
        #[arg(long)]
        stale: bool,

        /// Show detailed gap analysis (pub Rust items without spec entities)
        #[arg(long)]
        gaps_detail: bool,
    },
    /// Extension authoring commands
    Extension {
        #[command(subcommand)]
        action: ExtensionAction,
    },
    /// An extension's command: `specforge <extension> <command>`, e.g.
    /// `specforge product features` (the project's extensions declare them)
    #[command(external_subcommand)]
    External(Vec<String>),
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
        format: OutputFormat,
    },
    /// Validate extension project structure
    Build {
        /// Path to the extension project
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
    /// Validate extension manifest against schema
    Validate {
        /// Path to the extension project
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: OutputFormat,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Init {
            name,
            version,
            extensions,
            format,
        } => {
            let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            init::run(
                &path,
                name.as_deref(),
                version.as_deref(),
                &extensions,
                format,
            )
        }
        Commands::Check {
            path,
            strict,
            format,
            lint,
            cache,
        } => check::run(&path, strict, format, &lint, cache),
        Commands::Export {
            path,
            format,
            scope,
            no_schema,
            with_schema,
            schema_version,
            max_tokens,
        } => export::run(
            &path,
            format,
            scope.as_deref(),
            // context, brief and a budgeted graph export leave the schema
            // out unless asked for it (the policy lives in specforge-ops).
            match (no_schema, with_schema) {
                (true, _) => specforge_ops::export::Schema::Without,
                (_, true) => specforge_ops::export::Schema::With,
                _ => specforge_ops::export::Schema::Default,
            },
            schema_version.as_deref(),
            max_tokens,
        ),
        Commands::Schema {
            path,
            kind,
            publish,
            format,
        } => export::run_schema(&path, kind.as_deref(), publish, format),
        Commands::Model {
            path,
            format,
            group_by,
            fields,
            extension,
            kinds,
            root,
            depth,
        } => model::run(
            &path,
            format,
            group_by,
            fields,
            extension.as_deref(),
            &kinds,
            root.as_deref(),
            depth,
        ),
        Commands::Outline {
            path,
            format,
            fields,
            deps,
        } => outline::run(&path, format, fields, deps),
        Commands::Query {
            entity,
            path,
            depth,
            kind,
        } => query::run(&path, &entity, depth, &kind),
        Commands::Trace {
            entity,
            path,
            format,
        } => trace::run(&path, entity.as_deref(), format),
        Commands::Format {
            paths,
            path,
            check,
            diff,
            stdin,
        } => format::run(&path, check, diff, stdin, &paths),
        Commands::Stats { path, format } => stats::run(&path, format),
        Commands::Add {
            specifier,
            path,
            format,
            allow_unsigned,
            yes,
        } => add::run(&specifier, &path, format, allow_unsigned, yes),
        Commands::Analyze {
            pass,
            path,
            json,
            strict,
            test_results,
            min,
            prove,
        } => {
            let project = path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            analyze::run(
                &project,
                pass,
                json,
                strict,
                test_results.as_deref().map(Path::new),
                min,
                prove,
            )
        }
        Commands::Watch {
            path,
            json,
            verify_incremental,
        } => {
            let project = path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            watch::run(&project, json, verify_incremental)
        }
        Commands::New {
            name,
            extension,
            path,
            format,
        } => new::run(&name, extension, &path, format),
        Commands::Remove {
            name,
            path,
            force,
            format,
        } => remove::run(&name, &path, force, format),
        Commands::Extensions { path, format } => extensions::run(&path, format),
        Commands::Publish { path, format } => publish::run(&path, format),
        Commands::Search {
            query,
            path,
            format,
        } => search::run(&query, &path, format),
        Commands::Update {
            name,
            path,
            format,
            major,
            allow_unsigned,
            yes,
        } => update::run(name.as_deref(), &path, format, major, allow_unsigned, yes),
        Commands::Login {
            registry,
            token,
            path,
            format,
        } => login::run(registry.as_deref(), token.as_deref(), &path, format),
        Commands::Logout { registry, format } => login::run_logout(registry.as_deref(), format),
        Commands::Providers { path, format } => providers::run(&path, format),
        Commands::Collect {
            path,
            runner,
            no_run,
            reports,
            yes,
            format,
        } => collect::run(
            &path,
            &collect::Options {
                runner: runner.as_deref(),
                no_run,
                reports: &reports,
                yes,
            },
            format,
        ),
        Commands::Doctor { path, format } => doctor::run(&path, format),
        Commands::Mcp { path } => mcp::run(&path),
        Commands::Completions { shell } => {
            // The built-ins, and the commands the extensions of the project
            // in the current directory contribute (`specforge product ...`).
            let mut cmd =
                extension_command::with_extension_commands(Cli::command(), Path::new("."));
            clap_complete::generate(shell, &mut cmd, "specforge", &mut std::io::stdout());
            0
        }
        Commands::Explain { code } => explain::run(&code),
        Commands::Migrate {
            path,
            dry_run,
            no_backup,
            rollback,
            target_version,
            format,
        } => migrate::run(
            &path,
            dry_run,
            no_backup,
            rollback,
            target_version.as_deref(),
            format,
        ),
        Commands::InferStatus {
            path,
            format,
            gaps,
            stale,
            gaps_detail,
        } => infer_status::run(&path, format, gaps, stale, gaps_detail),
        Commands::External(argv) => {
            let builtins: Vec<String> = Cli::command()
                .get_subcommands()
                .map(|c| c.get_name().to_string())
                .collect();
            extension_command::run(&argv, &builtins)
        }
        Commands::Extension { action } => match action {
            ExtensionAction::Init { name, path, format } => {
                extension_authoring::run_init(&path, name.as_deref(), format)
            }
            ExtensionAction::Build { path, format } => {
                extension_authoring::run_build(&path, format)
            }
            ExtensionAction::Validate { path, format } => {
                extension_authoring::run_validate(&path, format)
            }
        },
    };
    std::process::exit(exit_code);
}
