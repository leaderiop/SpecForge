//! A milestone's `release` (ADR 0390 D6): the release evidence `gsd/E535` reads. On each milestone neither shipped nor
//! abandoned whose name is a version, `packages` holds the `name` and `version` of every `@code/core` `package` fact
//! whose version is not the milestone's ([`crate::release::matches`]) and whose name is not in
//! `specforge.release_exempt`, sorted, at most [`LINES`], and `more` counts the rest (absent when none): two
//! members, so no package name, not even `more` or one with a space, reads as the count. Name and version are as the
//! facts hold them (the guard escapes what it prints). Written only when some package fact carries a version: absent
//! means never observed, empty means clean. A package with no version is not judged. It reflects the facts of the last
//! collect. On a shipped milestone, `tag` and `commit` copy the `tag` `gsd-git` recorded, unless `git.create_tag` is
//! false (GSD then tags no release).

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{name_of, of_kind, text};
use crate::release::{matches, parse};

/// `@code/core`'s kind, named once as `thread.rs` names `test`.
pub const PACKAGE: &str = "package";
const LINES: usize = 50;

pub fn values(input: &PassInput) -> Vec<PassValue> {
    let settings = Settings::current();
    let exempt: Vec<&str> = settings
        .get("specforge.release_exempt")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut values = release(input, &exempt);
    if settings.bool("git.create_tag") != Some(false) {
        values.extend(tagged(input));
    }
    values
}

/// A shipped milestone's `release` copies the tag `gsd-git` recorded (`milestone.tag`): its `name` as `tag`, and its
/// `commit`, absent when the tag resolves to none. Absent when no tag was recorded.
fn tagged(input: &PassInput) -> Vec<PassValue> {
    of_kind(input, "milestone")
        .filter(|m| !m.fields.contains_key("abandoned") && m.fields.contains_key("completion"))
        .filter_map(|m| {
            let tag = m.values.get("tag")?;
            let name = tag.get("name")?.as_str()?;
            let mut release = json!({ "tag": name });
            if let Some(commit) = tag.get("commit").and_then(Value::as_str) {
                release["commit"] = json!(commit);
            }
            Some(value(&m.id, "release", release))
        })
        .collect()
}

fn release(input: &PassInput, exempt: &[&str]) -> Vec<PassValue> {
    let packages: Vec<(&str, &str)> = of_kind(input, PACKAGE)
        .filter_map(|p| Some((text(p, "name").unwrap_or(&p.id), text(p, "version")?)))
        .collect();
    if packages.is_empty() {
        return Vec::new();
    }
    of_kind(input, "milestone")
        .filter(|m| !m.fields.contains_key("abandoned") && !m.fields.contains_key("completion"))
        .filter_map(|m| {
            let version = parse(name_of(&m.id))?;
            let mut off: Vec<&(&str, &str)> = packages
                .iter()
                .filter(|(name, v)| !matches(&version, v) && !exempt.contains(name))
                .collect();
            off.sort();
            let listed: Vec<Value> = off
                .iter()
                .take(LINES)
                .map(|(name, v)| json!({ "name": name, "version": v }))
                .collect();
            let mut release = json!({ "packages": listed });
            if off.len() > LINES {
                release["more"] = json!(off.len() - LINES);
            }
            Some(value(&m.id, "release", release))
        })
        .collect()
}

#[cfg(test)]
mod tests;
