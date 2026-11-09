//! The workspace's named packages as `package` facts (ADR 0347 D5), the targets of their items' `package`.

use std::collections::{BTreeMap, BTreeSet};

use specforge_code_facts::{Package, PackageId};
use specforge_extension_sdk::prelude::Fact;
use specforge_javascript_workspace::Workspace;

/// One `package` fact per named package of `workspace`, `pkg:npm/<name>`: its version, its engines, its directory and
/// the project's own packages it depends on. Two `package.json` of one name are one package, the shallowest's.
pub(crate) fn facts(workspace: &Workspace) -> Vec<Fact> {
    let mut ordered: Vec<_> = workspace.packages().iter().collect();
    ordered.sort_by_key(|p| (p.dir.split('/').filter(|s| !s.is_empty()).count(), &p.dir));
    let mut by_name = BTreeMap::new();
    for package in ordered {
        by_name.entry(package.name.as_str()).or_insert(package);
    }
    let own: BTreeSet<&str> = by_name.keys().copied().collect();
    by_name
        .values()
        .map(|package| {
            Package {
                id: PackageId::npm(&package.name),
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
        .collect()
}
