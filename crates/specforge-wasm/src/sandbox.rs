use std::path::Path;

use crate::manifest::SandboxPolicy;

/// Merge a package's declared sandbox policy with project-level overrides.
///
/// Project overrides can further restrict the package's policy but never expand it.
pub fn merge_sandbox_policy(
    manifest_policy: &SandboxPolicy,
    project_overrides: Option<&SandboxPolicy>,
) -> SandboxPolicy {
    let Some(overrides) = project_overrides else {
        return manifest_policy.clone();
    };

    SandboxPolicy {
        max_memory_bytes: manifest_policy
            .max_memory_bytes
            .min(overrides.max_memory_bytes),
        max_fuel: if overrides.max_fuel > 0 {
            if manifest_policy.max_fuel > 0 {
                manifest_policy.max_fuel.min(overrides.max_fuel)
            } else {
                overrides.max_fuel
            }
        } else {
            manifest_policy.max_fuel
        },
        allowed_domains: if overrides.allowed_domains.is_empty() {
            manifest_policy.allowed_domains.clone()
        } else {
            // Intersection: only domains in both lists
            manifest_policy
                .allowed_domains
                .iter()
                .filter(|d| overrides.allowed_domains.contains(d))
                .cloned()
                .collect()
        },
        allow_emit_file: manifest_policy.allow_emit_file && overrides.allow_emit_file,
        allow_http: manifest_policy.allow_http && overrides.allow_http,
    }
}

/// Validate that a file path is within the allowed output directory.
///
/// Prevents path traversal attacks (e.g., `../../etc/passwd`).
pub fn validate_file_path(
    path: &str,
    policy: &SandboxPolicy,
    output_dir: &Path,
) -> Result<(), String> {
    if !policy.allow_emit_file {
        return Err("file emission is disabled by sandbox policy".to_string());
    }

    let normalized = Path::new(path);

    // Reject absolute paths
    if normalized.is_absolute() {
        return Err(format!("absolute paths are not allowed: {path}"));
    }

    // Reject path traversal
    for component in normalized.components() {
        if let std::path::Component::ParentDir = component {
            return Err(format!("path traversal is not allowed: {path}"));
        }
    }

    // Verify the resolved path stays within output_dir
    let resolved = output_dir.join(normalized);
    if !resolved.starts_with(output_dir) {
        return Err(format!(
            "path escapes output directory: {path} -> {}",
            resolved.display()
        ));
    }

    Ok(())
}

/// Validate that a URL's domain is in the sandbox allowlist.
pub fn validate_domain(url: &str, policy: &SandboxPolicy) -> Result<(), String> {
    if !policy.allow_http {
        return Err("HTTP access is disabled by sandbox policy".to_string());
    }

    if policy.allowed_domains.is_empty() {
        // No allowlist means all domains allowed (when allow_http is true)
        return Ok(());
    }

    // Extract domain from URL
    let domain = extract_domain(url).ok_or_else(|| format!("invalid URL: {url}"))?;

    if policy.allowed_domains.iter().any(|d| d == &domain) {
        Ok(())
    } else {
        Err(format!(
            "domain `{domain}` is not in allowed domains: [{}]",
            policy.allowed_domains.join(", ")
        ))
    }
}

/// Extract the domain from a URL string.
fn extract_domain(url: &str) -> Option<String> {
    // Simple extraction: skip scheme, take until port/path
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let domain = after_scheme
        .split('/')
        .next()?
        .split(':')
        .next()?;
    Some(domain.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy() -> SandboxPolicy {
        SandboxPolicy {
            max_memory_bytes: 128 * 1024 * 1024,
            max_fuel: 1_000_000,
            allowed_domains: vec!["api.example.com".to_string()],
            allow_emit_file: true,
            allow_http: true,
        }
    }

    #[test]
    fn merge_no_overrides() {
        let policy = test_policy();
        let merged = merge_sandbox_policy(&policy, None);
        assert_eq!(merged.max_memory_bytes, policy.max_memory_bytes);
        assert_eq!(merged.max_fuel, policy.max_fuel);
    }

    #[test]
    fn merge_restricts_memory() {
        let policy = test_policy();
        let overrides = SandboxPolicy {
            max_memory_bytes: 64 * 1024 * 1024,
            ..SandboxPolicy::default()
        };
        let merged = merge_sandbox_policy(&policy, Some(&overrides));
        assert_eq!(merged.max_memory_bytes, 64 * 1024 * 1024);
    }

    #[test]
    fn merge_intersection_domains() {
        let policy = SandboxPolicy {
            allowed_domains: vec!["a.com".to_string(), "b.com".to_string()],
            allow_http: true,
            ..SandboxPolicy::default()
        };
        let overrides = SandboxPolicy {
            allowed_domains: vec!["b.com".to_string(), "c.com".to_string()],
            allow_http: true,
            ..SandboxPolicy::default()
        };
        let merged = merge_sandbox_policy(&policy, Some(&overrides));
        assert_eq!(merged.allowed_domains, vec!["b.com"]);
    }

    #[test]
    fn merge_disables_http() {
        let policy = SandboxPolicy {
            allow_http: true,
            ..SandboxPolicy::default()
        };
        let overrides = SandboxPolicy {
            allow_http: false,
            ..SandboxPolicy::default()
        };
        let merged = merge_sandbox_policy(&policy, Some(&overrides));
        assert!(!merged.allow_http);
    }

    #[test]
    fn validate_file_path_normal() {
        let policy = test_policy();
        let out = Path::new("/output");
        assert!(validate_file_path("src/main.rs", &policy, out).is_ok());
    }

    #[test]
    fn validate_file_path_absolute_rejected() {
        let policy = test_policy();
        let out = Path::new("/output");
        assert!(validate_file_path("/etc/passwd", &policy, out).is_err());
    }

    #[test]
    fn validate_file_path_traversal_rejected() {
        let policy = test_policy();
        let out = Path::new("/output");
        assert!(validate_file_path("../../../etc/passwd", &policy, out).is_err());
    }

    #[test]
    fn validate_file_path_disabled() {
        let policy = SandboxPolicy {
            allow_emit_file: false,
            ..SandboxPolicy::default()
        };
        let out = Path::new("/output");
        assert!(validate_file_path("safe.rs", &policy, out).is_err());
    }

    #[test]
    fn validate_domain_allowed() {
        let policy = test_policy();
        assert!(validate_domain("https://api.example.com/v1/data", &policy).is_ok());
    }

    #[test]
    fn validate_domain_not_allowed() {
        let policy = test_policy();
        assert!(validate_domain("https://evil.com/steal", &policy).is_err());
    }

    #[test]
    fn validate_domain_http_disabled() {
        let policy = SandboxPolicy {
            allow_http: false,
            ..SandboxPolicy::default()
        };
        assert!(validate_domain("https://api.example.com", &policy).is_err());
    }

    #[test]
    fn validate_domain_empty_allowlist_allows_all() {
        let policy = SandboxPolicy {
            allow_http: true,
            allowed_domains: vec![],
            ..SandboxPolicy::default()
        };
        assert!(validate_domain("https://anything.com", &policy).is_ok());
    }

    #[test]
    fn extract_domain_https() {
        assert_eq!(
            extract_domain("https://api.example.com/v1"),
            Some("api.example.com".to_string())
        );
    }

    #[test]
    fn extract_domain_with_port() {
        assert_eq!(
            extract_domain("https://localhost:8080/path"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn extract_domain_invalid() {
        assert_eq!(extract_domain("not-a-url"), None);
    }
}
