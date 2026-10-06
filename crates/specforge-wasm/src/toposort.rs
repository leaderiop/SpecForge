use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::ExtensionDeclaration;

/// Sort extensions in topological order based on peer dependencies.
/// Extensions with no dependencies come first.
/// Ties are broken by extension name for determinism.
///
/// A cycle among **required** peers is E027. An optional peer is only a
/// preference for load order, so two extensions may name each other as
/// optional peers: the required edges are sorted first, then the optional
/// ones are added in name order, each skipped when it would close a cycle.
pub fn topological_sort_extensions(
    declarations: &[ExtensionDeclaration],
) -> Result<Vec<String>, Vec<Diagnostic>> {
    use std::collections::BTreeSet;

    let installed: BTreeSet<&str> = declarations.iter().map(|d| d.name()).collect();
    // (peer, dependent): the peer loads first.
    let mut required: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut optional: BTreeSet<(&str, &str)> = BTreeSet::new();
    for d in declarations {
        for peer in d.peers() {
            if installed.contains(peer.name.as_str()) && peer.name != d.name() {
                let edge = (peer.name.as_str(), d.name());
                if peer.optional {
                    optional.insert(edge);
                } else {
                    required.insert(edge);
                }
            }
        }
    }

    if let Err(in_cycle) = kahn(&installed, &required) {
        return Err(vec![Diagnostic {
            code: "E027".to_string(),
            severity: Severity::Error,
            message: format!(
                "cycle detected in peer dependencies: {}",
                in_cycle.join(", ")
            ),
            span: None,
            suggestion: Some("remove or break the circular dependency".to_string()),
            data: None,
            origin: None,
        }]);
    }

    // Optional edges, in (dependent, peer) name order: kept unless the
    // dependent already loads before the peer.
    let mut edges = required;
    let mut optional: Vec<(&str, &str)> = optional.into_iter().collect();
    optional.sort_by_key(|&(peer, dependent)| (dependent, peer));
    for (peer, dependent) in optional {
        if !edges.contains(&(peer, dependent)) && !reaches(&edges, dependent, peer) {
            edges.insert((peer, dependent));
        }
    }
    Ok(kahn(&installed, &edges).expect("optional edges that close no cycle leave an order"))
}

/// Whether `to` loads after `from` through `edges`.
fn reaches(edges: &std::collections::BTreeSet<(&str, &str)>, from: &str, to: &str) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![from];
    while let Some(node) = stack.pop() {
        if node == to {
            return true;
        }
        if !seen.insert(node) {
            continue;
        }
        stack.extend(
            edges
                .iter()
                .filter(|(peer, _)| *peer == node)
                .map(|(_, dependent)| *dependent),
        );
    }
    false
}

/// Kahn's algorithm with ties broken by name. Err: the extensions left on
/// a cycle.
fn kahn(
    names: &std::collections::BTreeSet<&str>,
    edges: &std::collections::BTreeSet<(&str, &str)>,
) -> Result<Vec<String>, Vec<String>> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut in_degree: BTreeMap<&str, usize> = names.iter().map(|&n| (n, 0)).collect();
    for &(_, dependent) in edges {
        *in_degree.get_mut(dependent).expect("an installed name") += 1;
    }
    let mut queue: BTreeSet<&str> = in_degree
        .iter()
        .filter(|&(_, &degree)| degree == 0)
        .map(|(&name, _)| name)
        .collect();
    let mut order = Vec::with_capacity(names.len());
    while let Some(name) = queue.iter().next().copied() {
        queue.remove(name);
        order.push(name.to_string());
        for &(peer, dependent) in edges {
            if peer == name {
                let degree = in_degree.get_mut(dependent).expect("an installed name");
                *degree -= 1;
                if *degree == 0 {
                    queue.insert(dependent);
                }
            }
        }
    }
    if order.len() == names.len() {
        Ok(order)
    } else {
        Err(in_degree
            .iter()
            .filter(|&(_, &degree)| degree > 0)
            .map(|(&name, _)| name.to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_protocol_types::{HandshakeResponse, PeerDependency};

    /// A declaration of `name` (version 1.0.0) whose only content is `peers`.
    fn make_manifest(name: &str, peers: &[(&str, &str)]) -> ExtensionDeclaration {
        ExtensionDeclaration {
            handshake: HandshakeResponse {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                peer_dependencies: peers
                    .iter()
                    .map(|(n, v)| PeerDependency {
                        name: n.to_string(),
                        version: v.to_string(),
                        optional: false,
                    })
                    .collect(),
                ..HandshakeResponse::default()
            },
            ..ExtensionDeclaration::default()
        }
    }

    // B:topological_sort_extensions — verify unit "extensions sorted in dependency order"
    #[test]
    fn test_extensions_sorted_in_dependency_order() {
        let manifests = vec![
            make_manifest(
                "@specforge/governance",
                &[("@specforge/software", ">=1.0.0")],
            ),
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
        assert_eq!(err[0].code, "E027");
        assert!(err[0].message.contains("cycle"));
    }

    /// A declaration of `name` whose peers are `required` and `optional`.
    fn with_peers(name: &str, required: &[&str], optional: &[&str]) -> ExtensionDeclaration {
        let mut manifest = make_manifest(name, &[]);
        let peer = |n: &&str, optional: bool| PeerDependency {
            name: n.to_string(),
            version: ">=1.0.0".to_string(),
            optional,
        };
        manifest.handshake.peer_dependencies = required
            .iter()
            .map(|n| peer(n, false))
            .chain(optional.iter().map(|n| peer(n, true)))
            .collect();
        manifest
    }

    #[specforge_test_macros::test(
        behavior = "topological_sort_extensions",
        verify = "extensions naming each other as optional peers sort without a cycle"
    )]
    fn a_mutual_optional_pair_sorts_by_name() {
        let manifests = vec![
            with_peers("B", &[], &["A"]),
            with_peers("A", &[], &["B"]),
            with_peers("C", &[], &[]),
        ];
        // A's optional edge on B comes first in name order and is kept;
        // B's on A would close a cycle and is dropped. Same on any input
        // order.
        let order = topological_sort_extensions(&manifests).unwrap();
        assert_eq!(order, ["B", "A", "C"]);
        let reversed: Vec<_> = manifests.into_iter().rev().collect();
        assert_eq!(topological_sort_extensions(&reversed).unwrap(), order);
    }

    #[specforge_test_macros::test(
        behavior = "topological_sort_extensions",
        verify = "a cycle among required peers is E027, an optional edge closing a cycle is dropped"
    )]
    fn only_a_required_cycle_is_e027() {
        // Required cycle: E027, naming only the extensions on it.
        let required = vec![
            with_peers("A", &["B"], &[]),
            with_peers("B", &["A"], &[]),
            with_peers("C", &[], &["A"]),
        ];
        let err = topological_sort_extensions(&required).unwrap_err();
        assert_eq!(err[0].code, "E027");
        assert!(err[0].message.ends_with(": A, B"), "{}", err[0].message);

        // Mixed cycle: A requires B, B has A as an optional peer: the
        // required edge decides, the optional one is dropped.
        let mixed = vec![with_peers("A", &["B"], &[]), with_peers("B", &[], &["A"])];
        assert_eq!(topological_sort_extensions(&mixed).unwrap(), ["B", "A"]);

        // A longer mixed cycle: X requires Y, Y requires Z, Z optionally
        // peers on X.
        let longer = vec![
            with_peers("X", &["Y"], &[]),
            with_peers("Y", &["Z"], &[]),
            with_peers("Z", &[], &["X"]),
        ];
        assert_eq!(
            topological_sort_extensions(&longer).unwrap(),
            ["Z", "Y", "X"]
        );
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
            make_manifest(
                "@specforge/governance",
                &[("@specforge/software", ">=1.0.0")],
            ),
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
        assert_eq!(err[0].code, "E027");
        assert_eq!(err[0].severity, Severity::Error);
    }
}
