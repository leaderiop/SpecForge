//! The two sets of builtins that load into one registry (ADR 0369 D3). A set is a list of builtins by
//! name. Two builtins that declare one kind keyword are E026 in a registry build (the first
//! registration wins), so a test that builds a registry from builtins builds it from one set, and
//! these tests hold the sets themselves: who is in which, and that each set builds clean.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::collections::BTreeSet;

use specforge_common::DiagnosticData;
use specforge_component::builtins::{BUILTINS, Family};
use specforge_component::committed::pinned_builtin_declarations;
use specforge_protocol_types::{
    EntityKindDescriptor, ExtensionDeclaration, HandshakeResponse, PROTOCOL_VERSION,
};
use specforge_registry::{CandidateVocabulary, RegistryBuild, build_registries};

fn set(family: Family) -> BTreeSet<&'static str> {
    family.names().iter().copied().collect()
}

#[test]
fn the_two_sets_are_adr_0369s_and_every_builtin_is_in_one() {
    let old = set(Family::Old);
    let gsd = set(Family::Gsd);
    let names = |of: &BTreeSet<&'static str>| -> Vec<&'static str> { of.iter().copied().collect() };
    let sorted = |mut list: Vec<&'static str>| {
        list.sort_unstable();
        list
    };

    assert_eq!(
        names(&old.difference(&gsd).copied().collect()),
        sorted(vec![
            "@specforge/product",
            "@specforge/software",
            "@specforge/governance",
            "@specforge/formal",
            "@specforge/testing",
            "@specforge/cargo-test",
            "@specforge/vitest",
        ]),
        "the old set only"
    );
    assert_eq!(
        names(&old.intersection(&gsd).copied().collect()),
        sorted(vec![
            "@code/core",
            "@specforge/checks",
            "@rust/cargo",
            "@rust/source",
            "@rust/test",
            "@javascript/source",
            "@javascript/node-test",
            "@rust/coverage",
            "@rust/lint",
            "@rust/deps",
            "@rust/api",
            "@rust/proof",
            "@javascript/coverage",
            "@javascript/lint",
            "@javascript/deps",
            "@javascript/api",
            "@javascript/vitest",
            "@javascript/jest",
            "@javascript/playwright",
        ]),
        "both sets"
    );
    assert_eq!(
        names(&gsd.difference(&old).copied().collect()),
        ["@gsd/core"],
        "the gsd set only"
    );

    let rows: BTreeSet<&str> = BUILTINS.iter().map(|builtin| builtin.name).collect();
    for builtin in BUILTINS {
        assert!(
            old.contains(builtin.name) || gsd.contains(builtin.name),
            "'{}' is a BUILTINS row in neither set: ADR 0369 D3 places every builtin",
            builtin.name
        );
    }
    for name in &old {
        assert!(
            rows.contains(name),
            "'{name}' of the old set is no BUILTINS row"
        );
    }
    for name in &gsd {
        assert!(
            rows.contains(name),
            "'{name}' of the gsd set is no BUILTINS row"
        );
    }
}

#[test]
fn each_set_builds_one_registry_with_no_e026_and_no_missing_peer() {
    for family in Family::ALL {
        let names = set(family);
        // A misspelt name would shrink the set silently: each name is an embedded builtin.
        assert_eq!(
            family.builtins().count(),
            family.names().len(),
            "{family:?}: every name of the set is an embedded builtin"
        );
        let declarations: Vec<_> = pinned_builtin_declarations()
            .unwrap()
            .into_iter()
            .filter(|declaration| names.contains(declaration.name()))
            .collect();
        assert_eq!(
            declarations.len(),
            family.builtins().count(),
            "{family:?}: one pinned declaration per embedded builtin of the set"
        );

        let build = build_registries(declarations);
        for code in ["E026", "E027"] {
            let found: Vec<_> = build.diagnostics().filter(|d| d.code == code).collect();
            assert!(found.is_empty(), "{family:?}: {code}: {found:?}");
        }
    }
}

/// The kinds `@gsd/core` declares, as a declaration named `@gsd/core` with nothing else in it. They are
/// read from `gsd/core/tests/golden/kinds.json`, which the gsd suite's own golden test keeps equal to
/// the extension's declaration, so this follows every later change to those kinds with no edit here.
fn gsd_core_kinds_golden() -> ExtensionDeclaration {
    let path = specforge_repo_root::repo_root().join("gsd/core/tests/golden/kinds.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let golden: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let entities: Vec<EntityKindDescriptor> =
        serde_json::from_value(golden["entities"]["items"].clone())
            .unwrap_or_else(|e| panic!("{}: .entities.items: {e}", path.display()));
    assert!(!entities.is_empty(), "{}: no kind", path.display());
    ExtensionDeclaration {
        handshake: HandshakeResponse {
            name: "@gsd/core".to_string(),
            version: "0.0.0".to_string(),
            protocol_version: PROTOCOL_VERSION.to_string(),
            ..Default::default()
        },
        entities,
        ..Default::default()
    }
}

/// The keywords of the E026 diagnostics of `build`.
fn shadowed_keywords(build: &RegistryBuild) -> BTreeSet<String> {
    build
        .diagnostics()
        .filter(|d| d.code == "E026")
        .map(|d| match d.data.as_deref() {
            Some(DiagnosticData::ShadowedKeyword { keyword }) => keyword.clone(),
            other => panic!("E026 carries no shadowed keyword: {other:?}"),
        })
        .collect()
}

/// `@gsd/core` with its set alone reports no E026, and the same registry with the builtins of the old
/// set only added reports E026 for exactly the keywords `@gsd/core` shares with them. `in_set` holds the
/// declarations of the gsd set's other builtins. The shared keywords are computed from the declarations
/// with the vocabulary's own keyword rule, never listed. Returns the build of the set alone.
#[expect(
    clippy::print_stdout,
    reason = "the shared keywords are the control's finding: a run with --success-output shows them"
)]
fn the_gsd_set_alone_reports_no_e026(
    in_set: Vec<ExtensionDeclaration>,
    gsd_core: ExtensionDeclaration,
) -> RegistryBuild {
    assert_eq!(gsd_core.name(), "@gsd/core");
    let gsd = set(Family::Gsd);
    let old = set(Family::Old);
    let mut declarations = in_set;
    declarations.retain(|declaration| declaration.name() != "@gsd/core");
    declarations.push(gsd_core.clone());

    let alone = build_registries(declarations.clone());
    assert_eq!(
        shadowed_keywords(&alone),
        BTreeSet::new(),
        "@gsd/core with its set alone"
    );

    // The control: the builtins that are in the old set only declare keywords `@gsd/core` declares.
    let old_only: Vec<ExtensionDeclaration> = pinned_builtin_declarations()
        .unwrap()
        .into_iter()
        .filter(|declaration| old.contains(declaration.name()) && !gsd.contains(declaration.name()))
        .collect();
    let owners = CandidateVocabulary::of(&old_only).unwrap();
    let staged = CandidateVocabulary::of(std::slice::from_ref(&gsd_core)).unwrap();
    let shared: BTreeSet<String> = staged
        .kind_keywords()
        .filter(|keyword| !owners.kind_owners(keyword).is_empty())
        .map(str::to_string)
        .collect();
    println!(
        "shared kind keywords ({}): {}",
        shared.len(),
        shared.iter().cloned().collect::<Vec<_>>().join(", ")
    );
    assert!(!shared.is_empty(), "the control shares no keyword");
    for keyword in ["decision", "deliverable", "milestone"] {
        assert!(
            shared.contains(keyword),
            "'{keyword}' is not shared: {shared:?}"
        );
    }

    declarations.extend(old_only);
    let together = build_registries(declarations);
    assert_eq!(
        shadowed_keywords(&together),
        shared,
        "the E026 of the set with the old-only builtins added"
    );
    alone
}

/// `@gsd/core` with its set alone reports no E026, here with its kinds read from the tracked golden, so
/// it runs in every test run. The control shows the test can see E026, which is a registry build's: the
/// same registry with the old-only builtins added reports it for the keywords they share. The vendored
/// `@gsd/core` row loads with its set in `each_set_builds_one_registry_with_no_e026_and_no_missing_peer`.
#[test]
fn gsd_cores_kinds_golden_with_its_set_reports_no_e026() {
    let gsd = set(Family::Gsd);
    let in_set: Vec<ExtensionDeclaration> = pinned_builtin_declarations()
        .unwrap()
        .into_iter()
        .filter(|declaration| gsd.contains(declaration.name()))
        .collect();
    the_gsd_set_alone_reports_no_e026(in_set, gsd_core_kinds_golden());
}
