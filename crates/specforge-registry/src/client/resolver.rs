use semver::{Version, VersionReq};
use specforge_common::{Diagnostic, Severity};

use super::http_client::HttpRegistryClient;
use super::registry_client::RegistryError;
use super::registry_config::RegistryConfig;

/// Resolve a version range against a registry, returning the highest matching version.
///
/// If `range` is "latest", returns the highest available version.
/// Otherwise, parses `range` as a semver requirement and picks the best match.
pub fn resolve_version(
    name: &str,
    range: &str,
    client: &HttpRegistryClient,
    registry: &RegistryConfig,
) -> Result<String, Diagnostic> {
    let versions = client.fetch_versions(name, registry).map_err(|e| match e {
        RegistryError::NotFound { .. } => Diagnostic {
            code: "R-RES-001".to_string(),
            severity: Severity::Error,
            message: format!("package '{}' not found in registry '{}'", name, registry.alias),
            span: None,
            suggestion: Some("check the package name and registry configuration".to_string()),
        },
        other => other.to_diagnostic(),
    })?;

    if versions.is_empty() {
        return Err(Diagnostic {
            code: "R-RES-002".to_string(),
            severity: Severity::Error,
            message: format!("no versions published for '{}'", name),
            span: None,
            suggestion: None,
        });
    }

    if range == "latest" || range == "*" {
        return pick_highest(&versions, name);
    }

    let req = VersionReq::parse(range).map_err(|e| Diagnostic {
        code: "R-RES-003".to_string(),
        severity: Severity::Error,
        message: format!("invalid version range '{}': {}", range, e),
        span: None,
        suggestion: Some("use semver syntax: ^1.0, ~2.3, >=1.0.0 <2.0.0".to_string()),
    })?;

    let mut matching: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .collect();

    matching.sort();

    matching.last().map(|v| v.to_string()).ok_or_else(|| Diagnostic {
        code: "R-RES-004".to_string(),
        severity: Severity::Error,
        message: format!(
            "no version of '{}' satisfies range '{}'. Available: {}",
            name,
            range,
            versions.join(", ")
        ),
        span: None,
        suggestion: Some("try a different version range or check available versions".to_string()),
    })
}

fn pick_highest(versions: &[String], name: &str) -> Result<String, Diagnostic> {
    let mut parsed: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .collect();

    parsed.sort();

    parsed.last().map(|v| v.to_string()).ok_or_else(|| Diagnostic {
        code: "R-RES-002".to_string(),
        severity: Severity::Error,
        message: format!("no valid semver versions found for '{}'", name),
        span: None,
        suggestion: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_highest_selects_largest_version() {
        let versions = vec![
            "1.0.0".to_string(),
            "2.1.0".to_string(),
            "1.5.3".to_string(),
            "2.0.0".to_string(),
        ];
        let result = pick_highest(&versions, "test").unwrap();
        assert_eq!(result, "2.1.0");
    }

    #[test]
    fn pick_highest_empty_returns_error() {
        let result = pick_highest(&[], "test");
        assert!(result.is_err());
    }
}
