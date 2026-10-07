//! Which version of a registry package a requirement asks for (ADR 0036).
//!
//! The `Registry` port lists a package's versions and fetches one; the rule
//! that chooses among them is [`VersionRequirement::pick`], run here for
//! `add` and `update`, so production and every test fake resolve alike.

use crate::OpError;
use crate::registry::Registry;
use specforge_common::codes;
use specforge_protocol_types::PackageRef;
use specforge_protocol_types::package::{PackageName, Version, VersionRequirement};

/// The version `package` asks for. An exact version asks nothing of the
/// registry; anything else lists the package's versions and picks.
///
/// R-RES-002 when the package publishes none, R-RES-004 (naming the
/// requirement and the published versions) when none qualifies.
pub fn resolve(registry: &dyn Registry, package: &PackageRef) -> Result<Version, OpError> {
    resolve_requirement(registry, &package.name, &package.requirement)
}

/// [`resolve`] for a name and a requirement held apart (`update` asks for
/// `^current`, or the latest).
pub fn resolve_requirement(
    registry: &dyn Registry,
    name: &PackageName,
    requirement: &VersionRequirement,
) -> Result<Version, OpError> {
    if let Some(exact) = requirement.exact() {
        return Ok(exact.clone());
    }
    let published = registry.versions(name)?;
    if published.is_empty() {
        return Err(OpError::diagnostic(
            codes::R_RES_002,
            format!("no versions published for '{name}'"),
        ));
    }
    requirement.pick(&published).cloned().ok_or_else(|| {
        let available: Vec<String> = published.iter().map(Version::to_string).collect();
        OpError::diagnostic(
            codes::R_RES_004,
            format!(
                "no version of '{name}' satisfies '{requirement}'. Available: {}",
                available.join(", ")
            ),
        )
        .with_suggestion("try a different version requirement or check available versions")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::Trust;
    use crate::registry::Package;
    use specforge_test_macros::test as specforge_test;
    use std::cell::Cell;

    /// A registry publishing `versions` of every package, counting its
    /// listings.
    struct Publishing {
        versions: Vec<Version>,
        listed: Cell<usize>,
    }

    impl Publishing {
        fn of(versions: &[&str]) -> Self {
            Publishing {
                versions: versions
                    .iter()
                    .map(|v| Version::parse(v).unwrap())
                    .collect(),
                listed: Cell::new(0),
            }
        }
    }

    impl Registry for Publishing {
        fn versions(&self, _: &PackageName) -> Result<Vec<Version>, OpError> {
            self.listed.set(self.listed.get() + 1);
            Ok(self.versions.clone())
        }

        fn fetch(
            &self,
            _: &PackageName,
            _: &Version,
            _: bool,
            _: Trust,
        ) -> Result<Package, OpError> {
            unreachable!("resolving a requirement fetches nothing")
        }
    }

    #[specforge_test(
        behavior = "upgrade_wasm_extension",
        verify = "one rule picks the version a requirement asks for"
    )]
    fn resolve_asks_nothing_for_an_exact_version() {
        let registry = Publishing::of(&["1.0.0"]);

        let version = resolve(&registry, &PackageRef::parse("@acme/tool@2.5.0").unwrap()).unwrap();

        assert_eq!(version.to_string(), "2.5.0");
        assert_eq!(registry.listed.get(), 0);
    }

    #[specforge_test(
        behavior = "upgrade_wasm_extension",
        verify = "one rule picks the version a requirement asks for"
    )]
    fn resolve_picks_from_the_published_versions() {
        let registry = Publishing::of(&["1.4.0", "1.9.0", "2.0.0-beta.1", "2.0.0"]);
        for (reference, want) in [
            ("@acme/tool", "2.0.0"),
            ("@acme/tool@1.x", "1.9.0"),
            ("@acme/tool@^1.2", "1.9.0"),
            ("@acme/tool@1.4", "1.9.0"),
        ] {
            let version = resolve(&registry, &PackageRef::parse(reference).unwrap()).unwrap();
            assert_eq!(version.to_string(), want, "{reference}");
        }
    }

    #[specforge_test(
        behavior = "upgrade_wasm_extension",
        verify = "one rule picks the version a requirement asks for"
    )]
    fn resolve_reports_r_res_004_with_the_published_versions() {
        let registry = Publishing::of(&["1.4.0", "1.9.0"]);

        let error = resolve(&registry, &PackageRef::parse("@acme/tool@^3.0").unwrap()).unwrap_err();

        assert_eq!(error.code, "R-RES-004");
        assert!(error.message.contains("'@acme/tool'"), "{error:?}");
        assert!(error.message.contains("^3.0"), "{error:?}");
        assert!(error.message.contains("1.4.0, 1.9.0"), "{error:?}");
    }

    #[test]
    fn a_package_publishing_nothing_is_r_res_002() {
        let registry = Publishing::of(&[]);

        let error = resolve(&registry, &PackageRef::parse("@acme/tool").unwrap()).unwrap_err();

        assert_eq!(error.code, "R-RES-002");
    }
}
