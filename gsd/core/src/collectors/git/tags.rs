//! A shipped milestone's `tag` (ADR 0390 D6): GSD tags a release `v<version>` after its close, and `gsd-git` records
//! whether that tag resolves, for `derive` to copy into `milestone.release` and `gsd/E535` to judge. A pass never asks
//! the git port (ADR 0335 D4): this collector does, once per shipped milestone whose name is a version, with the port's
//! `commit` verb on `refs/tags/v<version>` (an annotated tag resolves to its commit), the version as the name wrote it
//! (`v2026.01`, the tag GSD creates). The revision is built from the parsed version alone, digits and dots, so no
//! other text of a milestone's name reaches the port.
//!
//! No observation writes nothing: a shallow clone holds no tags, and with `git.create_tag` false GSD tags no release.

use serde_json::{Value, json};
use specforge_extension_sdk::host::{GitRepo, git};
use specforge_extension_sdk::prelude::*;

use super::said;
use crate::passes::plans::name_of;
use crate::release::parse;

/// Sets each shipped milestone's `tag` where it changed; the count set.
pub(super) fn plan(
    nodes: &[GraphNode],
    repo: &GitRepo,
    builder: &mut PlanBuilder,
) -> Result<usize, String> {
    let create_tag = Settings::current().bool("git.create_tag") != Some(false);
    let sets = tags(nodes, repo.shallow, create_tag)?;
    for (milestone, tag) in &sets {
        builder.set(milestone, "tag", tag.clone());
    }
    Ok(sets.len())
}

/// The `tag` of each shipped milestone whose name is a version, `{name, commit}` with `commit` absent when the tag
/// resolves to none, where it differs from the one the milestone holds.
pub(super) fn tags(
    nodes: &[GraphNode],
    shallow: bool,
    create_tag: bool,
) -> Result<Vec<(String, Value)>, String> {
    if shallow || !create_tag {
        return Ok(Vec::new());
    }
    let mut sets = Vec::new();
    for node in nodes
        .iter()
        .filter(|n| n.kind == "milestone" && n.fields.contains_key("completion"))
    {
        let Some(version) = parse(name_of(&node.id)) else {
            continue;
        };
        let name = format!("v{version}");
        let commit = git::commit(&format!("refs/tags/v{version}"))
            .map_err(|e| said(&format!("read the tag {name}"), &e))?;
        let tag = match commit {
            Some(commit) => json!({ "name": name, "commit": commit }),
            None => json!({ "name": name }),
        };
        if node.fields.get("tag") != Some(&tag) {
            sets.push((node.id.clone(), tag));
        }
    }
    Ok(sets)
}
