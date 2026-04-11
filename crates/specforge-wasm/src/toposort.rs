use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;

/// Sort extensions in topological order based on peer dependencies.
/// Extensions with no dependencies come first.
/// Ties are broken by extension name for determinism.
pub fn topological_sort_extensions(manifests: &[ManifestV2]) -> Result<Vec<String>, Vec<Diagnostic>> {
    use std::collections::{BTreeSet, HashMap};

    // Build adjacency: name -> set of dependencies (peers that are also installed)
    let installed: std::collections::HashSet<&str> = manifests.iter().map(|m| m.name.as_str()).collect();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for m in manifests {
        in_degree.entry(m.name.as_str()).or_insert(0);
        for peer in &m.peer_dependencies {
            if installed.contains(peer.name.as_str()) {
                *in_degree.entry(m.name.as_str()).or_insert(0) += 1;
                dependents.entry(peer.name.as_str()).or_default().push(m.name.as_str());
            }
        }
    }

    // Kahn's algorithm with BTreeSet for deterministic tie-breaking
    let mut queue: BTreeSet<&str> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();

    let mut order = Vec::with_capacity(manifests.len());

    while let Some(name) = queue.iter().next().copied() {
        queue.remove(name);
        order.push(name.to_string());

        if let Some(deps) = dependents.get(name) {
            for &dep in deps {
                let deg = in_degree.get_mut(dep).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.insert(dep);
                }
            }
        }
    }

    if order.len() != manifests.len() {
        // Cycle detected — find the extensions involved
        let in_cycle: Vec<String> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg > 0)
            .map(|(&name, _)| name.to_string())
            .collect();
        return Err(vec![Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "cycle detected in peer dependencies: {}",
                in_cycle.join(", ")
            ),
            span: None,
            suggestion: Some("remove or break the circular dependency".to_string()),
        }]);
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_manifest;

    // B:topological_sort_extensions — verify unit "extensions sorted in dependency order"
    #[test]
    fn test_extensions_sorted_in_dependency_order() {
        let manifests = vec![
            make_manifest("@specforge/governance", &[("@specforge/software", ">=1.0.0")]),
            make_manifest("@specforge/software", &[]),
        ];

        let order = topological_sort_extensions(&manifests).unwrap();
        assert_eq!(order, vec!["@specforge/software", "@specforge/governance"]);
    }

    // B:topological_sort_extensions — verify unit "cycle in peer dependencies produces error"
    #[test]
    fn test_cycle_in_peer_dependencies_produces_error() {
        let manifests = vec![
            make_manifest("A", &[("B", ">=1.0.0")]),
            make_manifest("B", &[("A", ">=1.0.0")]),
        ];

        let err = topological_sort_extensions(&manifests).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].code, "E031");
        assert!(err[0].message.contains("cycle"));
    }

    // B:topological_sort_extensions — verify unit "deterministic ordering on ties"
    #[test]
    fn test_deterministic_ordering_on_ties() {
        // Three independent extensions — sorted alphabetically
        let manifests = vec![
            make_manifest("C-ext", &[]),
            make_manifest("A-ext", &[]),
            make_manifest("B-ext", &[]),
        ];

        let order = topological_sort_extensions(&manifests).unwrap();
        assert_eq!(order, vec!["A-ext", "B-ext", "C-ext"]);
    }

    // B:topological_sort_extensions — verify contract "requires/ensures consistency for topological extension sorting"
    #[test]
    fn test_topological_sort_contract() {
        // requires: manifests_loaded — we have parsed manifests
        let manifests = vec![
            make_manifest("@specforge/product", &[("@specforge/software", ">=1.0.0")]),
            make_manifest("@specforge/governance", &[("@specforge/software", ">=1.0.0")]),
            make_manifest("@specforge/software", &[]),
        ];

        // ensures: extensions_sorted_emitted — sorted order returned
        let order = topological_sort_extensions(&manifests).unwrap();
        assert_eq!(order[0], "@specforge/software");
        assert_eq!(order.len(), 3);

        // ensures: sort_deterministic — ties broken by name
        assert_eq!(order[1], "@specforge/governance");
        assert_eq!(order[2], "@specforge/product");

        // ensures: cycles_diagnosed
        let cyclic = vec![
            make_manifest("X", &[("Y", ">=1.0.0")]),
            make_manifest("Y", &[("Z", ">=1.0.0")]),
            make_manifest("Z", &[("X", ">=1.0.0")]),
        ];
        let err = topological_sort_extensions(&cyclic).unwrap_err();
        assert_eq!(err[0].code, "E031");
        assert_eq!(err[0].severity, Severity::Error);
    }
}
