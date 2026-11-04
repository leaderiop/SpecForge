//! Which versions a requirement admits, each ecosystem by its own rules: Cargo's through `semver`, npm's through
//! `nodejs-semver`, PyPI's PEP 440 specifiers through `pep440_rs`. A prerelease is admitted only as each says: Cargo
//! and npm when a comparator names a prerelease of the same version, PyPI when a specifier names any prerelease.

use std::cmp::Ordering;
use std::str::FromStr;

use crate::coord::{Ecosystem, Release};

/// The versions a package's value asks for.
#[derive(Debug, Clone)]
pub enum Requirement {
    /// None written: every release that is no prerelease.
    Any,
    Cargo(semver::VersionReq),
    Npm(nodejs_semver::Range),
    Pypi(pep440_rs::VersionSpecifiers),
}

/// A version of one ecosystem, ordered as it orders them.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Version {
    Cargo(semver::Version),
    Npm(nodejs_semver::Version),
    Pypi(pep440_rs::Version),
}

impl Version {
    fn parse(ecosystem: Ecosystem, text: &str) -> Option<Version> {
        match ecosystem {
            Ecosystem::Crates => semver::Version::parse(text).ok().map(Version::Cargo),
            Ecosystem::Npm => nodejs_semver::Version::parse(text).ok().map(Version::Npm),
            Ecosystem::Pypi => pep440_rs::Version::from_str(text).ok().map(Version::Pypi),
        }
    }

    fn is_prerelease(&self) -> bool {
        match self {
            Version::Cargo(v) => !v.pre.is_empty(),
            Version::Npm(v) => v.is_prerelease(),
            Version::Pypi(v) => v.any_prerelease(),
        }
    }

    fn order(&self, other: &Version) -> Ordering {
        match (self, other) {
            (Version::Cargo(a), Version::Cargo(b)) => a.cmp(b),
            (Version::Npm(a), Version::Npm(b)) => a.cmp(b),
            (Version::Pypi(a), Version::Pypi(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

impl Requirement {
    /// `text` as a requirement of `ecosystem`; why it is none otherwise.
    pub fn parse(ecosystem: Ecosystem, text: &str) -> Result<Requirement, String> {
        let text = text.trim();
        match ecosystem {
            Ecosystem::Crates => semver::VersionReq::parse(text)
                .map(Requirement::Cargo)
                .map_err(|e| e.to_string()),
            Ecosystem::Npm => nodejs_semver::Range::parse(text)
                .map(Requirement::Npm)
                .map_err(|e| e.to_string()),
            Ecosystem::Pypi => pep440_rs::VersionSpecifiers::from_str(text)
                .map(Requirement::Pypi)
                .map_err(|e| e.to_string()),
        }
    }

    fn admits(&self, version: &Version) -> bool {
        match (self, version) {
            (Requirement::Any, version) => !version.is_prerelease(),
            (Requirement::Cargo(req), Version::Cargo(v)) => req.matches(v),
            (Requirement::Npm(range), Version::Npm(v)) => range.satisfies(v),
            (Requirement::Pypi(specifiers), Version::Pypi(v)) => {
                specifiers.contains(v)
                    && (!v.any_prerelease() || specifiers.iter().any(|s| s.any_prerelease()))
            }
            _ => false,
        }
    }

    /// Whether `version`, of `ecosystem`, meets it; a version that does not parse meets none.
    pub fn matches(&self, ecosystem: Ecosystem, version: &str) -> bool {
        Version::parse(ecosystem, version).is_some_and(|v| self.admits(&v))
    }
}

/// The newest of `releases` that `keep` keeps, by `ecosystem`'s order; releases whose version does not parse are left.
fn newest_where(
    ecosystem: Ecosystem,
    releases: &[Release],
    keep: impl Fn(&Release, &Version) -> bool,
) -> Option<&Release> {
    releases
        .iter()
        .filter_map(|release| Some((release, Version::parse(ecosystem, &release.version)?)))
        .filter(|(release, version)| keep(release, version))
        .max_by(|(_, a), (_, b)| a.order(b))
        .map(|(release, _)| release)
}

/// The newest release `requirement` admits, yanked ones left out.
pub fn newest<'r>(
    ecosystem: Ecosystem,
    requirement: &Requirement,
    releases: &'r [Release],
) -> Option<&'r Release> {
    newest_where(ecosystem, releases, |release, version| {
        !release.yanked && requirement.admits(version)
    })
}

/// The newest release that is not yanked, whatever it asks: what a message names when none is admitted.
pub fn newest_unyanked(ecosystem: Ecosystem, releases: &[Release]) -> Option<&Release> {
    newest_where(ecosystem, releases, |release, _| !release.yanked)
}
