//! A milestone's version, read from its name (`v1.7`, `1.7`, `v1.7.2`), Cargo's compatibility rule between two
//! (ADR 0390 D5), and whether a package's version is the milestone's (D6). Hand-written on purpose: no version library
//! accepts `v1.7`.

use std::cmp::Ordering;
use std::fmt;

/// One to three numeric parts; a part not written reads as 0, so `v1.0` and `v1.0.0` are one version, ordered and
/// compared by its parts alone. `written` counts the parts the name wrote: it decides how many a package must match
/// and how the version prints.
#[derive(Debug, Clone, Copy)]
pub struct Version {
    pub parts: [u64; 3],
    pub written: usize,
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.parts == other.parts
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parts.cmp(&other.parts)
    }
}

/// The parts as written, digits and dots only (`1.7` for `v1.7`): a tag name is `v` and this, never the raw name.
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.parts[..self.written]
            .iter()
            .map(u64::to_string)
            .collect();
        f.write_str(&parts.join("."))
    }
}

/// The version a name writes: one leading `v` stripped, then one to three dot-separated decimal parts; anything else
/// (`mvp`, `v1.7-rc1`, `v1.2.3.4`) is not a version.
pub fn parse(name: &str) -> Option<Version> {
    let digits = name.strip_prefix('v').unwrap_or(name);
    let mut parts = [0u64; 3];
    let mut written = 0;
    for (index, part) in digits.split('.').enumerate() {
        if index == 3 || part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        parts[index] = part.parse().ok()?;
        written = index + 1;
    }
    Some(Version { parts, written })
}

/// Cargo's rule: `newer` is API-compatible with `older` when they agree up to and including the first non-zero part
/// of `older` (`1.7` to `1.8` yes, `1.7` to `2.0` no; for `0.x` the minor counts as major).
pub fn compatible(older: &Version, newer: &Version) -> bool {
    let first = older.parts.iter().position(|p| *p != 0).unwrap_or(2);
    older.parts[..=first] == newer.parts[..=first]
}

/// Whether a package's version is the milestone's: their first max(2, the milestone's written parts) parts agree
/// (`v1.7` takes `1.7.0` and `1.7.3`, `v1.7.2` only `1.7.2`). A `+build` suffix is ignored; a pre-release
/// (`1.7.0-rc.1`) and anything that is no version is not equal.
pub fn matches(milestone: &Version, package_version: &str) -> bool {
    let bare = package_version
        .split_once('+')
        .map_or(package_version, |(v, _)| v);
    let Some(package) = parse(bare) else {
        return false;
    };
    let compared = milestone.written.max(2);
    milestone.parts[..compared] == package.parts[..compared]
}

#[cfg(test)]
mod tests;
