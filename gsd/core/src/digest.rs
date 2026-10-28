//! The digest of a phase's context: what `gsd/W406` compares. GSD judges drift by file times (CONTEXT.md newer than
//! RESEARCH.md); a pass sees no git history, so a research or spec records the digest of the context it was written
//! against and the drift pass compares it with the context's current one.
//!
//! The digest covers what GSD's `CONTEXT.md` holds: the context, the decisions with `origin context` and the deferred
//! ideas of its phase. It is `fnv1a64:` and 16 lowercase hex digits of FNV-1a 64 over the members sorted by id; per
//! member `id 0x1F kind 0x1E`, then each written field sorted by name, computed fields left out, as
//! `name 0x1F value 0x1E`, where value is the compact JSON of the structured value when the kernel sends one and the
//! field text otherwise; members are separated by `0x1D`. FNV and not SHA-256: drift needs no collision resistance
//! (the `H` field is guarded by the ledger), and the prefix keeps it apart from a kernel `sha256:` digest.
//! A respelt reference changes the digest; GSD's commit-time check flags any edit to CONTEXT.md too.

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

/// The members of the context `context_id`, sorted by id: the context, and in its scope the decisions with
/// `origin context` and the deferred ideas.
fn members<'a>(input: &'a PassInput, context_id: &str) -> Vec<&'a PassEntity> {
    let scope = scope_of(context_id);
    let mut found: Vec<&PassEntity> = input
        .entities
        .iter()
        .filter(|e| {
            e.id == context_id
                || (scope_of(&e.id) == scope
                    && match e.kind.as_str() {
                        "decision" => e.fields.get("origin").map(String::as_str) == Some("context"),
                        "deferred_idea" => true,
                        _ => false,
                    })
        })
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// The digest of the context `context_id`, `None` when the input holds no such entity.
pub fn context(input: &PassInput, context_id: &str) -> Option<String> {
    let computed = crate::kinds::computed();
    let members = members(input, context_id);
    members.iter().find(|e| e.id == context_id)?;
    let mut bytes = Vec::new();
    for (index, entity) in members.iter().enumerate() {
        if index > 0 {
            bytes.push(0x1D);
        }
        bytes.extend_from_slice(entity.id.as_bytes());
        bytes.push(0x1F);
        bytes.extend_from_slice(entity.kind.as_bytes());
        bytes.push(0x1E);
        // `fields` is a sorted map: names come in order.
        for (name, text) in &entity.fields {
            if computed.contains(&format!("{}.{name}", entity.kind)) {
                continue;
            }
            bytes.extend_from_slice(name.as_bytes());
            bytes.push(0x1F);
            match entity.values.get(name) {
                Some(value) => bytes.extend_from_slice(value.to_string().as_bytes()),
                None => bytes.extend_from_slice(text.as_bytes()),
            }
            bytes.push(0x1E);
        }
    }
    Some(format!("fnv1a64:{:016x}", fnv1a64(&bytes)))
}

#[cfg(test)]
mod tests;
