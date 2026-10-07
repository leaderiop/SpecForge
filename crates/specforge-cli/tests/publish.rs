//! `specforge publish` uploads an extension binary with the declaration it
//! reads from it, against the real registry server running in process.

use crate::registry::greet_wasm;
use specforge_registry_client::{HttpRegistryClient, RegistryClient, RegistryConfig};
use specforge_registry_server::{auth, db::Database, handlers, rate::RateLimiter, state::AppState};
use specforge_test::prelude::*;
use std::sync::Arc;
use tempfile::TempDir;

/// A registry server on a local port, with a publisher token, until the
/// runtime is dropped.
struct LocalRegistry {
    url: String,
    token: String,
    _runtime: tokio::runtime::Runtime,
    _data: TempDir,
}

impl LocalRegistry {
    fn start() -> Self {
        let data = TempDir::new().unwrap();
        let database = Database::open(&data.path().join("registry.db")).unwrap();
        let token = auth::create_token(&database, None, "publisher", Some(1), false);
        let state = Arc::new(AppState {
            database,
            storage: specforge_registry_server::storage::LocalStorage::new(
                data.path().join("packages"),
            ),
            rate_limiter: RateLimiter::new(60),
            publish_limit_per_token: 100,
            publish_limit_per_ip: 100,
        });
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.spawn(async move {
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();
            axum::serve(listener, handlers::router(state))
                .await
                .unwrap();
        });
        LocalRegistry {
            url,
            token,
            _runtime: runtime,
            _data: data,
        }
    }

    fn config(&self) -> RegistryConfig {
        RegistryConfig {
            alias: "local".to_string(),
            url: self.url.clone(),
            scope_filter: None,
            default_registry: true,
        }
    }
}

#[specforge_test(
    behavior = "publish_to_registry",
    verify = "publish derives the stored declaration from the binary"
)]
fn publish_stores_the_declaration_the_binary_declares() {
    let registry = LocalRegistry::start();
    let project = TempDir::new().unwrap();
    std::fs::write(
        project.path().join("specforge.json"),
        serde_json::json!({
            "name": "p", "version": "0.1.0",
            "registries": [{ "alias": "local", "url": registry.url, "default_registry": true }]
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(project.path().join("greet.wasm"), greet_wasm()).unwrap();
    let home = TempDir::new().unwrap();

    // Only the binary: it is all publish reads.
    let output = std::process::Command::new(assert_cmd::cargo_bin!("specforge"))
        .arg("publish")
        .arg(project.path().join("greet.wasm"))
        .arg("--path")
        .arg(project.path())
        .args(["--format", "json"])
        .env("HOME", home.path())
        .env("SPECFORGE_REGISTRY_TOKEN", &registry.token)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let published: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(published["name"], "@sdk/greet");
    assert_eq!(published["version"], "0.1.0");

    // The stored manifest is exactly greet's declaration.
    let served = HttpRegistryClient::new()
        .fetch(
            &specforge_protocol_types::PackageName::parse("@sdk/greet").unwrap(),
            &specforge_protocol_types::package::Version::new(0, 1, 0),
            &registry.config(),
        )
        .unwrap();
    let stored: specforge_protocol_types::ExtensionDeclaration =
        serde_json::from_str(&served.manifest).unwrap();
    let runtime = specforge_component::ComponentRuntime::new();
    runtime
        .load_module_bytes("@sdk/greet", &greet_wasm())
        .unwrap();
    let declared = specforge_wasm::protocol::load_declaration(&runtime, "@sdk/greet")
        .unwrap()
        .declaration;
    assert_eq!(stored, declared);
    assert_eq!(
        stored.handshake.description.as_deref(),
        Some("Friendly greetings")
    );
}
