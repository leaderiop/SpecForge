//! Signed-publish round trip through the real HTTP boundary (spec #21, T1).
//!
//! Spawns the actual axum router on an ephemeral port, publishes a signed
//! package through `HttpRegistryClient` + `publish_to_registry` (the same
//! orchestration the CLI runs), then asserts the signature survives storage,
//! is served in metadata, and verifies offline — the registry is not needed
//! as a trust anchor.

use specforge_registry::client::registry_config::{AuthMethod, RegistryConfig, RegistryCredential};
use specforge_registry::{
    HttpRegistryClient, ManifestV2, PackageSignature, SigningKey, publish_to_registry,
    verify_signature,
};
use specforge_registry_server::{auth, db::Database, handlers, state::AppState};
use std::sync::Arc;

fn minimal_manifest() -> ManifestV2 {
    serde_json::from_str(
        r#"{
            "name": "@test/signed-ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "ext.wasm"
        }"#,
    )
    .unwrap()
}

fn spawn_server(data_dir: &std::path::Path) -> (String, tokio::task::JoinHandle<()>) {
    let db_path = data_dir.join("registry.db");
    let database = Database::open(&db_path).expect("failed to open database");
    let store = specforge_registry_server::storage::LocalStorage::new(data_dir.join("packages"));
    let state = Arc::new(AppState {
        database,
        storage: store,
        rate_limiter: specforge_registry_server::rate::RateLimiter::new(60),
        publish_limit_per_token: 100,
        publish_limit_per_ip: 100,
    });
    let app = handlers::router(state);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).expect("set nonblocking");
    let listener = tokio::net::TcpListener::from_std(listener).expect("convert to tokio listener");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server failed");
    });
    (format!("http://{}", addr), handle)
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[tokio::test]
async fn signed_publish_round_trips_through_http_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let (base_url, server) = spawn_server(dir.path());

    // A publisher token the upload can authenticate with.
    let raw_token = {
        let db_path = dir.path().join("registry.db");
        let database = Database::open(&db_path).expect("open db");
        auth::create_token(&database, None, "test-publisher", Some(1), false)
    };

    let manifest = minimal_manifest();
    let wasm_bytes: &[u8] = b"\0asm-fake-extension-bytes";

    let key = SigningKey::generate();
    let registry = RegistryConfig {
        alias: "test".to_string(),
        url: format!("{}/v1", base_url),
        scope_filter: None,
        default_registry: true,
    };
    let credential = RegistryCredential {
        alias: "test".to_string(),
        auth_method: AuthMethod::Bearer(raw_token),
    };

    // Publish exactly as the CLI orchestrates it (blocking HTTP client must
    // run off the async reactor: spawn_blocking is the sanctioned pattern —
    let manifest_for_task = manifest.clone();
    let registry_for_task = registry.clone();
    let key_for_task = key.clone();
    let url = tokio::task::spawn_blocking(move || {
        let client = HttpRegistryClient::new();
        publish_to_registry(
            wasm_bytes,
            &manifest_for_task,
            &registry_for_task,
            Some(&credential),
            &client,
            false,
            Some(&key_for_task),
        )
    })
    .await
    .expect("publish task panicked")
    .expect("signed publish should succeed");
    assert!(url.contains("signed-ext"), "{}", url);

    // Metadata serves the signature object, the key id, and the manifest.
    let metadata_url = format!("{}/v1/packages/%40test%2Fsigned-ext/1.0.0", base_url);
    let body: serde_json::Value = reqwest::get(&metadata_url)
        .await
        .expect("metadata request")
        .json()
        .await
        .expect("metadata json");
    assert_eq!(body["key_id"], key.key_id().as_str());
    let signature: PackageSignature = serde_json::from_str(
        body["signature"]
            .as_str()
            .expect("signature served as string"),
    )
    .expect("signature wire object");
    assert_eq!(signature.key_id, key.key_id());
    assert_eq!(signature.public_key, key.public_key_hex());
    assert!(!signature.signed_at.is_empty());
    let served_manifest = body["manifest"].as_str().expect("manifest served");
    assert_eq!(served_manifest, serde_json::to_string(&manifest).unwrap());

    // Offline verification from served data alone: wasm bytes (as downloaded)
    // + served manifest + timestamp inside the signature object. The registry
    // is not the trust anchor.
    verify_signature(
        &manifest.name,
        &manifest.version,
        &sha256_hex(wasm_bytes),
        &sha256_hex(served_manifest.as_bytes()),
        &signature,
    )
    .expect("served signature must verify offline against served metadata");

    server.abort();
}

#[test]
fn tampered_wasm_fails_offline_verification() {
    // A signature over one wasm hash must not verify against different bytes.
    let key = SigningKey::generate();
    let manifest = minimal_manifest();
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    let wasm = b"\0asm-real-bytes";
    let signature = key.sign_package(
        &manifest.name,
        &manifest.version,
        &sha256_hex(wasm),
        &sha256_hex(manifest_json.as_bytes()),
        "2026-09-24T00:00:00+00:00",
    );

    let tampered = b"\0asm-swapped-bytes";
    let err = verify_signature(
        &manifest.name,
        &manifest.version,
        &sha256_hex(tampered),
        &sha256_hex(manifest_json.as_bytes()),
        &signature,
    )
    .unwrap_err();
    assert!(err.contains("verification failed"), "{}", err);
}

#[test]
fn tampered_manifest_fails_offline_verification() {
    // Swapping the manifest (e.g. to relax the sandbox policy) breaks the
    // signature even when the wasm bytes are untouched.
    let key = SigningKey::generate();
    let manifest = minimal_manifest();
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    let wasm = b"\0asm-bytes";
    let signature = key.sign_package(
        &manifest.name,
        &manifest.version,
        &sha256_hex(wasm),
        &sha256_hex(manifest_json.as_bytes()),
        "2026-09-24T00:00:00+00:00",
    );

    let mut swapped: ManifestV2 = minimal_manifest();
    swapped.version = "9.9.9".to_string();
    let swapped_json = serde_json::to_string(&swapped).unwrap();
    let err = verify_signature(
        &manifest.name,
        &manifest.version,
        &sha256_hex(wasm),
        &sha256_hex(swapped_json.as_bytes()),
        &signature,
    )
    .unwrap_err();
    assert!(err.contains("verification failed"), "{}", err);
}
