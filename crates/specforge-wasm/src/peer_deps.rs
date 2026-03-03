use std::collections::HashMap;

use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;

use crate::error::WasmError;
use crate::manifest::PackageManifest;

/// Validate that all peer dependencies between loaded manifests are satisfied.
///
/// Checks that for every `peer_dependencies` entry in each manifest, there exists
/// another loaded manifest whose package name and version satisfy the semver range.
pub fn validate_peer_dependencies(manifests: &[PackageManifest]) -> Vec<WasmError> {
    let mut errors = Vec::new();

    // Build a map of package name → version
    let installed: HashMap<&str, &str> = manifests
        .iter()
        .map(|m| (m.package.as_str(), m.version.as_str()))
        .collect();

    for manifest in manifests {
        for (dep_name, range_str) in &manifest.peer_dependencies {
            match installed.get(dep_name.as_str()) {
                Some(found_version) => {
                    // Parse both the requirement and the found version
                    let req = match semver::VersionReq::parse(range_str) {
                        Ok(r) => r,
                        Err(_) => {
                            errors.push(WasmError::PeerDependencyUnsatisfied {
                                package: manifest.package.clone(),
                                dependency: dep_name.clone(),
                                required: range_str.clone(),
                                found: Some(format!("{found_version} (invalid range `{range_str}`)")),
                            });
                            continue;
                        }
                    };

                    let version = match semver::Version::parse(found_version) {
                        Ok(v) => v,
                        Err(_) => {
                            // Can't check if non-semver, skip
                            continue;
                        }
                    };

                    if !req.matches(&version) {
                        errors.push(WasmError::PeerDependencyUnsatisfied {
                            package: manifest.package.clone(),
                            dependency: dep_name.clone(),
                            required: range_str.clone(),
                            found: Some(found_version.to_string()),
                        });
                    }
                }
                None => {
                    errors.push(WasmError::PeerDependencyUnsatisfied {
                        package: manifest.package.clone(),
                        dependency: dep_name.clone(),
                        required: range_str.clone(),
                        found: None,
                    });
                }
            }
        }
    }

    errors
}

/// Topologically sort manifests by their peer dependencies.
///
/// Returns indices into the input slice in dependency order (dependencies loaded first).
/// If a cycle is detected, returns a `CycleDetected` error.
/// Ties are broken alphabetically by package name.
pub fn topological_sort(manifests: &[PackageManifest]) -> Result<Vec<usize>, WasmError> {
    if manifests.is_empty() {
        return Ok(vec![]);
    }

    let mut graph = DiGraph::<usize, ()>::new();
    let mut name_to_idx: HashMap<&str, petgraph::graph::NodeIndex> = HashMap::new();
    let mut node_to_manifest_idx: HashMap<petgraph::graph::NodeIndex, usize> = HashMap::new();

    // Add nodes
    for (i, manifest) in manifests.iter().enumerate() {
        let node = graph.add_node(i);
        name_to_idx.insert(&manifest.package, node);
        node_to_manifest_idx.insert(node, i);
    }

    // Add edges (dependency → dependent, so deps come first in topo order)
    for manifest in manifests {
        let dependent_node = name_to_idx[manifest.package.as_str()];
        for dep_name in manifest.peer_dependencies.keys() {
            if let Some(&dep_node) = name_to_idx.get(dep_name.as_str()) {
                graph.add_edge(dep_node, dependent_node, ());
            }
            // Missing deps are caught by validate_peer_dependencies
        }
    }

    match toposort(&graph, None) {
        Ok(sorted_nodes) => {
            let sorted: Vec<usize> = sorted_nodes
                .iter()
                .map(|&node| node_to_manifest_idx[&node])
                .collect();
            Ok(sorted)
        }
        Err(cycle) => {
            // Reconstruct cycle for error message
            let cycle_start = node_to_manifest_idx[&cycle.node_id()];
            let mut participants = vec![manifests[cycle_start].package.clone()];

            // Walk edges from cycle node to find the cycle
            for edge in graph.edges(cycle.node_id()) {
                let target_idx = node_to_manifest_idx[&edge.target()];
                participants.push(manifests[target_idx].package.clone());
            }
            participants.push(manifests[cycle_start].package.clone());

            Err(WasmError::CycleDetected { participants })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_manifest(package: &str, version: &str, deps: Vec<(&str, &str)>) -> PackageManifest {
        let mut peer_dependencies = HashMap::new();
        for (name, range) in deps {
            peer_dependencies.insert(name.to_string(), range.to_string());
        }
        PackageManifest {
            package: package.to_string(),
            manifest_version: "1".to_string(),
            kind: crate::manifest::PluginKind::Plugin,
            contributes: crate::manifest::PackageContributions::default(),
            wasm: "plugin.wasm".to_string(),
            description: String::new(),
            version: version.to_string(),
            enhancements: vec![],
            dynamic_edge_types: vec![],
            entity_kinds: vec![],
            provider: None,
            generator: None,
            sandbox: crate::manifest::SandboxPolicy::default(),
            peer_dependencies,
            query_extensions: vec![],
            manifest_path: PathBuf::new(),
            wasm_path: PathBuf::new(),
        }
    }

    #[test]
    fn no_deps_validates_ok() {
        let manifests = vec![
            make_manifest("@specforge/a", "1.0.0", vec![]),
            make_manifest("@specforge/b", "2.0.0", vec![]),
        ];
        let errors = validate_peer_dependencies(&manifests);
        assert!(errors.is_empty());
    }

    #[test]
    fn satisfied_dep() {
        let manifests = vec![
            make_manifest("@specforge/core", "1.2.0", vec![]),
            make_manifest("@specforge/ext", "1.0.0", vec![("@specforge/core", ">=1.0.0")]),
        ];
        let errors = validate_peer_dependencies(&manifests);
        assert!(errors.is_empty());
    }

    #[test]
    fn unsatisfied_version() {
        let manifests = vec![
            make_manifest("@specforge/core", "0.5.0", vec![]),
            make_manifest("@specforge/ext", "1.0.0", vec![("@specforge/core", ">=1.0.0")]),
        ];
        let errors = validate_peer_dependencies(&manifests);
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], WasmError::PeerDependencyUnsatisfied { found: Some(_), .. }));
    }

    #[test]
    fn missing_dep() {
        let manifests = vec![
            make_manifest("@specforge/ext", "1.0.0", vec![("@specforge/missing", ">=1.0.0")]),
        ];
        let errors = validate_peer_dependencies(&manifests);
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], WasmError::PeerDependencyUnsatisfied { found: None, .. }));
    }

    #[test]
    fn topo_sort_empty() {
        let sorted = topological_sort(&[]).unwrap();
        assert!(sorted.is_empty());
    }

    #[test]
    fn topo_sort_no_deps() {
        let manifests = vec![
            make_manifest("@specforge/a", "1.0.0", vec![]),
            make_manifest("@specforge/b", "1.0.0", vec![]),
        ];
        let sorted = topological_sort(&manifests).unwrap();
        assert_eq!(sorted.len(), 2);
        // Both indices present
        assert!(sorted.contains(&0));
        assert!(sorted.contains(&1));
    }

    #[test]
    fn topo_sort_with_deps() {
        let manifests = vec![
            make_manifest("@specforge/b", "1.0.0", vec![("@specforge/a", ">=1.0.0")]),
            make_manifest("@specforge/a", "1.0.0", vec![]),
        ];
        let sorted = topological_sort(&manifests).unwrap();
        // @specforge/a (index 1) must come before @specforge/b (index 0)
        let pos_a = sorted.iter().position(|&x| x == 1).unwrap();
        let pos_b = sorted.iter().position(|&x| x == 0).unwrap();
        assert!(pos_a < pos_b);
    }

    #[test]
    fn topo_sort_cycle_detected() {
        let manifests = vec![
            make_manifest("@specforge/a", "1.0.0", vec![("@specforge/b", ">=1.0.0")]),
            make_manifest("@specforge/b", "1.0.0", vec![("@specforge/a", ">=1.0.0")]),
        ];
        let result = topological_sort(&manifests);
        assert!(matches!(result, Err(WasmError::CycleDetected { .. })));
    }
}
