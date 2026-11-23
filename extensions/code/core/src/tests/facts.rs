//! The collected kinds each code judges (ADR 0389 D4), which doctor reads, and the packs W102 names.

use super::*;

/// The kinds each catalogued code names, as the declaration carries them.
fn judged() -> Vec<(String, Vec<String>)> {
    declaration()
        .codes
        .into_iter()
        .map(|code| (code.code, code.facts))
        .collect()
}

#[test]
fn each_code_names_the_collected_kinds_it_judges() {
    let expected: Vec<(String, Vec<String>)> = [
        ("E101", &["item", "package", "test", "test_run"][..]),
        ("W102", &[]),
        ("W401", &["test"]),
        ("I402", &["item", "test"]),
        ("W104", &["api_item", "coverage", "static_finding"]),
        ("W105", &["static_finding"]),
        ("W106", &["dependency"]),
        ("W107", &["vulnerability"]),
    ]
    .iter()
    .map(|(code, kinds)| (code.to_string(), strings(kinds)))
    .collect();

    assert_eq!(judged(), expected);
}

#[test]
fn every_judged_kind_is_a_kind_it_declares() {
    let declared = declaration();
    for (code, kinds) in judged() {
        for kind in kinds {
            assert!(
                KINDS.contains(&kind.as_str()) || REPORT_KINDS.contains(&kind.as_str()),
                "{code} judges {kind}"
            );
            assert!(
                declared.entities.iter().any(|k| k.name == kind),
                "{code} judges {kind}, which the declaration has no kind of"
            );
        }
    }
}

#[test]
fn w102_names_every_runner_pack_of_a_test_scheme_and_one_pack_otherwise() {
    let packs = |kind: &str, family: &str| -> Vec<&str> {
        refs::PACKS
            .iter()
            .find(|(k, f, _)| *k == kind && *f == family)
            .map(|(_, _, packs)| packs.to_vec())
            .unwrap_or_default()
    };
    assert_eq!(
        packs("test", "js"),
        [
            "@javascript/node-test",
            "@javascript/vitest",
            "@javascript/jest",
            "@javascript/playwright"
        ]
    );
    assert_eq!(packs("test", "rust"), ["@rust/test", "@rust/proof"]);
    for (kind, family, named) in refs::PACKS {
        if kind != "test" {
            assert_eq!(named.len(), 1, "{kind} {family}: {named:?}");
        }
    }
}

#[test]
fn the_handshake_is_version_1_4_0() {
    assert_eq!(declaration().handshake.version, "1.4.0");
}
