use specforge_test::prelude::*;

/// Formal stands alone: its peers are optional, and the rules whose only
/// possible reference is another extension's kind name that edge, so they
/// are inert where the kind does not load (ADR 0280, ADR 0020 D5).
#[specforge_test(
    behavior = "fa_declare_manifest",
    verify = "software and governance are optional peers, and W123 and W128 name the edge their message names"
)]
fn formal_needs_neither_software_nor_governance() {
    let declaration = crate::specforge_extension_build().declaration();
    let peers: Vec<(&str, bool)> = declaration
        .peers()
        .iter()
        .map(|p| (p.name.as_str(), p.optional))
        .collect();
    assert_eq!(
        peers,
        [
            ("@specforge/software", true),
            ("@specforge/governance", true)
        ]
    );
    for (code, edge) in [
        ("W123", "BehaviorSatisfiesProperty"),
        ("W128", "EventFollowsProtocol"),
    ] {
        let rule = declaration
            .validation_rules
            .iter()
            .find(|r| r.code == code)
            .unwrap_or_else(|| panic!("{code} is declared"));
        assert_eq!(rule.edge_type.as_deref(), Some(edge), "{code}");
    }
}
