mod db;
mod handlers;
mod storage;
mod auth;
mod state;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "specforge-registry", version, about = "SpecForge extension registry server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the registry server
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "4873")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Data directory for storage and database
        #[arg(long, default_value = "./registry-data")]
        data_dir: PathBuf,
    },
    /// Create an authentication token
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
}

#[derive(Subcommand)]
enum TokenAction {
    /// Create a new token
    Create {
        /// Scope filter (e.g., "@myorg") — empty means full access
        #[arg(long)]
        scope: Option<String>,

        /// Label for the token
        #[arg(long, default_value = "default")]
        label: String,

        /// Data directory (must match the server's)
        #[arg(long, default_value = "./registry-data")]
        data_dir: PathBuf,
    },
    /// List all tokens
    List {
        /// Data directory
        #[arg(long, default_value = "./registry-data")]
        data_dir: PathBuf,
    },
    /// Revoke a token
    Revoke {
        /// Token prefix (first 8 chars)
        prefix: String,

        /// Data directory
        #[arg(long, default_value = "./registry-data")]
        data_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "specforge_registry_server=info,tower_http=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, host, data_dir } => {
            std::fs::create_dir_all(&data_dir).expect("failed to create data directory");

            let database = db::Database::open(&data_dir.join("registry.db"))
                .expect("failed to open database");
            let store = storage::LocalStorage::new(data_dir.join("packages"));
            let app_state = Arc::new(state::AppState { database, storage: store });

            let app = handlers::router(app_state);

            let addr = format!("{}:{}", host, port);
            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .unwrap_or_else(|e| panic!("failed to bind to {}: {}", addr, e));

            tracing::info!("registry server listening on http://{}", addr);

            axum::serve(listener, app)
                .await
                .expect("server error");
        }
        Commands::Token { action } => match action {
            TokenAction::Create { scope, label, data_dir } => {
                std::fs::create_dir_all(&data_dir).expect("failed to create data directory");
                let database = db::Database::open(&data_dir.join("registry.db"))
                    .expect("failed to open database");

                let token = auth::create_token(&database, scope.as_deref(), &label);
                println!("Token created successfully.\n");
                println!("  token: {}", token);
                println!("  scope: {}", scope.as_deref().unwrap_or("(full access)"));
                println!("  label: {}", label);
                println!("\nStore this token securely — it cannot be retrieved later.");
            }
            TokenAction::List { data_dir } => {
                let database = db::Database::open(&data_dir.join("registry.db"))
                    .expect("failed to open database");
                let tokens = auth::list_tokens(&database);
                if tokens.is_empty() {
                    println!("No tokens found.");
                } else {
                    println!("{:<12} {:<20} {:<16} {}", "PREFIX", "LABEL", "SCOPE", "CREATED");
                    for t in &tokens {
                        println!("{:<12} {:<20} {:<16} {}",
                            &t.token_hash[..8],
                            t.label,
                            t.scope.as_deref().unwrap_or("(all)"),
                            t.created_at,
                        );
                    }
                }
            }
            TokenAction::Revoke { prefix, data_dir } => {
                let database = db::Database::open(&data_dir.join("registry.db"))
                    .expect("failed to open database");
                if auth::revoke_token(&database, &prefix) {
                    println!("Token revoked.");
                } else {
                    println!("No token found with prefix '{}'.", prefix);
                }
            }
        },
    }
}
