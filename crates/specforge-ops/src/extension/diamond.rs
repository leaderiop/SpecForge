//! The ADR-0001 version-diamond gate: `add` decides whether one locked
//! version can satisfy every requirer of a peer before it installs,
//! instead of leaving `doctor` to find the conflict afterwards.

use crate::{OpError, OpErrorKind};
use semver::{Version, VersionReq};
use specforge_common::{Diagnostic, codes};
use specforge_installed::LockFile;
use specforge_registry::PeerDependency;

/// Check that installing `package` (declaring `peers`) leaves every locked
/// peer satisfied. A locked peer outside a declared range is refused:
/// R-RES-006 when one registry version would satisfy every requirer (the
/// fix is to reinstall the peer at it), R-RES-005 when none would.
/// `versions` lists a peer's published versions; it is asked only for a
/// peer whose locked version falls outside a range. A malformed range or
/// locked version is left to the peer-dependency validation (W062).
pub fn check_diamonds(
    lock: &LockFile,
    package: &str,
    peers: &[PeerDependency],
    versions: &dyn Fn(&str) -> Result<Vec<String>, OpError>,
) -> Result<(), OpError> {
    for peer in peers {
        let Some(locked) = lock.entries.iter().find(|e| e.name.as_str() == peer.name) else {
            continue;
        };
        let satisfied = match (
            semver::VersionReq::parse(&peer.version),
            semver::Version::parse(&locked.version),
        ) {
            (Ok(req), Ok(version)) => req.matches(&version),
            _ => true,
        };
        if satisfied {
            continue;
        }

        let requirers = lock.requirers_of(&peer.name, Some((package, &peer.version)));
        let published = versions(&peer.name)?;
        return match unify_diamond(&peer.name, &published, &requirers)
        {
            Ok(unified) => Err(OpError::coded(
                OpErrorKind::Conflict,
                codes::R_RES_006,
                format!(
                    "version diamond: '{package}' requires peer '{}' {} but {} is locked; {} {unified} would satisfy every requirer",
                    peer.name, peer.version, locked.version, peer.name
                ),
            )
            .with_suggestion(format!(
                "no command pins peer versions yet; manually reinstall '{}' at {unified} (or a version satisfying every requirer), then retry add",
                peer.name
            ))),
            Err(diagnostic) => Err(OpError::from(diagnostic)),
        };
    }
    Ok(())
}

/// Unify a version diamond: several requirers each declare a semver range
/// for the same package. Pick the highest of its published `versions` that
/// satisfies every requirer's range (intersection, not backtracking: when
/// none does, R-RES-005 names each requirer; a malformed range is R-RES-003).
fn unify_diamond(
    name: &str,
    versions: &[String],
    requirers: &[(String, String)],
) -> Result<String, Diagnostic> {
    let mut reqs = Vec::with_capacity(requirers.len());
    for (requirer, range) in requirers {
        let req = VersionReq::parse(range).map_err(|e| {
            Diagnostic::new(
                codes::R_RES_003,
                format!(
                    "'{}' declares an invalid version range '{}' for peer '{}': {}",
                    requirer, range, name, e
                ),
            )
            .with_suggestion("use semver syntax: ^1.0, ~2.3, >=1.0.0 <2.0.0".to_string())
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
        Diagnostic::new(
            codes::R_RES_005,
            format!(
                "version diamond for '{}': no single version satisfies every requirer ({}). Available: {}",
                name,
                wanted,
                versions.join(", ")
            ),
        )
        .with_suggestion(
            "no version unifies these ranges; upgrade the requirer with the narrowest range or pin a compatible peer version manually"
                .to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_installed::LockFileEntry;
    use specforge_test_macros::test as specforge_test;

    fn peer(name: &str, range: &str) -> PeerDependency {
        PeerDependency {
            name: name.to_string(),
            version: range.to_string(),
            optional: false,
        }
    }

    fn entry(name: &str, version: &str, peers: Vec<PeerDependency>) -> LockFileEntry {
        LockFileEntry {
            name: specforge_protocol_types::PackageName::parse(name).unwrap(),
            version: version.to_string(),
            source: specforge_installed::LockSource::Registry,
            wasm_hash: String::new(),
            key_id: None,
            peer_dependencies: peers,
        }
    }

    /// `@acme/base` locked at 1.0.0 and required by `@acme/other` at
    /// `other_range`.
    fn lock(other_range: &str) -> LockFile {
        LockFile {
            lockfile_version: 1,
            entries: vec![
                entry("@acme/base", "1.0.0", Vec::new()),
                entry(
                    "@acme/other",
                    "1.0.0",
                    vec![peer("@acme/base", other_range)],
                ),
            ],
        }
    }

    fn published(name: &str) -> Result<Vec<String>, OpError> {
        assert_eq!(name, "@acme/base");
        Ok(vec!["1.0.0".into(), "1.4.0".into(), "2.0.0".into()])
    }

    fn offline(_: &str) -> Result<Vec<String>, OpError> {
        panic!("a satisfied peer needs no registry")
    }

    #[specforge_test(
        behavior = "add_extension_to_existing_project",
        verify = "a peer locked outside the new extension's range fails R-RES-006 naming a version that satisfies every requirer"
    )]
    fn a_unifiable_diamond_is_refused_with_the_version_that_unifies() {
        let err = check_diamonds(
            &lock(">=1.0.0"),
            "@acme/app",
            &[peer("@acme/base", "^2.0")],
            &published,
        )
        .unwrap_err();
        assert_eq!(err.code, "R-RES-006");
        assert!(err.message.contains("@acme/base 2.0.0"), "{err:?}");
        assert!(err.suggestion.unwrap().contains("2.0.0"));
    }

    #[specforge_test(
        behavior = "add_extension_to_existing_project",
        verify = "a peer no single version satisfies for every requirer fails R-RES-005"
    )]
    fn an_impossible_diamond_is_refused_naming_each_requirer() {
        let err = check_diamonds(
            &lock("^1.0"),
            "@acme/app",
            &[peer("@acme/base", "^2.0")],
            &published,
        )
        .unwrap_err();
        assert_eq!(err.code, "R-RES-005");
        assert!(
            err.message.contains("@acme/other wants @acme/base ^1.0")
                && err.message.contains("@acme/app wants @acme/base ^2.0"),
            "{err:?}"
        );
    }

    #[test]
    fn a_locked_peer_inside_the_range_passes_without_the_registry() {
        let ok = check_diamonds(
            &lock("^1.0"),
            "@acme/app",
            &[peer("@acme/base", "^1.0"), peer("@acme/unlocked", "^3")],
            &offline,
        );
        assert_eq!(ok, Ok(()));
    }

    #[test]
    fn a_registry_failure_is_the_error() {
        let err = check_diamonds(
            &lock("^1.0"),
            "@acme/app",
            &[peer("@acme/base", "^2.0")],
            &|_| Err(OpError::diagnostic(codes::E063, "no registry")),
        )
        .unwrap_err();
        assert_eq!(err.code, "E063");
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
