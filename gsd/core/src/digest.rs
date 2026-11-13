//! The scope helpers the passes share, and the digest of the files a verdict covered: `fnv1a64:` and 16 lowercase hex
//! digits of FNV-1a 64 over the sorted `(path, digest)` pairs.

use specforge_extension_sdk::prelude::*;

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0100_0000_01b3;

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

/// The scope an entity is in: its id without the last `/`-segment.
pub fn scope_of(id: &str) -> &str {
    id.rsplit_once('/').map_or("", |(scope, _)| scope)
}

/// The quick plan a context, decision or research names in `plan`: its resolved edge.
pub fn quick_plan<'a>(input: &'a PassInput, id: &str) -> Option<&'a str> {
    input
        .edges
        .iter()
        .find(|e| e.source == id && e.label == "plan")
        .map(|e| e.target.as_str())
}

/// What owns a context, decision or research: its quick plan, else its scope (its phase, or `""` for the project).
pub fn owner<'a>(input: &'a PassInput, entity: &'a PassEntity) -> &'a str {
    quick_plan(input, &entity.id).unwrap_or_else(|| scope_of(&entity.id))
}

fn hex(bytes: &[u8]) -> String {
    format!("fnv1a64:{:016x}", fnv1a64(bytes))
}

/// One digest over `(path, digest)` pairs: the files a verdict covered, as the host digested them.
pub fn files<'a>(covered: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut pairs: Vec<_> = covered.into_iter().collect();
    pairs.sort_unstable();
    pairs.dedup();
    let mut bytes = Vec::new();
    for (path, digest) in pairs {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0x1F);
        bytes.extend_from_slice(digest.as_bytes());
        bytes.push(0x1E);
    }
    hex(&bytes)
}

#[cfg(test)]
mod tests;
