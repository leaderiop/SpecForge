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
            message: format!(
                "package '{}' not found in registry '{}'",
                name, registry.alias
            ),
            span: None,
            suggestion: Some("check the package name and registry configuration".to_string()),
            data: None,
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
            data: None,
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
        data: None,
    })?;

    let mut matching: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .collect();

    matching.sort();

    matching
        .last()
        .map(|v| v.to_string())
        .ok_or_else(|| Diagnostic {
            code: "R-RES-004".to_string(),
            severity: Severity::Error,
            message: format!(
                "no version of '{}' satisfies range '{}'. Available: {}",
                name,
                range,
                versions.join(", ")
            ),
            span: None,
            suggestion: Some(
                "try a different version range or check available versions".to_string(),
            ),
            data: None,
        })
}

/// Resolve a version diamond: several requirers each declare a semver range
/// for the *same* package. Unify by intersecting the ranges against the
/// registry's actual published versions — pick the highest version that
/// satisfies every requirer's range simultaneously.
///
/// This is intersection-based unification, not general backtracking: if no
/// single version satisfies every requirer, resolution fails with a diagnostic
/// naming each conflicting requirer rather than searching for an alternative
/// combination of *other* packages' versions (that broader search is out of
/// scope for v1 — see the version-diamond-resolution ADR).
pub fn resolve_diamond(
    name: &str,
    requirers: &[(String, String)],
    client: &HttpRegistryClient,
    registry: &RegistryConfig,
) -> Result<String, Diagnostic> {
    let versions = client.fetch_versions(name, registry).map_err(|e| match e {
        RegistryError::NotFound { .. } => Diagnostic {
            code: "R-RES-001".to_string(),
            severity: Severity::Error,
            message: format!(
                "package '{}' not found in registry '{}'",
                name, registry.alias
            ),
            span: None,
            suggestion: Some("check the package name and registry configuration".to_string()),
            data: None,
        },
        other => other.to_diagnostic(),
    })?;

    unify_diamond(name, &versions, requirers)
}

/// Pure version of [`resolve_diamond`]: takes the candidate version list
/// directly instead of fetching it, so the unification logic is testable
/// without a registry client.
pub fn unify_diamond(
    name: &str,
    versions: &[String],
    requirers: &[(String, String)],
) -> Result<String, Diagnostic> {
    let mut reqs = Vec::with_capacity(requirers.len());
    for (requirer, range) in requirers {
        let req = VersionReq::parse(range).map_err(|e| Diagnostic {
            code: "R-RES-003".to_string(),
            severity: Severity::Error,
            message: format!(
                "'{}' declares an invalid version range '{}' for peer '{}': {}",
                requirer, range, name, e
            ),
            span: None,
            suggestion: Some("use semver syntax: ^1.0, ~2.3, >=1.0.0 <2.0.0".to_string()),
            data: None,
        })?;
        reqs.push((requirer.as_str(), range.as_str(), req));
    }

    let mut candidates: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .collect();
    candidates.sort();

    let unified = candidates
        .into_iter()
        .rev()
        .find(|v| reqs.iter().all(|(_, _, req)| req.matches(v)));

    unified.map(|v| v.to_string()).ok_or_else(|| {
        let wanted = reqs
            .iter()
            .map(|(requirer, range, _)| format!("{} wants {} {}", requirer, name, range))
            .collect::<Vec<_>>()
            .join("; ");
        Diagnostic {
            code: "R-RES-005".to_string(),
            severity: Severity::Error,
            message: format!(
                "version diamond for '{}': no single version satisfies every requirer ({}). Available: {}",
                name,
                wanted,
                versions.join(", ")
            ),
            span: None,
            suggestion: Some(
                "no version unifies these ranges; upgrade the requirer with the narrowest range or pin a compatible peer version manually".to_string(),
            ),
            data: None,
        }
    })
}

fn pick_highest(versions: &[String], name: &str) -> Result<String, Diagnostic> {
    let mut parsed: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .collect();

    parsed.sort();

    parsed
        .last()
        .map(|v| v.to_string())
        .ok_or_else(|| Diagnostic {
            code: "R-RES-002".to_string(),
            severity: Severity::Error,
            message: format!("no valid semver versions found for '{}'", name),
            span: None,
            suggestion: None,
            data: None,
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

    fn versions(vs: &[&str]) -> Vec<String> {
        vs.iter().map(|v| v.to_string()).collect()
    }

    fn req(requirer: &str, range: &str) -> (String, String) {
        (requirer.to_string(), range.to_string())
    }

    // -- unify_diamond --

    #[test]
    fn unify_diamond_picks_highest_version_satisfying_every_requirer() {
        let vs = versions(&["1.0.0", "1.5.0", "2.0.0", "2.5.0", "3.0.0"]);
        let requirers = vec![req("@a/ext", "^2.0.0"), req("@b/ext", ">=2.0.0, <3.0.0")];
        let result = unify_diamond("@shared/lib", &vs, &requirers).unwrap();
        assert_eq!(result, "2.5.0");
    }

    #[test]
    fn unify_diamond_single_requirer_behaves_like_resolve_version() {
        let vs = versions(&["1.0.0", "1.2.0", "2.0.0"]);
        let requirers = vec![req("@a/ext", "^1.0.0")];
        let result = unify_diamond("@shared/lib", &vs, &requirers).unwrap();
        assert_eq!(result, "1.2.0");
    }

    #[test]
    fn unify_diamond_incompatible_ranges_reports_diamond_conflict() {
        let vs = versions(&["1.0.0", "1.5.0", "2.0.0", "2.5.0"]);
        // @a wants a 1.x line, @b wants a 2.x line — no version satisfies both.
        let requirers = vec![req("@a/ext", "^1.0.0"), req("@b/ext", "^2.0.0")];
        let err = unify_diamond("@shared/lib", &vs, &requirers).unwrap_err();
        assert_eq!(err.code, "R-RES-005");
        assert!(err.message.contains("@a/ext"));
        assert!(err.message.contains("@b/ext"));
        assert!(err.message.contains("^1.0.0"));
        assert!(err.message.contains("^2.0.0"));
    }

    #[test]
    fn unify_diamond_malformed_range_names_the_requirer() {
        let vs = versions(&["1.0.0"]);
        let requirers = vec![req("@a/ext", "not-a-range")];
        let err = unify_diamond("@shared/lib", &vs, &requirers).unwrap_err();
        assert_eq!(err.code, "R-RES-003");
        assert!(err.message.contains("@a/ext"));
    }

    #[test]
    fn unify_diamond_no_requirers_picks_highest_available() {
        let vs = versions(&["1.0.0", "2.0.0"]);
        let result = unify_diamond("@shared/lib", &vs, &[]).unwrap();
        assert_eq!(result, "2.0.0");
    }
}
