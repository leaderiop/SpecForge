//! What `specforge update` moves a locked package to (ADR 0036 D3, as amended by round 7).
//!
//! [`crate::VersionRequirement::pick`] chooses the version a requirement asks for; [`upgrade_target`] is the
//! one rule that chooses the version an update moves a locked package to. It is `pick` over the range
//! the update admits, so semver's pre-release rule reads it as it reads any range.

use semver::{Comparator, Op, Version, VersionReq};

use crate::highest_matching;

/// How far an update may move a locked version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Within the locked version's caret range (`^locked`): `specforge update`.
    Compatible,
    /// To any newer version, a new major one included: `specforge update --major`.
    Major,
}

/// The version an update of a package locked at `locked` moves to among `published`; `None` when
/// it is up to date.
///
/// From a SemVer version: the highest published version newer than `locked` (by precedence, so a
/// version differing only in build metadata is not newer) that `reach` admits, `^locked` for
/// [`Reach::Compatible`] and any for [`Reach::Major`]. A pre-release is admitted only on `locked`'s
/// own `MAJOR.MINOR.PATCH` when `locked` is a pre-release of it (semver's rule, as `pick` reads any
/// range): `2.0.0-beta.1` moves to `2.0.0-beta.2`, `2.0.0` and on, never to `3.0.0-alpha.1`, and a
/// release never moves to a pre-release. Never to `locked` or below: a locked version no longer
/// published, with nothing newer, is up to date.
pub fn upgrade_target<'a>(
    locked: &Version,
    published: &'a [Version],
    reach: Reach,
) -> Option<&'a Version> {
    highest_matching(published, &admitted(locked, reach))
}

/// What `reach` admits from `current`: `>current`, and `^current` as well unless `Major`. Each is its
/// own requirement, so semver's pre-release rule reads each as it reads any range.
fn admitted(current: &Version, reach: Reach) -> Vec<VersionReq> {
    let from = |op| VersionReq {
        comparators: vec![Comparator {
            op,
            major: current.major,
            minor: Some(current.minor),
            patch: Some(current.patch),
            pre: current.pre.clone(),
        }],
    };
    let mut admitted = vec![from(Op::Greater)];
    if reach == Reach::Compatible {
        admitted.push(from(Op::Caret));
    }
    admitted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(
        behavior = "upgrade_wasm_extension",
        verify = "one rule picks the version an update moves to, never the locked version or below"
    )]
    fn the_upgrade_target_is_the_newest_version_the_reach_admits_never_lower() {
        use Reach::{Compatible, Major};
        #[rustfmt::skip]
        let table: &[(&str, &[&str], Reach, Option<&str>)] = &[
            ("2.0.0-beta.1", &["1.4.0", "2.0.0-beta.1"], Major, None),
            ("2.0.0-beta.1", &["1.4.0", "2.0.0-beta.1"], Compatible, None),
            ("2.0.0-beta.1", &["2.0.0-beta.1", "2.0.0-beta.2"], Compatible, Some("2.0.0-beta.2")),
            ("2.0.0-beta.1", &["2.0.0-beta.1", "2.0.0-beta.2"], Major, Some("2.0.0-beta.2")),
            ("2.0.0-beta.1", &["2.0.0-beta.2", "2.0.0"], Compatible, Some("2.0.0")),
            ("2.0.0-beta.1", &["2.0.0", "2.1.0", "3.0.0"], Compatible, Some("2.1.0")),
            ("2.0.0-beta.1", &["2.0.0", "2.1.0", "3.0.0"], Major, Some("3.0.0")),
            ("2.0.0-beta.1", &["2.0.0-beta.2", "2.1.0-alpha.1", "3.0.0-alpha.1"], Major, Some("2.0.0-beta.2")),
            ("2.0.0-beta.9", &["2.0.0-beta.10"], Compatible, Some("2.0.0-beta.10")),
            ("1.4.0", &["1.4.0", "2.0.0-beta.1"], Major, None),
            ("1.4.0", &["1.4.0", "1.5.0-rc.1"], Compatible, None),
            ("1.4.0", &["1.4.0", "1.5.0", "2.0.0"], Compatible, Some("1.5.0")),
            ("1.4.0", &["1.4.0", "1.5.0", "2.0.0"], Major, Some("2.0.0")),
            ("1.2.0", &["1.0.0"], Compatible, None),
            ("1.2.0", &["1.0.0"], Major, None),
            ("1.2.0", &["1.0.0", "2.0.0"], Compatible, None),
            ("1.2.0", &["1.0.0", "2.0.0"], Major, Some("2.0.0")),
            ("0.0.9", &["0.0.9", "0.1.0"], Compatible, None),
            ("0.0.9", &["0.0.9", "0.1.0"], Major, Some("0.1.0")),
            ("0.2.3", &["0.2.9", "0.3.0"], Compatible, Some("0.2.9")),
            ("1.2.3", &["1.2.3", "1.9.0", "2.0.0"], Compatible, Some("1.9.0")),
            ("1.0.0+build.1", &["1.0.0+build.2"], Major, None),
            ("1.0.0+build.1", &["1.0.0+build.2", "1.0.1"], Compatible, Some("1.0.1")),
            ("0.1.0-alpha.1", &["0.1.0"], Compatible, Some("0.1.0")),
            ("1.0.0", &[], Major, None),
        ];
        assert_eq!(table.len(), 25);
        for (row, (locked, published, reach, want)) in table.iter().enumerate() {
            let published: Vec<Version> = published
                .iter()
                .map(|v| Version::parse(v).unwrap())
                .collect();
            let target = upgrade_target(&Version::parse(locked).unwrap(), &published, *reach)
                .map(Version::to_string);
            assert_eq!(
                target.as_deref(),
                *want,
                "row {}: locked {locked}, published {published:?}, {reach:?}",
                row + 1
            );
        }
    }
}
