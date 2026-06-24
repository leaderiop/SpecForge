use axum::{
    Router,
    extract::{Path, Query, State, Multipart},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Json},
    routing::{get, put, post, delete},
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
        .route("/v1/packages/{name}/{version}", put(publish_package))
        .route("/v1/packages/{name}/{version}", delete(yank_package))
        .route("/v1/packages/{name}/{version}/download", get(download_package))
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
            Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody {
                    code: "NOT_FOUND".to_string(),
                    message: format!("package '{}' not found", name),
                },
            }).unwrap()),
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
                Json(serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "NOT_FOUND".to_string(),
                        message: format!("{}@{} not found", name, version),
                    },
                }).unwrap()),
            );
        }
    };

    let wasm_url = format!("/v1/packages/{}/{}/download", encode_name(&name), version);
    let keywords: Vec<String> = if pkg.keywords.is_empty() {
        vec![]
    } else {
        pkg.keywords.split(',').map(|s| s.trim().to_string()).collect()
    };

    (
        StatusCode::OK,
        Json(serde_json::to_value(PackageMetadataResponse {
            name: pkg.name,
            version: pkg.version,
            sha256: pkg.sha256,
            size_bytes: pkg.size_bytes,
            description: pkg.description,
            keywords,
            publisher: pkg.publisher,
            published_at: pkg.published_at,
            wasm_url,
        }).unwrap()),
    )
}

async fn download_package(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
) -> impl IntoResponse {
    let name = decode_name(&name);

    match state.storage.read_wasm(&name, &version) {
        Some(data) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/wasm")],
            data,
        ).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody {
                    code: "NOT_FOUND".to_string(),
                    message: format!("binary not found for {}@{}", name, version),
                },
            }).unwrap()),
        ).into_response(),
    }
}

async fn search_packages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let results = state.database.search(&query.q, query.limit);

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
                Json(serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "UNAUTHORIZED".to_string(),
                        message: "missing Authorization header".to_string(),
                    },
                }).unwrap()),
            );
        }
    };

    let token_record = match auth::validate_bearer(&state.database, &auth_header) {
        Some(r) => r,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "UNAUTHORIZED".to_string(),
                        message: "invalid or revoked token".to_string(),
                    },
                }).unwrap()),
            );
        }
    };

    if !auth::token_has_scope(&token_record, &name) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody {
                    code: "FORBIDDEN".to_string(),
                    message: format!("token does not have publish permission for scope '{}'", name),
                },
            }).unwrap()),
        );
    }

    // Check if version already exists
    if state.database.get_package_version(&name, &version).is_some() {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody {
                    code: "DUPLICATE_VERSION".to_string(),
                    message: format!("version {} already exists for {}", version, name),
                },
            }).unwrap()),
        );
    }

    // Parse multipart form
    let mut wasm_bytes: Option<Vec<u8>> = None;
    let mut manifest_json: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "wasm" => {
                wasm_bytes = field.bytes().await.ok().map(|b| b.to_vec());
            }
            "manifest" => {
                manifest_json = field.text().await.ok();
            }
            _ => {}
        }
    }

    let wasm_data = match wasm_bytes {
        Some(d) if !d.is_empty() => d,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::to_value(ErrorResponse {
                    error: ErrorBody {
                        code: "BAD_REQUEST".to_string(),
                        message: "missing 'wasm' field in multipart body".to_string(),
                    },
                }).unwrap()),
            );
        }
    };

    // Compute SHA256
    let mut hasher = Sha256::new();
    hasher.update(&wasm_data);
    let sha256 = hex::encode(hasher.finalize());

    // Parse description/keywords from manifest
    let (description, keywords) = if let Some(ref json_str) = manifest_json {
        let v: serde_json::Value = serde_json::from_str(json_str).unwrap_or_default();
        let desc = v.get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let kw = v.get("keywords")
            .and_then(|k| k.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(","))
            .unwrap_or_default();
        (desc, kw)
    } else {
        (String::new(), String::new())
    };

    // Store wasm binary
    if let Err(e) = state.storage.store_wasm(&name, &version, &wasm_data) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody {
                    code: "STORAGE_ERROR".to_string(),
                    message: e,
                },
            }).unwrap()),
        );
    }

    // Insert into database
    let pkg = PackageVersion {
        name: name.clone(),
        version: version.clone(),
        sha256,
        size_bytes: wasm_data.len() as u64,
        description,
        keywords,
        publisher: token_record.label.clone(),
        published_at: chrono::Utc::now().to_rfc3339(),
    };

    if let Err(e) = state.database.insert_package(&pkg) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody {
                    code: "DUPLICATE_VERSION".to_string(),
                    message: e,
                },
            }).unwrap()),
        );
    }

    tracing::info!("published {}@{} ({} bytes)", name, version, wasm_data.len());

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(serde_json::json!({
            "name": name,
            "version": version,
            "sha256": pkg.sha256,
            "size_bytes": pkg.size_bytes,
        })).unwrap()),
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
            return (StatusCode::UNAUTHORIZED, Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody { code: "UNAUTHORIZED".to_string(), message: "missing auth".to_string() },
            }).unwrap()));
        }
    };

    let token_record = match auth::validate_bearer(&state.database, &auth_header) {
        Some(r) => r,
        None => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody { code: "UNAUTHORIZED".to_string(), message: "invalid token".to_string() },
            }).unwrap()));
        }
    };

    if !auth::token_has_scope(&token_record, &name) {
        return (StatusCode::FORBIDDEN, Json(serde_json::to_value(ErrorResponse {
            error: ErrorBody { code: "FORBIDDEN".to_string(), message: "insufficient scope".to_string() },
        }).unwrap()));
    }

    if state.database.yank_version(&name, &version) {
        tracing::info!("yanked {}@{}", name, version);
        (StatusCode::OK, Json(serde_json::to_value(serde_json::json!({"yanked": true})).unwrap()))
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::to_value(ErrorResponse {
            error: ErrorBody { code: "NOT_FOUND".to_string(), message: format!("{}@{} not found", name, version) },
        }).unwrap()))
    }
}

async fn verify_auth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let auth_header = match headers.get("authorization").and_then(|v| v.to_str().ok()) {
        Some(h) => h.to_string(),
        None => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody { code: "UNAUTHORIZED".to_string(), message: "missing auth header".to_string() },
            }).unwrap()));
        }
    };

    match auth::validate_bearer(&state.database, &auth_header) {
        Some(record) => {
            (StatusCode::OK, Json(serde_json::to_value(serde_json::json!({
                "valid": true,
                "scope": record.scope,
                "label": record.label,
            })).unwrap()))
        }
        None => {
            (StatusCode::UNAUTHORIZED, Json(serde_json::to_value(ErrorResponse {
                error: ErrorBody { code: "UNAUTHORIZED".to_string(), message: "invalid or revoked token".to_string() },
            }).unwrap()))
        }
    }
}

fn decode_name(encoded: &str) -> String {
    encoded.replace("%2F", "/").replace("%2f", "/")
}

fn encode_name(name: &str) -> String {
    name.replace('/', "%2F")
}
