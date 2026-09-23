use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::auth;
use crate::db::PackageVersion;
use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/packages/{name}", get(get_package_versions))
        .route("/v1/packages/{name}/{version}", get(get_package_version))
        .route(
            "/v1/packages/{name}/{version}",
            put(publish_package).layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route("/v1/packages/{name}/{version}", delete(yank_package))
        .route(
            "/v1/packages/{name}/{version}/download",
            get(download_package),
        )
        .route("/v1/search", get(search_packages))
        .route("/v1/auth/verify", post(verify_auth))
        .route("/health", get(health_check))
        .with_state(state)
}

// --- Response types ---

#[derive(Serialize)]
struct PackageVersionsResponse {
    name: String,
    versions: Vec<String>,
}

#[derive(Serialize)]
struct PackageMetadataResponse {
    name: String,
    version: String,
    sha256: String,
    size_bytes: u64,
    description: String,
    keywords: Vec<String>,
    publisher: String,
    published_at: String,
    wasm_url: String,
    /// Wire signature object (JSON with sig/keyId/pubkey/signedAt); empty when unsigned.
    #[serde(skip_serializing_if = "String::is_empty")]
    signature: String,
    /// Short publisher key id; empty when unsigned.
    #[serde(skip_serializing_if = "String::is_empty")]
    key_id: String,
    /// Exact manifest JSON uploaded with the package, for offline
    /// verification of manifest_sha256; empty when not stored.
    #[serde(skip_serializing_if = "String::is_empty")]
    manifest: String,
}
#[derive(Serialize)]
struct SearchResponse {
    results: Vec<SearchHit>,
}

#[derive(Serialize)]
struct SearchHit {
    name: String,
    version: String,
    description: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    50
}

// --- Handlers ---

async fn health_check() -> &'static str {
    "ok"
}

async fn get_package_versions(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let name = decode_name(&name);
    let versions = state.database.get_package_versions(&name);

    if versions.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "NOT_FOUND".to_string(),
                        message: format!("package '{}' not found", name),
                    },
                })
                .unwrap(),
            ),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::to_value(PackageVersionsResponse { name, versions }).unwrap()),
    )
}

async fn get_package_version(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
) -> impl IntoResponse {
    let name = decode_name(&name);

    let pkg = match state.database.get_package_version(&name, &version) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: ErrorBody {
                            code: "NOT_FOUND".to_string(),
                            message: format!("{}@{} not found", name, version),
                        },
                    })
                    .unwrap(),
                ),
            );
        }
    };

    let wasm_url = format!("/v1/packages/{}/{}/download", encode_name(&name), version);
    let keywords: Vec<String> = if pkg.keywords.is_empty() {
        vec![]
    } else {
        pkg.keywords
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    (
        StatusCode::OK,
        Json(
            serde_json::to_value(PackageMetadataResponse {
                name: pkg.name,
                version: pkg.version,
                sha256: pkg.sha256,
                size_bytes: pkg.size_bytes,
                description: pkg.description,
                keywords,
                publisher: pkg.publisher,
                published_at: pkg.published_at,
                wasm_url,
                signature: pkg.signature,
                key_id: pkg.key_id,
                manifest: pkg.manifest,
            })
            .unwrap(),
        ),
    )
}

async fn download_package(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
) -> impl IntoResponse {
    let name = decode_name(&name);

    // Storage reads are blocking file I/O: run them on the blocking pool.
    let storage_state = Arc::clone(&state);
    let storage_name = name.clone();
    let storage_version = version.clone();
    let data = tokio::task::spawn_blocking(move || {
        storage_state
            .storage
            .read_wasm(&storage_name, &storage_version)
    })
    .await
    .expect("storage read task panicked");

    match data {
        Some(data) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/wasm")],
            data,
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "NOT_FOUND".to_string(),
                        message: format!("binary not found for {}@{}", name, version),
                    },
                })
                .unwrap(),
            ),
        )
            .into_response(),
    }
}

async fn search_packages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    // rusqlite queries are blocking: run the search on the blocking pool.
    let results = tokio::task::spawn_blocking(move || state.database.search(&query.q, query.limit))
        .await
        .expect("search query task panicked");

    let hits: Vec<SearchHit> = results
        .into_iter()
        .map(|p| SearchHit {
            name: p.name,
            version: p.version,
            description: p.description,
        })
        .collect();

    Json(serde_json::to_value(SearchResponse { results: hits }).unwrap())
}

async fn publish_package(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let name = decode_name(&name);

    // Auth check
    let auth_header = match headers.get("authorization").and_then(|v| v.to_str().ok()) {
        Some(h) => h.to_string(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: ErrorBody {
                            code: "UNAUTHORIZED".to_string(),
                            message: "missing Authorization header".to_string(),
                        },
                    })
                    .unwrap(),
                ),
            );
        }
    };

    let token_record = match auth::validate_bearer(&state.database, &auth_header) {
        Some(r) => r,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: ErrorBody {
                            code: "UNAUTHORIZED".to_string(),
                            message: "invalid or revoked token".to_string(),
                        },
                    })
                    .unwrap(),
                ),
            );
        }
    };

    if !auth::token_has_scope(&token_record, &name) {
        return (
            StatusCode::FORBIDDEN,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "FORBIDDEN".to_string(),
                        message: format!(
                            "token does not have publish permission for scope '{}'",
                            name
                        ),
                    },
                })
                .unwrap(),
            ),
        );
    }

    // Check if version already exists
    if state
        .database
        .get_package_version(&name, &version)
        .is_some()
    {
        return (
            StatusCode::CONFLICT,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "DUPLICATE_VERSION".to_string(),
                        message: format!("version {} already exists for {}", version, name),
                    },
                })
                .unwrap(),
            ),
        );
    }

    // Parse multipart form
    let mut wasm_bytes: Option<Vec<u8>> = None;
    let mut manifest_json: Option<String> = None;
    let mut signature_json: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "wasm" => {
                wasm_bytes = field.bytes().await.ok().map(|b| b.to_vec());
            }
            "manifest" => {
                manifest_json = field.text().await.ok();
            }
            "signature" => {
                signature_json = field.text().await.ok();
            }
            _ => {}
        }
    }

    let wasm_data = match wasm_bytes {
        Some(d) if !d.is_empty() => d,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: ErrorBody {
                            code: "BAD_REQUEST".to_string(),
                            message: "missing 'wasm' field in multipart body".to_string(),
                        },
                    })
                    .unwrap(),
                ),
            );
        }
    };

    // Compute SHA256 — hashing the payload is CPU-bound: run it on the
    // blocking pool.
    let hash_data = wasm_data.clone();
    let sha256 = tokio::task::spawn_blocking(move || {
        let mut hasher = Sha256::new();
        hasher.update(&hash_data);
        hex::encode(hasher.finalize())
    })
    .await
    .expect("sha256 hashing task panicked");
    // Parse description/keywords from manifest
    let (description, keywords) = if let Some(json_str) = &manifest_json {
        let v: serde_json::Value = serde_json::from_str(json_str).unwrap_or_default();
        let desc = v
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let kw = v
            .get("keywords")
            .and_then(|k| k.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        (desc, kw)
    } else {
        (String::new(), String::new())
    };

    // Store wasm binary — file writes are blocking I/O: run them on the
    // blocking pool.
    let store_state = Arc::clone(&state);
    let store_name = name.clone();
    let store_version = version.clone();
    let store_data = wasm_data.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || {
        store_state
            .storage
            .store_wasm(&store_name, &store_version, &store_data)
    })
    .await
    .expect("storage write task panicked")
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "STORAGE_ERROR".to_string(),
                        message: e,
                    },
                })
                .unwrap(),
            ),
        );
    }

    // Extract the short key id from the signature wire object for display
    // and indexing. The full signature object is stored verbatim so clients
    // can verify offline (spec #21: the registry is not the trust anchor).
    let key_id = signature_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v.get("keyId").and_then(|k| k.as_str()).map(str::to_string))
        .unwrap_or_default();

    // Insert into database — rusqlite is blocking: run it on the blocking
    // pool.
    let pkg = PackageVersion {
        name: name.clone(),
        version: version.clone(),
        sha256,
        size_bytes: wasm_data.len() as u64,
        description,
        keywords,
        publisher: token_record.label.clone(),
        published_at: chrono::Utc::now().to_rfc3339(),
        signature: signature_json.unwrap_or_default(),
        key_id: key_id.clone(),
        manifest: manifest_json.unwrap_or_default(),
    };

    let insert_state = Arc::clone(&state);
    let insert_pkg = pkg.clone();
    if let Err(e) =
        tokio::task::spawn_blocking(move || insert_state.database.insert_package(&insert_pkg))
            .await
            .expect("database insert task panicked")
    {
        return (
            StatusCode::CONFLICT,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "DUPLICATE_VERSION".to_string(),
                        message: e,
                    },
                })
                .unwrap(),
            ),
        );
    }

    tracing::info!("published {}@{} ({} bytes)", name, version, wasm_data.len());

    (
        StatusCode::CREATED,
        Json(
            serde_json::to_value(serde_json::json!({
                "name": name,
                "version": version,
                "sha256": pkg.sha256,
                "size_bytes": pkg.size_bytes,
                "key_id": pkg.key_id,
            }))
            .unwrap(),
        ),
    )
}

async fn yank_package(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let name = decode_name(&name);

    let auth_header = match headers.get("authorization").and_then(|v| v.to_str().ok()) {
        Some(h) => h.to_string(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: ErrorBody {
                            code: "UNAUTHORIZED".to_string(),
                            message: "missing auth".to_string(),
                        },
                    })
                    .unwrap(),
                ),
            );
        }
    };

    let token_record = match auth::validate_bearer(&state.database, &auth_header) {
        Some(r) => r,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: ErrorBody {
                            code: "UNAUTHORIZED".to_string(),
                            message: "invalid token".to_string(),
                        },
                    })
                    .unwrap(),
                ),
            );
        }
    };

    if !auth::token_has_scope(&token_record, &name) {
        return (
            StatusCode::FORBIDDEN,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "FORBIDDEN".to_string(),
                        message: "insufficient scope".to_string(),
                    },
                })
                .unwrap(),
            ),
        );
    }

    if state.database.yank_version(&name, &version) {
        tracing::info!("yanked {}@{}", name, version);
        (
            StatusCode::OK,
            Json(serde_json::to_value(serde_json::json!({"yanked": true})).unwrap()),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "NOT_FOUND".to_string(),
                        message: format!("{}@{} not found", name, version),
                    },
                })
                .unwrap(),
            ),
        )
    }
}

async fn verify_auth(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let auth_header = match headers.get("authorization").and_then(|v| v.to_str().ok()) {
        Some(h) => h.to_string(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: ErrorBody {
                            code: "UNAUTHORIZED".to_string(),
                            message: "missing auth header".to_string(),
                        },
                    })
                    .unwrap(),
                ),
            );
        }
    };

    match auth::validate_bearer(&state.database, &auth_header) {
        Some(record) => (
            StatusCode::OK,
            Json(
                serde_json::to_value(serde_json::json!({
                    "valid": true,
                    "scope": record.scope,
                    "label": record.label,
                }))
                .unwrap(),
            ),
        ),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "UNAUTHORIZED".to_string(),
                        message: "invalid or revoked token".to_string(),
                    },
                })
                .unwrap(),
            ),
        ),
    }
}

fn decode_name(encoded: &str) -> String {
    encoded.replace("%2F", "/").replace("%2f", "/")
}

fn encode_name(name: &str) -> String {
    name.replace('/', "%2F")
}
