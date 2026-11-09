//! The workspace's named packages as `package` facts (ADR 0347 D5), the targets of their items' `package`.

use std::collections::BTreeMap;

use specforge_code_facts::{Package, PackageId};
use specforge_extension_sdk::prelude::Fact;
use specforge_javascript_workspace::Workspace;

/// One `package` fact per package of `workspace`, by id: its version, its engines, its directory and the project's
/// own packages it depends on, a name its dependency tables list taken as the shallowest package of that name here.
pub(crate) fn facts(workspace: &Workspace) -> Vec<Fact> {
    let depth = |dir: &str| dir.split('/').filter(|s| !s.is_empty()).count();
    let mut own: BTreeMap<&str, (usize, &str, &PackageId)> = BTreeMap::new();
    for package in workspace.packages() {
        let this = (depth(&package.dir), package.dir.as_str(), &package.id);
        let kept = own.entry(package.name.as_str()).or_insert(this);
        if (this.0, this.1) < (kept.0, kept.1) {
            *kept = this;
        }
    }
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
                    .filter(|name| **name != package.name)
                    .filter_map(|name| Some(own.get(name.as_str())?.2.clone()))
                    .collect(),
                at: None,
            }
            .fact()
        })
        .collect();
    facts.sort_by(|a, b| a.id.cmp(&b.id));
    facts
}
