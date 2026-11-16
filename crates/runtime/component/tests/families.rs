//! The two sets of builtins that load into one registry (ADR 0369 D3). A set is a list of builtins by
//! name. Two builtins that declare one kind keyword are E026 in a registry build (the first
//! registration wins), so a test that builds a registry from builtins builds it from one set, and
//! these tests hold the sets themselves: who is in which, and that each set builds clean.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::collections::BTreeSet;

use specforge_component::builtins::{BUILTINS, Family};
use specforge_component::committed::pinned_builtin_declarations;
use specforge_registry::build_registries;

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
    let unvendored: Vec<&str> = gsd
        .iter()
        .copied()
        .filter(|name| !rows.contains(name))
        .collect();
    assert!(
        unvendored.is_empty() || unvendored == ["@gsd/core"],
        "the gsd set names builtins with no row besides @gsd/core: {unvendored:?}"
    );
}

#[test]
fn each_set_builds_one_registry_with_no_e026_and_no_missing_peer() {
    for family in Family::ALL {
        let names = set(family);
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
