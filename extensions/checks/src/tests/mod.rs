//! The guest's tests, one module per family, against the declaration and the handlers themselves.

use specforge_extension_sdk::testing::MockHost;
use specforge_protocol_types::{
    CheckFinding, ExtensionDeclaration, PlannedItem, PortScope, SettingDescriptor, SettingsItem,
};
use specforge_test::prelude::*;

mod digits;
mod facts;
mod files;
mod formats;
mod git;
mod links;
mod media;
mod structured;

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
    verify = "the extension declares the scopes files, links, git and media-remote, and one switch per family on by default"
)]
fn declares_four_scopes_and_a_switch_per_family() {
    let declaration = declaration();
    let scopes: Vec<_> = declaration
        .capabilities
        .iter()
        .map(|capability| (capability.name.as_str(), &capability.port))
        .collect();
    assert!(matches!(scopes[0], ("files", PortScope::Fs(_))));
    assert!(matches!(scopes[1], ("links", PortScope::Http(_))));
    assert!(matches!(scopes[2], ("git", PortScope::Git(_))));
    assert!(matches!(scopes[3], ("media-remote", PortScope::Http(_))));
    assert_eq!(scopes.len(), 4);

    let switches = switches();
    let names: Vec<&str> = switches.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "families.files",
            "families.formats",
            "families.digits",
            "families.links",
            "families.git",
            "families.structured",
            "families.media"
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
    verify = "each switch gates only codes its family alone reports, and the shared E103, E016, W176 and network warnings under no switch"
)]
fn each_switch_gates_only_its_familys_codes() {
    let switches = switches();
    let gated: Vec<&str> = switches
        .iter()
        .flat_map(|switch| switch.gates.iter().map(String::as_str))
        .collect();
    let distinct: std::collections::BTreeSet<&str> = gated.iter().copied().collect();
    assert_eq!(distinct.len(), gated.len(), "a code gated twice: {gated:?}");
    for shared in ["E103", "E016", "W176", "W172", "W173", "W181", "W182"] {
        assert!(!distinct.contains(shared), "{shared} is shared");
    }
    let of = |name: &str| {
        let switch = switches.iter().find(|s| s.name == name).unwrap();
        switch.gates.clone()
    };
    assert_eq!(of("families.files"), ["E096"]);
    assert_eq!(of("families.structured"), ["E097", "E098"]);
    assert_eq!(of("families.git"), ["E106", "W180"]);
    assert!(of("families.formats").is_empty() && of("families.digits").is_empty());
    assert_eq!(of("families.media"), ["E099", "E113", "W177"]);
    assert_eq!(
        of("families.links"),
        ["E091", "E092", "E093", "E104", "E112", "W178", "W183"]
    );
}

/// What `checker` plans for `text` under `args`.
fn planned(checker: &str, text: &str, args: serde_json::Value) -> PlannedItem {
    MockHost::new(crate::specforge_extension_build()).plan(checker, text, args)
}

fn finding(checker: &str, text: &str, args: serde_json::Value) -> Option<CheckFinding> {
    let plan = planned(checker, text, args);
    assert!(plan.findings.len() < 2, "{:?}", plan.findings);
    plan.findings.into_iter().next()
}

fn clean(checker: &str, text: &str, args: serde_json::Value) {
    assert_eq!(finding(checker, text, args), None, "{checker} on '{text}'");
}

fn e103(checker: &str, text: &str, args: serde_json::Value) -> CheckFinding {
    let found =
        finding(checker, text, args).unwrap_or_else(|| panic!("{checker} accepted '{text}'"));
    assert_eq!(found.code, "E103");
    found
}

/// The bytes of the value an E103 points at.
fn e103_at(checker: &str, text: &str, args: serde_json::Value) -> [usize; 2] {
    let found = e103(checker, text, args);
    found
        .at
        .unwrap_or_else(|| panic!("{checker} on '{text}' points at nothing"))
}

/// What `checker` finds in a value of `text` once its one probe answered `outcome`.
fn judged(
    checker: &str,
    text: &str,
    args: serde_json::Value,
    outcome: specforge_protocol_types::ProbeOutcome,
) -> Vec<CheckFinding> {
    MockHost::new(crate::specforge_extension_build()).judge(checker, text, args, vec![outcome])
}
