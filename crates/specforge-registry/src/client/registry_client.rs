use specforge_common::{Diagnostic, Severity};

use super::registry_config::{RegistryConfig, RegistryCredential};
use crate::ManifestV2;

/// Response from fetching an extension package from a registry.
#[derive(Debug, Clone)]
pub struct RegistryResponse {
    pub name: String,
    pub version: String,
    pub wasm_url: String,
    pub sha256: String,
}

/// A single search result from a registry query.
#[derive(Debug, Clone)]
pub struct RegistrySearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Errors that can occur during registry operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    Unauthorized { guidance: String },
    Forbidden { guidance: String },
    RateLimited { retry_after_ms: u64 },
    Timeout { url: String },
    NetworkError { message: String },
    NotFound { specifier: String },
    DuplicateVersion { name: String, version: String },
}

impl RegistryError {
    /// Convert this error into a `Diagnostic`.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            RegistryError::Unauthorized { guidance } => Diagnostic {
                code: "R001".to_string(),
                severity: Severity::Error,
                message: format!("Registry authentication failed: {guidance}"),
                span: None,
                suggestion: Some("Run `specforge registry login` to authenticate.".to_string()),
            },
            RegistryError::Forbidden { guidance } => Diagnostic {
                code: "R002".to_string(),
                severity: Severity::Error,
                message: format!("Registry access forbidden: {guidance}"),
                span: None,
                suggestion: Some(
                    "Check your permissions for this registry or package scope.".to_string(),
                ),
            },
            RegistryError::RateLimited { retry_after_ms } => Diagnostic {
                code: "R003".to_string(),
                severity: Severity::Warning,
                message: format!("Registry rate limited. Retry after {retry_after_ms}ms."),
                span: None,
                suggestion: None,
            },
            RegistryError::Timeout { url } => Diagnostic {
                code: "R004".to_string(),
                severity: Severity::Error,
                message: format!("Registry request timed out: {url}"),
                span: None,
                suggestion: Some("Check your network connection or try again later.".to_string()),
            },
            RegistryError::NetworkError { message } => Diagnostic {
                code: "R005".to_string(),
                severity: Severity::Error,
                message: format!("Registry network error: {message}"),
                span: None,
                suggestion: Some("Check your network connection.".to_string()),
            },
            RegistryError::NotFound { specifier } => Diagnostic {
                code: "R006".to_string(),
                severity: Severity::Error,
                message: format!("Package not found: {specifier}"),
                span: None,
                suggestion: Some("Verify the package name and version.".to_string()),
            },
            RegistryError::DuplicateVersion { name, version } => Diagnostic {
                code: "R007".to_string(),
                severity: Severity::Error,
                message: format!("Version {version} already exists for package {name}."),
                span: None,
                suggestion: Some("Bump the version number before publishing.".to_string()),
            },
        }
    }
}

impl From<RegistryError> for Diagnostic {
    fn from(err: RegistryError) -> Self {
        err.to_diagnostic()
    }
}

/// Trait for interacting with extension registries.
///
/// Implementations handle the transport layer (HTTP, file system, etc.)
/// for fetching, searching, publishing, and authenticating with registries.
pub trait RegistryClient: Send + Sync {
    /// Fetch a specific extension package from the registry.
    fn fetch(
        &self,
        specifier: &str,
        registry: &RegistryConfig,
    ) -> Result<RegistryResponse, RegistryError>;

    /// Search for extensions matching a query string.
    fn search(
        &self,
        query: &str,
        registry: &RegistryConfig,
    ) -> Result<Vec<RegistrySearchResult>, RegistryError>;

    /// Publish an extension package (Wasm binary + manifest) to the registry.
    fn publish(
        &self,
        package: &[u8],
        manifest: &ManifestV2,
        registry: &RegistryConfig,
    ) -> Result<String, RegistryError>;

    /// Validate that the given credential authenticates successfully.
    fn authenticate(
        &self,
        registry: &RegistryConfig,
        credential: &RegistryCredential,
    ) -> Result<(), RegistryError>;
}

/// Retry policy for registry operations using exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub max_retries: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
            max_retries: 3,
        }
    }
}

impl RetryPolicy {
    /// Calculate the delay in milliseconds for a given attempt (0-indexed).
    ///
    /// Uses exponential backoff: `base_delay_ms * 2^attempt`, capped at `max_delay_ms`.
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.base_delay_ms.saturating_mul(2u64.saturating_pow(attempt));
        delay.min(self.max_delay_ms)
    }
}
