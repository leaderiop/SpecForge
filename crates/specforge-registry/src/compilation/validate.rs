use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::ExtensionDeclaration;

/// E027/W062 for every declaration's peers, against the loaded ones.
pub(crate) fn peer_dependencies(declarations: &[ExtensionDeclaration]) -> Vec<Diagnostic> {
    declarations
        .iter()
        .flat_map(|declaration| peer_dependencies_of(declaration, declarations))
        .collect()
}

/// The peer diagnostics of `declaration` against the `installed` ones
/// (`declaration` itself may be among them).
fn peer_dependencies_of(
    declaration: &ExtensionDeclaration,
    installed: &[ExtensionDeclaration],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let name = declaration.name();

    let installed: std::collections::HashMap<&str, &str> =
        installed.iter().map(|d| (d.name(), d.version())).collect();

    for peer in declaration.peers() {
        match installed.get(peer.name.as_str()) {
            None if peer.optional => {}
            None => {
                diagnostics.push(Diagnostic {
                    code: "E027".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "extension '{}' requires peer dependency '{}' {} which is not installed",
                        name, peer.name, peer.version
                    ),
                    span: None,
                    suggestion: Some(format!("install it with: specforge add {}", peer.name)),
                    data: None,
                    origin: None,
                });
            }
            Some(installed_version) => {
                // Validate that both the required range and installed version are parseable semver
                let req_parse = semver::VersionReq::parse(&peer.version);
                let ver_parse = semver::Version::parse(installed_version);

                if req_parse.is_err() {
                    diagnostics.push(Diagnostic {
                        code: "W062".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "extension '{}' declares peer dependency '{}' with malformed semver range '{}'",
                            name, peer.name, peer.version
                        ),
                        span: None,
                        suggestion: Some("use a valid semver range like ^1.0.0, ~1.2.0, or >=1.0.0".to_string()),
                        data: None,
                        origin: None,
                    });
                } else if ver_parse.is_err() {
                    diagnostics.push(Diagnostic {
                        code: "W062".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "extension '{}' has malformed version '{}' (not valid semver)",
                            peer.name, installed_version
                        ),
                        span: None,
                        suggestion: Some("use a valid semver version like 1.0.0".to_string()),
                        data: None,
                        origin: None,
                    });
                } else if !version_satisfies(installed_version, &peer.version) {
                    diagnostics.push(Diagnostic {
                        code: "E027".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "extension '{}' requires peer dependency '{}' {} but version {} is installed",
                            name, peer.name, peer.version, installed_version
                        ),
                        span: None,
                        suggestion: None,
                        data: None,
                        origin: None,
                    });
                }
            }
        }
    }

    diagnostics
}

/// Check if an installed version satisfies a required version range.
/// Supports semver ranges: ^X.Y.Z, ~X.Y.Z, >=X.Y.Z, >X.Y.Z, <=X.Y.Z, <X.Y.Z, and exact X.Y.Z.
fn version_satisfies(installed: &str, required: &str) -> bool {
    let Ok(ver) = semver::Version::parse(installed) else {
        return false;
    };
    let Ok(req) = semver::VersionReq::parse(required) else {
        // Fall back to exact match for non-parseable ranges
        return installed == required;
    };
    req.matches(&ver)
}

/// W017: a kind declared `testable` that does not accept `verify`
/// statements, so its entities could never declare the obligations
/// coverage counts. [`super::build::build_registries`] runs it once the
/// kinds are populated. A kind that accepts `verify` but is not testable
/// (a formal `property`) is a deliberate combination, not reported.
pub(crate) fn validate_extension_testability(kind_reg: &crate::KindRegistry) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (_, entry) in kind_reg.iter() {
        if entry.testable && !entry.supports_verify {
            diagnostics.push(Diagnostic {
                code: "W017".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "entity kind '{}' from '{}' is testable but does not support verify statements",
                    entry.kind_name, entry.source_extension
                ),
                span: None,
                suggestion: Some(
                    "declare the kind with supports_verify (KindBuilder::supports_verify)"
                        .to_string(),
                ),
                data: None,
                origin: None,
            });
        }
    }

    // Sort for deterministic output
    diagnostics.sort_by(|a, b| a.message.cmp(&b.message));
    diagnostics
}
