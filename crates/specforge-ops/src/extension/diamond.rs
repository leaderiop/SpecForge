//! The ADR-0001 version-diamond gate: `add` decides whether one locked
//! version can satisfy every requirer of a peer before it installs,
//! instead of leaving `doctor` to find the conflict afterwards. It judges a
//! peer by the one peer rule (ADR 0041).

use crate::OpError;
use crate::OpErrorKind;
use specforge_common::{Diagnostic, codes};
use specforge_installed::LockFile;
use specforge_protocol_types::PeerDependency;
use specforge_protocol_types::package::Version;
use specforge_protocol_types::peers::{PeerRequirement, Verdict, verdict};

/// What the gate may consult about a peer outside a range: the versions a
/// registry publishes of it (an install from a registry), or nothing (a
/// local install), when the refusal is E027.
pub type Published<'a> = Option<&'a dyn Fn(&str) -> Result<Vec<String>, OpError>>;

/// Check that installing `package` (declaring `peers`) leaves every locked
/// peer satisfied. A range that is not SemVer is E073. A locked peer outside
/// a declared range is refused: from a registry, R-RES-006 when one
/// published version would satisfy every requirer (the fix is to reinstall
/// the peer at it) and R-RES-005 when none would; without a registry
/// (`published` is `None`), E027. `published` lists a peer's published
/// versions; it is asked only for a peer whose locked version falls outside
/// a range. A peer that is not locked is `check`'s (E027).
pub fn check_diamonds(
    lock: &LockFile,
    package: &str,
    peers: &[PeerDependency],
    published: Published<'_>,
) -> Result<(), OpError> {
    for declared in peers {
        let locked = lock
            .entries
            .iter()
            .find(|e| e.name.as_str() == declared.name)
            .map(|e| e.version.as_str());
        let unsatisfied = match verdict(declared, locked) {
            Verdict::Satisfied | Verdict::Missing => continue,
            Verdict::Unreadable(why) => {
                return Err(OpError::from(specforge_common::peers::unreadable(
                    package, declared, &why,
                )));
            }
            unsatisfied @ (Verdict::OutOfRange { .. } | Verdict::NotSemver { .. }) => unsatisfied,
        };
        let Some(published) = published else {
            let diagnostic = specforge_common::peers::of(package, declared, &unsatisfied)
                .expect("an unsatisfied peer is a diagnostic");
            return Err(OpError::from(diagnostic));
        };

        let requirers = lock.requirers_of(&declared.name, Some((package, declared)));
        let versions = published(&declared.name)?;
        let locked = locked.expect("an installed peer is out of range or not SemVer");
        return match unify_diamond(&declared.name, &versions, &requirers) {
            Ok(unified) => Err(OpError::coded(
                OpErrorKind::Conflict,
                codes::R_RES_006,
                format!(
                    "version diamond: '{package}' requires peer '{}' {} but {locked} is locked; {} {unified} would satisfy every requirer",
                    declared.name, declared.version, declared.name
                ),
            )
            .with_suggestion(format!(
                "no command pins peer versions yet; manually reinstall '{}' at {unified} (or a version satisfying every requirer), then retry add",
                declared.name
            ))),
            Err(diagnostic) => Err(OpError::from(diagnostic)),
        };
    }
    Ok(())
}

/// Each locked extension other than those `changing` that requires `package` and that `staged`
/// (the lock as the change leaves it) leaves unsatisfied: `(dependent, why)`, in lock order.
pub(crate) fn broken_requirers(
    staged: &LockFile,
    package: &str,
    changing: &[&str],
    published: Published<'_>,
) -> Vec<(String, OpError)> {
    let mut broken = Vec::new();
    for entry in &staged.entries {
        if changing.contains(&entry.name.as_str()) {
            continue;
        }
        for peer in entry.peer_dependencies.iter().filter(|p| p.name == package) {
            if let Err(error) = check_diamonds(
                staged,
                entry.name.as_str(),
                std::slice::from_ref(peer),
                published,
            ) {
                broken.push((entry.name.to_string(), error));
            }
        }
    }
    broken
}

/// Unify a version diamond: several requirers each declare a SemVer range
/// for the same package. Pick the highest of its published `versions` that
/// satisfies every requirer's range (intersection, not backtracking: when
/// none does, R-RES-005 names each requirer; a range that is not SemVer is
/// E073).
fn unify_diamond(
    name: &str,
    versions: &[String],
    requirers: &[(String, PeerDependency)],
) -> Result<String, Diagnostic> {
    let mut reqs = Vec::with_capacity(requirers.len());
    for (requirer, declared) in requirers {
        let requirement = PeerRequirement::read(declared)
            .map_err(|why| specforge_common::peers::unreadable(requirer, declared, &why))?;
        reqs.push((requirer.as_str(), declared.version.as_str(), requirement));
    }

    let mut candidates: Vec<Version> = versions
        .iter()
        .filter_map(|v| Version::parse(v).ok())
        .collect();
    candidates.sort();

    let unified = candidates.into_iter().rev().find(|v| {
        reqs.iter()
            .all(|(_, _, requirement)| requirement.accepts(&v.to_string()) == Some(true))
    });

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
            Some(&published),
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
            Some(&published),
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
            Some(&offline),
        );
        assert_eq!(ok, Ok(()));
    }

    #[test]
    fn a_registry_failure_is_the_error() {
        let err = check_diamonds(
            &lock("^1.0"),
            "@acme/app",
            &[peer("@acme/base", "^2.0")],
            Some(&|_| Err(OpError::diagnostic(codes::E063, "no registry"))),
        )
        .unwrap_err();
        assert_eq!(err.code, "E063");
    }

    #[specforge_test(
        behavior = "add_extension_to_existing_project",
        verify = "an extension whose peer range is not SemVer is refused with E073 before anything is installed"
    )]
    fn an_unreadable_candidate_range_is_refused_e073() {
        let err = check_diamonds(
            &lock(">=1.0.0"),
            "@acme/app",
            &[peer("@acme/base", "one-ish")],
            Some(&offline),
        )
        .unwrap_err();
        assert_eq!(err.code, "E073", "{err:?}");
        assert!(
            err.message.contains("'@acme/app'") && err.message.contains("'one-ish'"),
            "{err:?}"
        );
    }

    #[test]
    fn an_unreadable_requirer_range_is_e073_naming_it() {
        let err = check_diamonds(
            &lock("one-ish"),
            "@acme/app",
            &[peer("@acme/base", "^2.0")],
            Some(&published),
        )
        .unwrap_err();
        assert_eq!(err.code, "E073", "{err:?}");
        assert!(err.message.contains("'@acme/other'"), "{err:?}");
    }

    #[specforge_test(
        behavior = "add_extension_to_existing_project",
        verify = "a local install whose peer is installed outside its range is refused with E027"
    )]
    fn a_local_candidate_out_of_range_is_e027() {
        let err = check_diamonds(
            &lock("^1.0"),
            "@acme/app",
            &[peer("@acme/base", "^2.0")],
            None,
        )
        .unwrap_err();
        assert_eq!(err.code, "E027", "{err:?}");
        assert_eq!(
            err.message,
            "extension '@acme/app' requires peer dependency '@acme/base' ^2.0 but version 1.0.0 is installed"
        );
    }

    #[test]
    fn a_locked_peer_at_a_version_that_is_not_semver_is_unified_like_one_out_of_range() {
        let mut locked = lock("^1.0");
        locked.entries[0].version = "local".to_string();
        let err = check_diamonds(
            &locked,
            "@acme/app",
            &[peer("@acme/base", ">=1.0.0")],
            Some(&|_: &str| Ok(vec!["1.0.0".to_string(), "2.0.0".to_string()])),
        )
        .unwrap_err();
        assert_eq!(err.code, "R-RES-006", "{err:?}");
        assert!(err.message.contains("@acme/base 1.0.0"), "{err:?}");
    }

    fn versions(vs: &[&str]) -> Vec<String> {
        vs.iter().map(|v| v.to_string()).collect()
    }

    fn req(requirer: &str, range: &str) -> (String, PeerDependency) {
        (requirer.to_string(), peer("@shared/lib", range))
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
        assert_eq!(err.code, "E073");
        assert!(err.message.contains("@a/ext"));
    }

    #[test]
    fn unify_diamond_no_requirers_picks_highest_available() {
        let vs = versions(&["1.0.0", "2.0.0"]);
        let result = unify_diamond("@shared/lib", &vs, &[]).unwrap();
        assert_eq!(result, "2.0.0");
    }
}
