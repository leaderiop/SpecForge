use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;

use super::registry_client::{RegistryClient, RegistryError, RegistryResponse, RegistrySearchResult};
use super::registry_config::{AuthMethod, RegistryConfig, RegistryCredential};
use crate::ManifestV2;

#[derive(Deserialize)]
struct PackageVersionResponse {
    name: String,
    version: String,
    sha256: String,
    wasm_url: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchHit>,
}

#[derive(Deserialize)]
struct SearchHit {
    name: String,
    version: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct PackageVersionsResponse {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    #[allow(dead_code)]
    code: String,
    message: String,
}

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
            AuthMethod::TokenEnvVar(var) => std::env::var(var).map_err(|_| {
                RegistryError::Unauthorized {
                    guidance: format!("environment variable '{}' not set", var),
                }
            }),
            AuthMethod::TokenFile(path) => std::fs::read_to_string(path)
                .map(|s| s.trim().to_string())
                .map_err(|_| RegistryError::Unauthorized {
                    guidance: format!("cannot read token file '{}'", path.display()),
                }),
        }
    }

    fn encode_package_name(name: &str) -> String {
        name.replace('/', "%2F")
    }

    /// Fetch all available versions for a package.
    pub fn fetch_versions(
        &self,
        name: &str,
        registry: &RegistryConfig,
    ) -> Result<Vec<String>, RegistryError> {
        let base = Self::base_url(registry);
        let encoded = Self::encode_package_name(name);
        let url = format!("{}/packages/{}", base, encoded);

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
                let body: PackageVersionsResponse =
                    resp.json().map_err(|e| RegistryError::NetworkError {
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
            429 => Err(RegistryError::RateLimited {
                retry_after_ms: 5000,
            }),
            _ => {
                let msg = resp
                    .json::<ErrorResponse>()
                    .map(|e| e.error.message)
                    .unwrap_or_else(|_| "unknown error".to_string());
                Err(RegistryError::NetworkError { message: msg })
            }
        }
    }

    /// Download the raw Wasm bytes for a specific package version.
    pub fn download_wasm(
        &self,
        wasm_url: &str,
    ) -> Result<Vec<u8>, RegistryError> {
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
            200 => resp.bytes().map(|b| b.to_vec()).map_err(|e| {
                RegistryError::NetworkError {
                    message: format!("failed to read response bytes: {}", e),
                }
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
        specifier: &str,
        registry: &RegistryConfig,
    ) -> Result<RegistryResponse, RegistryError> {
        let (name, version) = parse_specifier(specifier);
        let base = Self::base_url(registry);
        let encoded = Self::encode_package_name(&name);
        let url = format!("{}/packages/{}/{}", base, encoded, version);

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
                let body: PackageVersionResponse =
                    resp.json().map_err(|e| RegistryError::NetworkError {
                        message: format!("invalid response body: {}", e),
                    })?;
                Ok(RegistryResponse {
                    name: body.name,
                    version: body.version,
                    wasm_url: body.wasm_url,
                    sha256: body.sha256,
                })
            }
            404 => Err(RegistryError::NotFound {
                specifier: specifier.to_string(),
            }),
            401 => Err(RegistryError::Unauthorized {
                guidance: "token expired or invalid".to_string(),
            }),
            403 => Err(RegistryError::Forbidden {
                guidance: "insufficient permissions".to_string(),
            }),
            429 => Err(RegistryError::RateLimited {
                retry_after_ms: 5000,
            }),
            _ => {
                let msg = resp
                    .json::<ErrorResponse>()
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
    ) -> Result<Vec<RegistrySearchResult>, RegistryError> {
        let base = Self::base_url(registry);
        let url = format!("{}/search?q={}&limit=50", base, urlencoded(query));

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
                let body: SearchResponse =
                    resp.json().map_err(|e| RegistryError::NetworkError {
                        message: format!("invalid search response: {}", e),
                    })?;
                Ok(body
                    .results
                    .into_iter()
                    .map(|h| RegistrySearchResult {
                        name: h.name,
                        version: h.version,
                        description: h.description,
                    })
                    .collect())
            }
            429 => Err(RegistryError::RateLimited {
                retry_after_ms: 5000,
            }),
            _ => {
                let msg = resp
                    .json::<ErrorResponse>()
                    .map(|e| e.error.message)
                    .unwrap_or_else(|_| "search request failed".to_string());
                Err(RegistryError::NetworkError { message: msg })
            }
        }
    }

    fn publish(
        &self,
        package: &[u8],
        manifest: &ManifestV2,
        registry: &RegistryConfig,
    ) -> Result<String, RegistryError> {
        let base = Self::base_url(registry);
        let encoded = Self::encode_package_name(&manifest.name);
        let url = format!("{}/packages/{}/{}", base, encoded, manifest.version);

        let metadata = serde_json::to_string(manifest).map_err(|e| RegistryError::NetworkError {
            message: format!("failed to serialize manifest: {}", e),
        })?;

        let form = reqwest::blocking::multipart::Form::new()
            .text("manifest", metadata)
            .part(
                "wasm",
                reqwest::blocking::multipart::Part::bytes(package.to_vec())
                    .file_name("extension.wasm")
                    .mime_str("application/wasm")
                    .unwrap(),
            );

        let resp = self
            .client
            .put(&url)
            .multipart(form)
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
            200 | 201 => Ok(url),
            401 => Err(RegistryError::Unauthorized {
                guidance: "authentication required for publishing".to_string(),
            }),
            403 => Err(RegistryError::Forbidden {
                guidance: "you don't have publish permission for this scope".to_string(),
            }),
            409 => Err(RegistryError::DuplicateVersion {
                name: manifest.name.clone(),
                version: manifest.version.clone(),
            }),
            _ => {
                let msg = resp
                    .json::<ErrorResponse>()
                    .map(|e| e.error.message)
                    .unwrap_or_else(|_| "publish failed".to_string());
                Err(RegistryError::NetworkError { message: msg })
            }
        }
    }

    fn authenticate(
        &self,
        registry: &RegistryConfig,
        credential: &RegistryCredential,
    ) -> Result<(), RegistryError> {
        let token = Self::resolve_token(credential)?;
        let base = Self::base_url(registry);
        let url = format!("{}/auth/verify", base);

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
            200 => Ok(()),
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

/// Parse a specifier like `@scope/name@1.0.0` into (name, version).
/// If no version is given, defaults to "latest".
pub fn parse_specifier(specifier: &str) -> (String, String) {
    if let Some(at_pos) = specifier.rfind('@')
        && at_pos > 0
        && !specifier[..at_pos].is_empty()
    {
        let name = &specifier[..at_pos];
        let version = &specifier[at_pos + 1..];
        if !version.is_empty() && !version.starts_with('/') {
            return (name.to_string(), version.to_string());
        }
    }
    (specifier.to_string(), "latest".to_string())
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('@', "%40")
        .replace('/', "%2F")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_specifier_with_version() {
        let (name, version) = parse_specifier("@specforge/product@1.0.0");
        assert_eq!(name, "@specforge/product");
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn parse_specifier_without_version() {
        let (name, version) = parse_specifier("@specforge/product");
        assert_eq!(name, "@specforge/product");
        assert_eq!(version, "latest");
    }

    #[test]
    fn parse_specifier_with_range() {
        let (name, version) = parse_specifier("@specforge/product@^1.0");
        assert_eq!(name, "@specforge/product");
        assert_eq!(version, "^1.0");
    }

    #[test]
    fn encode_scoped_name() {
        assert_eq!(
            HttpRegistryClient::encode_package_name("@specforge/product"),
            "@specforge%2Fproduct"
        );
    }
}
