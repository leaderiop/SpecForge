//! Registry server contracts (spec #21, T3): publish-side rejection rules,
//! rate limiting, and the admin token API.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use specforge_registry_server::{auth, db::Database, handlers, rate::RateLimiter, state::AppState};
use std::sync::Arc;
use tower::ServiceExt as _;

const VALID_MANIFEST: &str =
    r#"{"name":"@test/signed-ext","version":"1.0.0","manifestVersion":2,"wasmPath":"ext.wasm"}"#;
const WASM: &[u8] = b"\0asm-fake-extension-bytes";

fn app_state(dir: &std::path::Path, publish_limit_per_token: u32) -> Arc<AppState> {
    let database = Database::open(&dir.join("registry.db")).expect("open db");
    let store = specforge_registry_server::storage::LocalStorage::new(dir.join("packages"));
    Arc::new(AppState {
        database,
        storage: store,
        rate_limiter: RateLimiter::new(60),
        publish_limit_per_token,
        publish_limit_per_ip: 10_000,
    })
}

fn app(state: Arc<AppState>) -> axum::Router {
    handlers::router(state)
}

fn app_clone(state: &Arc<AppState>) -> axum::Router {
    handlers::router(Arc::clone(state))
}

fn multipart_body(manifest: &str, wasm: &[u8], signature: Option<&str>) -> Body {
    let boundary = "testboundary123";
    let mut body = Vec::new();
    let mut part = |name: &str, content_type: &str, bytes: &[u8]| {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    };
    part("manifest", "application/json", manifest.as_bytes());
    part("wasm", "application/wasm", wasm);
    if let Some(sig) = signature {
        part("signature", "application/json", sig.as_bytes());
    }
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    Body::from(body)
}

fn put_request(token: &str, name: &str, version: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(format!(
            "/v1/packages/{}/{}",
            name.replace('/', "%2F"),
            version
        ))
        .header("authorization", format!("Bearer {}", token))
        .header(
            "content-type",
            "multipart/form-data; boundary=testboundary123".to_string(),
        )
        .body(body)
        .unwrap()
}

#[tokio::test]
async fn unsigned_publish_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), 100);
    let raw = auth::create_token(&state.database, None, "pub", Some(90), false);

    let response = app(state)
        .oneshot(put_request(
            &raw,
            "@test%2Fsigned-ext",
            "1.0.0",
            multipart_body(VALID_MANIFEST, WASM, None),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"]["code"], "UNSIGNED_PACKAGE");
}

#[tokio::test]
async fn name_mismatch_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), 100);
    let raw = auth::create_token(&state.database, None, "pub", Some(90), false);

    // Path says 2.0.0, manifest says 1.0.0.
    let response = app(state)
        .oneshot(put_request(
            &raw,
            "@test%2Fsigned-ext",
            "2.0.0",
            multipart_body(
                VALID_MANIFEST,
                WASM,
                Some(r#"{"sig":"x","keyId":"y","pubkey":"z","signedAt":"now"}"#),
            ),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"]["code"], "NAME_MISMATCH");
}

#[tokio::test]
async fn non_wasm_bytes_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), 100);
    let raw = auth::create_token(&state.database, None, "pub", Some(90), false);

    let response = app(state)
        .oneshot(put_request(
            &raw,
            "@test%2Fsigned-ext",
            "1.0.0",
            multipart_body(
                VALID_MANIFEST,
                b"not a wasm module",
                Some(r#"{"sig":"x","keyId":"y","pubkey":"z","signedAt":"now"}"#),
            ),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"]["code"], "INVALID_WASM");
}

#[tokio::test]
async fn invalid_manifest_schema_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), 100);
    let raw = auth::create_token(&state.database, None, "pub", Some(90), false);

    // Missing required fields entirely.
    let response = app(state)
        .oneshot(put_request(
            &raw,
            "@test%2Fsigned-ext",
            "1.0.0",
            multipart_body(
                r#"{"description":"no name or version"}"#,
                WASM,
                Some(r#"{"sig":"x","keyId":"y","pubkey":"z","signedAt":"now"}"#),
            ),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"]["code"], "INVALID_MANIFEST");
}

#[tokio::test]
async fn network_enabled_sandbox_policy_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), 100);
    let raw = auth::create_token(&state.database, None, "pub", Some(90), false);

    let manifest = r#"{"name":"@test/signed-ext","version":"1.0.0","manifestVersion":2,"wasmPath":"ext.wasm","sandboxPolicy":{"networkAccess":true}}"#;
    let response = app(state)
        .oneshot(put_request(
            &raw,
            "@test%2Fsigned-ext",
            "1.0.0",
            multipart_body(
                manifest,
                WASM,
                Some(r#"{"sig":"x","keyId":"y","pubkey":"z","signedAt":"now"}"#),
            ),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"]["code"], "SANDBOX_POLICY_REJECTED");
}

#[tokio::test]
async fn publish_rate_limit_returns_429() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), 2); // 2 publishes per window per token
    let raw = auth::create_token(&state.database, None, "pub", Some(90), false);
    let router = app(state);

    // The signature object content is irrelevant to the validation contract;
    // dedup prevents version reuse, so vary versions per attempt.
    for version in ["1.0.0", "1.0.1"] {
        let manifest = VALID_MANIFEST.replace("1.0.0", version);
        let response = router
            .clone()
            .oneshot(put_request(
                &raw,
                "@test%2Fsigned-ext",
                version,
                multipart_body(
                    &manifest,
                    WASM,
                    Some(r#"{"sig":"x","keyId":"y","pubkey":"z","signedAt":"now"}"#),
                ),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Third publish inside the window: rate limited.
    let manifest = VALID_MANIFEST.replace("1.0.0", "1.0.2");
    let response = router
        .oneshot(put_request(
            &raw,
            "@test%2Fsigned-ext",
            "1.0.2",
            multipart_body(
                &manifest,
                WASM,
                Some(r#"{"sig":"x","keyId":"y","pubkey":"z","signedAt":"now"}"#),
            ),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"]["code"], "RATE_LIMITED");
}

#[tokio::test]
async fn admin_api_requires_admin_token_and_manages_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), 100);
    let router = app_clone(&state);

    let publisher = auth::create_token(&state.database, None, "publisher", Some(90), false);
    let admin = auth::create_token(&state.database, None, "admin", Some(90), true);

    // Publisher token: forbidden on admin endpoints.
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/admin/tokens")
                .header("authorization", format!("Bearer {}", publisher))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // No auth: unauthorized.
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/admin/tokens")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Admin creates a scoped publish token (expires in 90 days by default).
    let create_body = r#"{"scope":"@acme","label":"ci","expires_in_days":30}"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/admin/tokens")
                .header("authorization", format!("Bearer {}", admin))
                .header("content-type", "application/json")
                .body(Body::from(create_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    let new_token = body["token"].as_str().unwrap().to_string();
    let prefix = body["prefix"].as_str().unwrap().to_string();

    // The created token authenticates and may publish within its scope.
    let verify = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/verify")
                .header("authorization", format!("Bearer {}", new_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify.status(), StatusCode::OK);

    // Admin revokes it by prefix; the token stops working.
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/v1/admin/tokens/{}", prefix))
                .header("authorization", format!("Bearer {}", admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let verify = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/verify")
                .header("authorization", format!("Bearer {}", new_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify.status(), StatusCode::UNAUTHORIZED);
}
