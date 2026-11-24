//! A milestone's version, read from its name (`v1.7`, `1.7`, `v1.7.2`), and Cargo's compatibility rule between two
//! (ADR 0390 D5). Hand-written on purpose: no version library accepts `v1.7`.

/// One to three numeric parts; a part not written reads as 0, so `v1.0` and `v1.0.0` are one version, ordered by
/// its parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub parts: [u64; 3],
}

/// The version a name writes: one leading `v` stripped, then one to three dot-separated decimal parts; anything else
/// (`mvp`, `v1.7-rc1`, `v1.2.3.4`) is not a version.
pub fn parse(name: &str) -> Option<Version> {
    let digits = name.strip_prefix('v').unwrap_or(name);
    let mut parts = [0u64; 3];
    for (index, part) in digits.split('.').enumerate() {
        if index == 3 || part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        parts[index] = part.parse().ok()?;
    }
    Some(Version { parts })
}

/// Cargo's rule: `newer` is API-compatible with `older` when they agree up to and including the first non-zero part
/// of `older` (`1.7` to `1.8` yes, `1.7` to `2.0` no; for `0.x` the minor counts as major).
pub fn compatible(older: &Version, newer: &Version) -> bool {
    let first = older.parts.iter().position(|p| *p != 0).unwrap_or(2);
    older.parts[..=first] == newer.parts[..=first]
}

#[cfg(test)]
mod tests;
