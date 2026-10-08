// Wasm lifecycle integration tests through the public API:
// - B:topological_sort_extensions

use specforge_common::Severity;
use specforge_protocol_types::PeerDependency;
use specforge_wasm::topological_sort_extensions;

fn make_declaration(
    name: &str,
    version: &str,
    peers: &[(&str, &str)],
) -> specforge_protocol_types::ExtensionDeclaration {
    specforge_protocol_types::ExtensionDeclaration {
        handshake: specforge_protocol_types::HandshakeResponse {
            name: name.to_string(),
            version: version.to_string(),
            peer_dependencies: peers
                .iter()
                .map(|(n, v)| PeerDependency {
                    name: n.to_string(),
                    version: v.to_string(),
                    optional: false,
                })
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ============================================================================
// B:topological_sort_extensions — integration tests
// ============================================================================

// B:topological_sort_extensions — verify integration "linear dependency chain → correct order"
#[test]
fn test_toposort_linear_chain() {
    let manifests = vec![
        make_declaration(
            "@specforge/governance",
            "1.0.0",
            &[("@specforge/software", ">=1.0.0")],
        ),
        make_declaration("@specforge/software", "1.0.0", &[]),
    ];

    let order = topological_sort_extensions(&manifests).unwrap();
    assert_eq!(order, vec!["@specforge/software", "@specforge/governance"]);
}

// B:topological_sort_extensions — verify integration "diamond dependency → both paths respected"
#[test]
fn test_toposort_diamond_dependency() {
    let manifests = vec![
        make_declaration("@specforge/software", "1.0.0", &[]),
        make_declaration(
            "@specforge/product",
            "1.0.0",
            &[("@specforge/software", ">=1.0.0")],
        ),
        make_declaration(
            "@specforge/governance",
            "1.0.0",
            &[("@specforge/software", ">=1.0.0")],
        ),
        make_declaration(
            "@specforge/dashboard",
            "1.0.0",
            &[
                ("@specforge/product", ">=1.0.0"),
                ("@specforge/governance", ">=1.0.0"),
            ],
        ),
    ];

    let order = topological_sort_extensions(&manifests).unwrap();
    // software must be first, dashboard must be last
    assert_eq!(order[0], "@specforge/software");
    assert_eq!(order[order.len() - 1], "@specforge/dashboard");
    assert_eq!(order.len(), 4);
}

// B:topological_sort_extensions — verify integration "cycle detected → E031 diagnostic"
#[test]
fn test_toposort_cycle_produces_e027() {
    let manifests = vec![
        make_declaration("A", "1.0.0", &[("B", ">=1.0.0")]),
        make_declaration("B", "1.0.0", &[("C", ">=1.0.0")]),
        make_declaration("C", "1.0.0", &[("A", ">=1.0.0")]),
    ];

    let err = topological_sort_extensions(&manifests).unwrap_err();
    assert_eq!(err.len(), 1);
    assert_eq!(err[0].code, "E027");
    assert_eq!(err[0].severity, Severity::Error);
    assert!(err[0].message.contains("cycle"));
}

// B:topological_sort_extensions — verify contract "requires manifests, ensures sorted or cycle error"
#[test]
fn test_toposort_contract() {
    // ensures: deterministic ordering on ties (alphabetical)
    let manifests = vec![
        make_declaration("Z-ext", "1.0.0", &[]),
        make_declaration("A-ext", "1.0.0", &[]),
        make_declaration("M-ext", "1.0.0", &[]),
    ];
    let order = topological_sort_extensions(&manifests).unwrap();
    assert_eq!(order, vec!["A-ext", "M-ext", "Z-ext"]);

    // ensures: empty input → empty output
    let empty = topological_sort_extensions(&[]).unwrap();
    assert!(empty.is_empty());
}
