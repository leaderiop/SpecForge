//! The guest's tests, one module per family, against the declaration and the handlers themselves.

use specforge_protocol_types::{ExtensionDeclaration, PortScope, SettingDescriptor, SettingsItem};
use specforge_test::prelude::*;

fn declaration() -> ExtensionDeclaration {
    crate::specforge_extension_build().declaration()
}

fn switches() -> Vec<SettingDescriptor> {
    declaration()
        .settings
        .into_iter()
        .filter_map(|item| match item {
            SettingsItem::Setting(setting) => Some(setting),
            _ => None,
        })
        .collect()
}

#[specforge_test(
    behavior = "ck_family_switches",
    verify = "the extension declares the scopes files, links and git, and five family switches on by default"
)]
fn declares_three_scopes_and_five_switches() {
    let declaration = declaration();
    let scopes: Vec<_> = declaration
        .capabilities
        .iter()
        .map(|capability| (capability.name.as_str(), &capability.port))
        .collect();
    assert!(matches!(scopes[0], ("files", PortScope::Fs(_))));
    assert!(matches!(scopes[1], ("links", PortScope::Http(_))));
    assert!(matches!(scopes[2], ("git", PortScope::Git(_))));
    assert_eq!(scopes.len(), 3);

    let switches = switches();
    let names: Vec<&str> = switches.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "families.files",
            "families.formats",
            "families.digits",
            "families.links",
            "families.git"
        ]
    );
    for switch in &switches {
        assert_eq!(switch.field_type, "bool", "{}", switch.name);
        assert_eq!(
            switch.default,
            Some(serde_json::json!(true)),
            "{}",
            switch.name
        );
    }
}

#[specforge_test(
    behavior = "ck_family_switches",
    verify = "each switch gates only codes its family alone reports, and E103 under no switch"
)]
fn each_switch_gates_only_its_familys_codes() {
    let switches = switches();
    let gated: Vec<&str> = switches
        .iter()
        .flat_map(|switch| switch.gates.iter().map(String::as_str))
        .collect();
    let distinct: std::collections::BTreeSet<&str> = gated.iter().copied().collect();
    assert_eq!(distinct.len(), gated.len(), "a code gated twice: {gated:?}");
    assert!(!distinct.contains("E103"));
    let of = |name: &str| {
        let switch = switches.iter().find(|s| s.name == name).unwrap();
        switch.gates.clone()
    };
    assert_eq!(of("families.files"), ["E096", "W176"]);
    assert_eq!(of("families.git"), ["E106", "W180"]);
    assert!(of("families.formats").is_empty() && of("families.digits").is_empty());
    assert_eq!(of("families.links").len(), 11);
}
