use std::time::SystemTime;

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use specforge_registry_wire::{
    ErrorBody, PackageMetadata, SearchHit, SearchQuery, SearchResults, TokenVerified, VersionList,
    form, path,
};

use super::registry_client::{RegistryClient, RegistryError};
use super::registry_config::{AuthMethod, RegistryConfig, RegistryCredential};
use specforge_protocol_types::package::Version;
use specforge_protocol_types::{ExtensionDeclaration, PackageName};

pub struct HttpRegistryClient {
    client: Client,
    timeout: std::time::Duration,
}

impl HttpRegistryClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
            timeout: std::time::Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build HTTP client");
        self
    }

    fn base_url(registry: &RegistryConfig) -> String {
        registry.url.trim_end_matches('/').to_string()
    }

    fn resolve_token(credential: &RegistryCredential) -> Result<String, RegistryError> {
        match &credential.auth_method {
            AuthMethod::Bearer(token) => Ok(token.clone()),
            AuthMethod::TokenEnvVar(var) => {
                std::env::var(var).map_err(|_| RegistryError::Unauthorized {
                    guidance: format!("environment variable '{}' not set", var),
                })
            }
            AuthMethod::TokenFile(path) => std::fs::read_to_string(path)
                .map(|s| s.trim().to_string())
                .map_err(|_| RegistryError::Unauthorized {
                    guidance: format!("cannot read token file '{}'", path.display()),
                }),
        }
    }

    /// Fetch all available versions for a package.
    pub fn fetch_versions(
        &self,
        name: &PackageName,
        registry: &RegistryConfig,
    ) -> Result<Vec<String>, RegistryError> {
        let base = Self::base_url(registry);
        let url = format!("{base}{}", path::package(name));

        let resp = self.client.get(&url).send().map_err(|e| {
            if e.is_timeout() {
                RegistryError::Timeout { url: url.clone() }
            } else {
                RegistryError::NetworkError {
                    message: e.to_string(),
                }
            }
        })?;

        match resp.status().as_u16() {
            200 => {
                let body: VersionList = resp.json().map_err(|e| RegistryError::NetworkError {
                    message: format!("invalid response body: {}", e),
                })?;
                Ok(body.versions)
            }
            404 => Err(RegistryError::NotFound {
                specifier: name.to_string(),
            }),
            401 => Err(RegistryError::Unauthorized {
                guidance: "token expired or invalid".to_string(),
            }),
            429 => Err(rate_limited(&resp)),
            _ => {
                let msg = resp
                    .json::<ErrorBody>()
                    .map(|e| e.error.message)
                    .unwrap_or_else(|_| "unknown error".to_string());
                Err(RegistryError::NetworkError { message: msg })
            }
        }
    }

    /// Download the raw Wasm bytes for a specific package version.
    pub fn download_wasm(&self, wasm_url: &str) -> Result<Vec<u8>, RegistryError> {
        let resp = self.client.get(wasm_url).send().map_err(|e| {
            if e.is_timeout() {
                RegistryError::Timeout {
                    url: wasm_url.to_string(),
                }
            } else {
                RegistryError::NetworkError {
                    message: e.to_string(),
                }
            }
        })?;

        match resp.status().as_u16() {
            200 => resp
                .bytes()
                .map(|b| b.to_vec())
                .map_err(|e| RegistryError::NetworkError {
                    message: format!("failed to read response bytes: {}", e),
                }),
            404 => Err(RegistryError::NotFound {
                specifier: wasm_url.to_string(),
            }),
            _ => Err(RegistryError::NetworkError {
                message: format!("download failed with status {}", resp.status()),
            }),
        }
    }
}

impl Default for HttpRegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryClient for HttpRegistryClient {
    fn fetch(
        &self,
        name: &PackageName,
        version: &Version,
        registry: &RegistryConfig,
    ) -> Result<PackageMetadata, RegistryError> {
        let base = Self::base_url(registry);
        let url = format!("{base}{}", path::version(name, version));

        let resp = self.client.get(&url).send().map_err(|e| {
            if e.is_timeout() {
                RegistryError::Timeout { url: url.clone() }
            } else {
                RegistryError::NetworkError {
                    message: e.to_string(),
                }
            }
        })?;

        match resp.status().as_u16() {
            200 => {
                let mut body: PackageMetadata =
                    resp.json().map_err(|e| RegistryError::NetworkError {
                        message: format!("invalid response body: {}", e),
                    })?;
                if body.wasm_url.starts_with('/') {
                    // the server may return a root-relative download path;
                    // resolve it against the configured registry base
                    body.wasm_url = format!("{base}{}", body.wasm_url);
                }
                Ok(body)
            }
            404 => Err(RegistryError::NotFound {
                specifier: format!("{name}@{version}"),
            }),
            401 => Err(RegistryError::Unauthorized {
                guidance: "token expired or invalid".to_string(),
            }),
            403 => Err(RegistryError::Forbidden {
                guidance: "insufficient permissions".to_string(),
            }),
            429 => Err(rate_limited(&resp)),
            _ => {
                let msg = resp
                    .json::<ErrorBody>()
                    .map(|e| e.error.message)
                    .unwrap_or_else(|_| "unknown error".to_string());
                Err(RegistryError::NetworkError { message: msg })
            }
        }
    }

    fn search(
        &self,
        query: &str,
        registry: &RegistryConfig,
    ) -> Result<Vec<SearchHit>, RegistryError> {
        let base = Self::base_url(registry);
        let url = format!(
            "{base}{}?{}",
            path::SEARCH,
            SearchQuery::new(query).to_query_string()
        );

        let resp = self.client.get(&url).send().map_err(|e| {
            if e.is_timeout() {
                RegistryError::Timeout { url: url.clone() }
            } else {
                RegistryError::NetworkError {
                    message: e.to_string(),
                }
            }
        })?;

        match resp.status().as_u16() {
            200 => {
                let body: SearchResults = resp.json().map_err(|e| RegistryError::NetworkError {
                    message: format!("invalid search response: {}", e),
                })?;
                Ok(body.results)
            }
            429 => Err(rate_limited(&resp)),
            _ => {
                let msg = resp
                    .json::<ErrorBody>()
                    .map(|e| e.error.message)
                    .unwrap_or_else(|_| "search request failed".to_string());
                Err(RegistryError::NetworkError { message: msg })
            }
        }
    }

    fn publish(
        &self,
        package: &[u8],
        declaration: &ExtensionDeclaration,
        manifest_json: &str,
        signature: Option<&str>,
        registry: &RegistryConfig,
        credential: Option<&RegistryCredential>,
    ) -> Result<String, RegistryError> {
        let base = Self::base_url(registry);
        let name = declaration
            .package_name()
            .map_err(|why| RegistryError::InvalidPackage {
                message: why.to_string(),
            })?;
        let version =
            Version::parse(declaration.version()).map_err(|why| RegistryError::InvalidPackage {
                message: format!("'{}' is not a SemVer version: {why}", declaration.version()),
            })?;
        let url = format!("{base}{}", path::version(&name, &version));

        // Build the multipart body manually: reqwest's blocking multipart
        // wrapper can fail with a body error on large wasm parts, while an
        // explicit body is deterministic and length-known.
        let boundary = format!(
            "specforge-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let body = form::body(&boundary, manifest_json, package, signature);

        let mut request = self
            .client
            .put(&url)
            .header(CONTENT_TYPE, form::content_type(&boundary))
            .body(body);
        if let Some(credential) = credential {
            let token = Self::resolve_token(credential)?;
            request = request.header(AUTHORIZATION, format!("Bearer {}", token));
        }

        let resp = request.send().map_err(|e| {
            if e.is_timeout() {
                RegistryError::Timeout { url: url.clone() }
            } else {
                RegistryError::NetworkError {
                    message: e.to_string(),
                }
            }
        })?;

        match resp.status().as_u16() {
            200 | 201 => Ok(url),
            401 => Err(RegistryError::Unauthorized {
                guidance: "authentication required for publishing".to_string(),
            }),
            403 => Err(RegistryError::Forbidden {
                guidance: "you don't have publish permission for this scope".to_string(),
            }),
            409 => Err(RegistryError::DuplicateVersion {
                name: declaration.name().to_string(),
                version: declaration.version().to_string(),
            }),
            status => {
                let msg = resp
                    .json::<ErrorBody>()
                    .map(|e| e.error.message)
                    .unwrap_or_else(|_| format!("publish failed with status {status}"));
                Err(RegistryError::NetworkError { message: msg })
            }
        }
    }

    fn authenticate(
        &self,
        registry: &RegistryConfig,
        credential: &RegistryCredential,
    ) -> Result<Option<String>, RegistryError> {
        let token = Self::resolve_token(credential)?;
        let base = Self::base_url(registry);
        let url = format!("{base}{}", path::AUTH_VERIFY);

        let resp = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header(CONTENT_TYPE, "application/json")
            .body("{}")
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    RegistryError::Timeout { url: url.clone() }
                } else {
                    RegistryError::NetworkError {
                        message: e.to_string(),
                    }
                }
            })?;

        match resp.status().as_u16() {
            200 => {
                let body: TokenVerified = resp.json().map_err(|e| RegistryError::NetworkError {
                    message: format!("invalid auth response: {}", e),
                })?;
                Ok(body.expires_at)
            }
            401 => Err(RegistryError::Unauthorized {
                guidance: "token is invalid or expired".to_string(),
            }),
            403 => Err(RegistryError::Forbidden {
                guidance: "token does not have required permissions".to_string(),
            }),
            _ => Err(RegistryError::NetworkError {
                message: format!("auth verification returned status {}", resp.status()),
            }),
        }
    }
}

/// Fallback delay when a 429 response carries no usable `Retry-After`.
const DEFAULT_RETRY_AFTER_MS: u64 = 5000;

/// Build the `RateLimited` error from a response's `Retry-After` header.
fn rate_limited(resp: &reqwest::blocking::Response) -> RegistryError {
    let header = resp
        .headers()
        .get(RETRY_AFTER)
        .and_then(|v| v.to_str().ok());
    RegistryError::RateLimited {
        retry_after_ms: parse_retry_after_ms(header, SystemTime::now()),
    }
}

/// Parse a `Retry-After` header value into a delay in milliseconds.
///
/// Per RFC 9110 the value is either delta-seconds or an HTTP-date. Falls back
/// to [`DEFAULT_RETRY_AFTER_MS`] when the header is absent or malformed. An
/// HTTP-date in the past yields `0`.
fn parse_retry_after_ms(header: Option<&str>, now: SystemTime) -> u64 {
    let Some(value) = header.map(str::trim) else {
        return DEFAULT_RETRY_AFTER_MS;
    };

    if let Ok(seconds) = value.parse::<u64>() {
        return seconds.saturating_mul(1000);
    }

    match httpdate::parse_http_date(value) {
        Ok(retry_at) => match retry_at.duration_since(now) {
            Ok(wait) => wait.as_millis() as u64,
            Err(_) => 0,
        },
        Err(_) => DEFAULT_RETRY_AFTER_MS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_delta_seconds() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        assert_eq!(parse_retry_after_ms(Some("120"), now), 120_000);
        assert_eq!(parse_retry_after_ms(Some(" 30 "), now), 30_000);
        assert_eq!(parse_retry_after_ms(Some("0"), now), 0);
    }

    #[test]
    fn retry_after_http_date() {
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        let future = now + std::time::Duration::from_secs(90);
        let past = now - std::time::Duration::from_secs(90);
        assert_eq!(
            parse_retry_after_ms(Some(&httpdate::fmt_http_date(future)), now),
            90_000
        );
        assert_eq!(
            parse_retry_after_ms(Some(&httpdate::fmt_http_date(past)), now),
            0
        );
    }

    #[test]
    fn retry_after_absent_or_malformed_falls_back_to_default() {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(parse_retry_after_ms(None, now), DEFAULT_RETRY_AFTER_MS);
        assert_eq!(
            parse_retry_after_ms(Some("soon"), now),
            DEFAULT_RETRY_AFTER_MS
        );
    }
}
