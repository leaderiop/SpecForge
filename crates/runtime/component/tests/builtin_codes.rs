//! Every builtin declares only codes it owns, at the level the catalog gives
//! them. The builtins write their rule codes as text and do not link the
//! catalog (a guest that did would be rebuilt on every explanation edit,
//! ADR 0026 D4); the registry build checks each rule when it registers it
//! (`Codebook::check`, W150), and this test checks them all, so a
//! mismatch fails a test and not only a user's compile. A `@specforge/`
//! builtin's codes are bare, in the static catalog, and it declares none; any
//! other builtin that reports (`specforge_registry::reports`: rules or
//! passes) catalogues its own, as a third party does, and one that only
//! collects has none to catalogue (ADR 0286 D2, ADR 0328 D2).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_component::ComponentRuntime;
use specforge_component::builtins::BUILTIN_EXTENSIONS;
use specforge_diagnostics::{Codebook, Level};
use specforge_protocol_types::{SettingsItem, ValidationSeverity};
use specforge_wasm::WasmRuntime as _;
use specforge_wasm::protocol::load_declaration;

fn level(severity: &ValidationSeverity) -> Level {
    match severity {
        ValidationSeverity::Error => Level::Error,
        ValidationSeverity::Warning => Level::Warning,
        ValidationSeverity::Info => Level::Info,
    }
}

#[specforge_test_macros::test(requirement = "FEATURE-016")]
fn builtin_rules_use_their_own_codes() {
    let runtime = ComponentRuntime::new();
    let mut checked = 0;
    let mut problems = Vec::new();
    for (name, bytes) in BUILTIN_EXTENSIONS {
        runtime
            .load(name, bytes, specforge_wasm::Provenance::third_party(bytes))
            .expect("builtin loads");
        let declaration = load_declaration(&runtime, name)
            .expect("builtin declares itself")
            .declaration;
        assert_eq!(declaration.name(), *name, "a builtin is named by its blob");
        // A first-party builtin's codes are in the static catalog, bare (ADR 0286 D2); any other
        // builtin's rules are checked against the static catalog plus its own (ADR 0328 D2).
        let mut book = Codebook::default();
        if name.starts_with("@specforge/") {
            assert!(declaration.codes.is_empty(), "{name} declares `codes`");
        } else {
            assert!(
                !specforge_registry::reports(&declaration) || !declaration.codes.is_empty(),
                "{name} is no @specforge/ builtin, reports through rules or passes, and catalogues no codes"
            );
            // The registry build's own reading, which also drops a malformed entry (E030).
            let codes = specforge_registry::extension_codes(&declaration);
            assert_eq!(
                codes.len(),
                declaration.codes.len(),
                "{name}: a catalogued code is malformed"
            );
            book.add(name, codes)
                .unwrap_or_else(|shared| panic!("{name}: {shared:?}"));
        }
        for rule in &declaration.validation_rules {
            checked += 1;
            if let Err(misuse) = book.check(declaration.name(), &rule.code, level(&rule.severity)) {
                problems.push(format!(
                    "{name}: rule '{}' ({:?}) uses {misuse}",
                    rule.code, rule.severity
                ));
            }
        }
    }
    assert!(
        checked >= 75,
        "only {checked} builtin rules were read; do the builtins still declare their rules?"
    );
    assert!(
        problems.is_empty(),
        "builtin rules use codes their extension may not (W150 on every compile):\n  {}",
        problems.join("\n  ")
    );
}

/// Every `@gsd/core` error code that an importing command's plan writes rather than refuses (ADR 0392): all of them,
/// as none declares `facts`. `E533` and `E535` read API and package facts but judge a closed milestone's claim, which
/// is history, so they land too (ADR 0392, Limits).
const GSD_CORE_LANDS: &[&str] = &[
    "E101", "E102", "E105", "E106", "E107", "E108", "E201", "E202", "E206", "E301", "E302", "E303",
    "E307", "E310", "E312", "E314", "E319", "E320", "E322", "E401", "E402", "E405", "E501", "E502",
    "E503", "E505", "E507", "E509", "E511", "E512", "E513", "E515", "E516", "E517", "E518", "E521",
    "E523", "E533", "E535",
];

#[specforge_test_macros::test(requirement = "FEATURE-131")]
fn the_gsd_core_errors_an_import_lands_are_pinned() {
    let name = "@gsd/core";
    let (_, bytes) = BUILTIN_EXTENSIONS
        .iter()
        .find(|(builtin, _)| *builtin == name)
        .expect("@gsd/core is a builtin");
    let runtime = ComponentRuntime::new();
    runtime
        .load(name, bytes, specforge_wasm::Provenance::third_party(bytes))
        .expect("builtin loads");
    let declaration = load_declaration(&runtime, name)
        .expect("builtin declares itself")
        .declaration;
    // The vendored blob read as the registry build reads it, so the pin follows what the host judges.
    let codes = specforge_registry::extension_codes(&declaration);
    let errors: Vec<String> = codes
        .iter()
        .filter(|code| code.local.starts_with('E'))
        .map(|code| code.local.clone())
        .collect();
    let mut book = Codebook::default();
    book.add(name, codes).unwrap();
    let (mut lands, mut refused): (Vec<&str>, Vec<&str>) = errors
        .iter()
        .map(String::as_str)
        .partition(|local| book.authored_rule(name, &format!("gsd/{local}")));
    lands.sort_unstable();
    refused.sort_unstable();
    assert!(
        refused.is_empty(),
        "these @gsd/core errors now refuse an import (their catalog entry declares `facts`, ADR 0392); \
         if that is meant, amend ADR 0392's Limits and this pin: {refused:?}"
    );
    assert_eq!(
        lands, GSD_CORE_LANDS,
        "the @gsd/core errors an import lands changed: update GSD_CORE_LANDS (ADR 0392)"
    );
}

#[specforge_test_macros::test(requirement = "FEATURE-007")]
fn the_checks_switches_gate_codes_the_catalog_gives_checks() {
    let runtime = ComponentRuntime::new();
    let (name, bytes) = BUILTIN_EXTENSIONS
        .iter()
        .find(|(name, _)| *name == "@specforge/checks")
        .expect("@specforge/checks is a builtin");
    runtime
        .load(name, bytes, specforge_wasm::Provenance::third_party(bytes))
        .expect("builtin loads");
    let declaration = load_declaration(&runtime, name)
        .expect("builtin declares itself")
        .declaration;
    let mut gated = 0;
    for item in &declaration.settings {
        let SettingsItem::Setting(setting) = item else {
            panic!("a checks setting is a family switch: {item:?}");
        };
        assert!(setting.name.starts_with("families."), "{}", setting.name);
        for code in &setting.gates {
            let entry = specforge_diagnostics::lookup(code)
                .unwrap_or_else(|| panic!("{code} is not catalogued"));
            assert_eq!(entry.owner, *name, "{code}");
            gated += 1;
        }
    }
    assert!(gated > 0, "no switch gates a code");
}
