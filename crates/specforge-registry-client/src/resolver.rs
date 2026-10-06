use semver::{Version, VersionReq};
use specforge_common::{Diagnostic, codes};

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
        RegistryError::NotFound { .. } => Diagnostic::new(
            codes::R_RES_001,
            format!(
                "package '{}' not found in registry '{}'",
                name, registry.alias
            ),
        )
        .with_suggestion("check the package name and registry configuration".to_string()),
        other => other.to_diagnostic(),
    })?;

    if versions.is_empty() {
        return Err(Diagnostic::new(
            codes::R_RES_002,
            format!("no versions published for '{}'", name),
        ));
    }

    if range == "latest" || range == "*" {
        return pick_highest(&versions, name);
    }

    let req = VersionReq::parse(range).map_err(|e| {
        Diagnostic::new(
            codes::R_RES_003,
            format!("invalid version range '{}': {}", range, e),
        )
        .with_suggestion("use semver syntax: ^1.0, ~2.3, >=1.0.0 <2.0.0".to_string())
    })?;

    let mut matching: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .collect();

    matching.sort();

    matching.last().map(|v| v.to_string()).ok_or_else(|| {
        Diagnostic::new(
            codes::R_RES_004,
            format!(
                "no version of '{}' satisfies range '{}'. Available: {}",
                name,
                range,
                versions.join(", ")
            ),
        )
        .with_suggestion("try a different version range or check available versions".to_string())
    })
}

fn pick_highest(versions: &[String], name: &str) -> Result<String, Diagnostic> {
    let mut parsed: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .collect();

    parsed.sort();

    parsed.last().map(|v| v.to_string()).ok_or_else(|| {
        Diagnostic::new(
            codes::R_RES_002,
            format!("no valid semver versions found for '{}'", name),
        )
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
