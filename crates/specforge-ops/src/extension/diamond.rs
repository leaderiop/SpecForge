//! The ADR-0001 version-diamond gate: `add` decides whether one locked
//! version can satisfy every requirer of a peer before it installs,
//! instead of leaving `doctor` to find the conflict afterwards.

use crate::OpError;
use specforge_registry::PeerDependency;
use specforge_wasm::{LockFile, collect_peer_requirers};

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
        let Some(locked) = lock.entries.iter().find(|e| e.name == peer.name) else {
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

        let requirers = collect_peer_requirers(lock, &peer.name, Some((package, &peer.version)));
        let published = versions(&peer.name)?;
        return match specforge_registry::resolver::unify_diamond(&peer.name, &published, &requirers)
        {
            Ok(unified) => Err(OpError::new(
                "R-RES-006",
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

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::LockFileEntry;

    fn peer(name: &str, range: &str) -> PeerDependency {
        PeerDependency {
            name: name.to_string(),
            version: range.to_string(),
            optional: false,
        }
    }

    fn entry(name: &str, version: &str, peers: Vec<PeerDependency>) -> LockFileEntry {
        LockFileEntry {
            name: name.to_string(),
            version: version.to_string(),
            source: "registry".to_string(),
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
            &|_| Err(OpError::new("E063", "no registry")),
        )
        .unwrap_err();
        assert_eq!(err.code, "E063");
    }
}
