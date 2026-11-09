//! The workspace's named packages as `package` facts (ADR 0347 D5), the targets of their items' `package`.

use std::collections::BTreeSet;

use specforge_code_facts::{Package, PackageId};
use specforge_extension_sdk::prelude::Fact;
use specforge_javascript_workspace::Workspace;

/// One `package` fact per named package of `workspace`, by id: its version, its engines, its directory and the
/// project's own packages it depends on, a name its dependency tables list taken as the package of that name the
/// plain `pkg:npm/<name>` names.
pub(crate) fn facts(workspace: &Workspace) -> Vec<Fact> {
    let own: BTreeSet<&str> = workspace
        .packages()
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    let mut facts: Vec<Fact> = workspace
        .packages()
        .iter()
        .map(|package| {
            Package {
                id: package.id.clone(),
                version: package.version.clone(),
                edition: None,
                rust_version: None,
                engines: package.engines.clone(),
                features: Vec::new(),
                targets: Vec::new(),
                path: package.dir.clone(),
                depends_on: package
                    .depends_on
                    .iter()
                    .filter(|name| own.contains(name.as_str()) && **name != package.name)
                    .map(|name| PackageId::npm(name))
                    .collect(),
                at: None,
            }
            .fact()
        })
        .collect();
    facts.sort_by(|a, b| a.id.cmp(&b.id));
    facts
}
