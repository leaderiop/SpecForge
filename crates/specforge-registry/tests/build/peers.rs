//! `registry_build_peer_dependencies`: every declared peer is checked
//! against the loaded declarations' versions as a semver range. An
//! unsatisfied peer is E027; a range or version that is not semver is W062.

use specforge_common::Severity;
use specforge_extension_sdk::prelude::*;
use specforge_test_macros::test as spec;

use crate::support::{
    build, coded, codes, declare, diagnostics, extension, optional_peer, peer, software, versioned,
};

/// A pass's handler that finds nothing.
fn no_findings(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "satisfied peer dependency passes validation"
)]
fn a_required_peer_in_range_passes() {
    let product = versioned(
        "@specforge/product",
        "1.0.0",
        vec![peer("@specforge/software", ">=1.0.0")],
    );
    let build = build([software(), product]);
    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "missing peer dependency produces hard error"
)]
fn a_missing_required_peer_is_e027() {
    let build = build([versioned(
        "@specforge/product",
        "1.0.0",
        vec![peer("@specforge/software", ">=1.0.0")],
    )]);

    let e027 = coded(&build, "E027");
    assert_eq!(e027.len(), 1, "{:?}", diagnostics(&build));
    assert_eq!(e027[0].severity, Severity::Error);
    assert_eq!(
        e027[0].message,
        "extension '@specforge/product' requires peer dependency '@specforge/software' >=1.0.0 which is not installed"
    );
    assert_eq!(
        e027[0].suggestion.as_deref(),
        Some("install it with: specforge add @specforge/software")
    );
    assert!(build.declaration_diagnostics.contains(e027[0]));
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "incompatible version produces hard error with required range"
)]
fn a_peer_out_of_range_is_e027_naming_range_and_version() {
    let build = build([
        versioned("@specforge/software", "0.5.0", vec![]),
        versioned(
            "@specforge/product",
            "1.0.0",
            vec![peer("@specforge/software", ">=1.0.0")],
        ),
    ]);

    let e027 = coded(&build, "E027");
    assert_eq!(e027.len(), 1, "{:?}", diagnostics(&build));
    assert_eq!(e027[0].severity, Severity::Error);
    assert_eq!(
        e027[0].message,
        "extension '@specforge/product' requires peer dependency '@specforge/software' >=1.0.0 but version 0.5.0 is installed"
    );
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "missing optional peer dependency passes validation"
)]
fn a_missing_optional_peer_passes() {
    let build = build([versioned(
        "@specforge/software",
        "1.0.0",
        vec![optional_peer("@specforge/product", "^1.0")],
    )]);
    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "installed optional peer outside its range produces hard error"
)]
fn an_installed_optional_peer_out_of_range_is_e027() {
    let build = build([
        versioned("@specforge/product", "2.0.0", vec![]),
        versioned(
            "@specforge/software",
            "1.0.0",
            vec![optional_peer("@specforge/product", "^1.0")],
        ),
    ]);

    assert_eq!(codes(&build), ["E027"]);
    let e027 = coded(&build, "E027");
    assert!(
        e027[0].message.contains("^1.0") && e027[0].message.contains("2.0.0"),
        "{}",
        e027[0].message
    );
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "a peer range matches as semver: caret, tilde or exact"
)]
fn a_peer_range_matches_as_semver() {
    for (range, installed, satisfied) in [
        ("^1.0.0", "1.2.3", true),
        ("^1.0.0", "2.0.0", false),
        ("~1.2.0", "1.2.5", true),
        ("~1.2.0", "1.3.0", false),
        ("1.0.0", "1.0.0", true),
        ("=1.0.0", "1.0.1", false),
        (">=1.0.0", "0.5.0", false),
        (">=1.0.0", "3.1.0", true),
    ] {
        let build = build([
            versioned("@specforge/software", installed, vec![]),
            versioned(
                "@specforge/product",
                "1.0.0",
                vec![peer("@specforge/software", range)],
            ),
        ]);
        let expected: &[&str] = if satisfied { &[] } else { &["E027"] };
        assert_eq!(codes(&build), expected, "{range} against {installed}");
    }
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "a malformed peer range or installed version is W062"
)]
fn a_malformed_peer_range_or_installed_version_is_w062() {
    let malformed_range = build([
        versioned("@specforge/software", "1.0.0", vec![]),
        versioned(
            "@specforge/product",
            "1.0.0",
            vec![peer("@specforge/software", "not-a-version")],
        ),
    ]);
    assert_eq!(codes(&malformed_range), ["W062"]);
    let w062 = coded(&malformed_range, "W062");
    assert_eq!(w062[0].severity, Severity::Warning);
    assert!(
        w062[0].message.contains("'not-a-version'"),
        "{}",
        w062[0].message
    );

    let malformed_version = build([
        versioned("@specforge/software", "bad-version", vec![]),
        versioned(
            "@specforge/product",
            "1.0.0",
            vec![peer("@specforge/software", "^1.0.0")],
        ),
    ]);
    assert_eq!(codes(&malformed_version), ["W062"]);
    let w062 = coded(&malformed_version, "W062");
    assert_eq!(w062[0].severity, Severity::Warning);
    assert!(
        w062[0].message.contains("'bad-version'"),
        "{}",
        w062[0].message
    );
}

/// Pinned until T2 (ADR 0041): a malformed range on a missing peer is judged
/// as a missing peer.
#[test]
fn pin_a_malformed_range_on_a_missing_peer_is_e027() {
    let build = build([versioned(
        "@t/bad",
        "1.0.0",
        vec![peer("@t/base", "one-ish")],
    )]);
    assert_eq!(codes(&build), ["E027"]);
    assert!(
        coded(&build, "E027")[0]
            .message
            .ends_with("'@t/base' one-ish which is not installed"),
        "{:?}",
        diagnostics(&build)
    );
}

/// Pinned until T2: a malformed range on an absent optional peer is silent.
#[test]
fn pin_a_malformed_range_on_an_absent_optional_peer_is_silent() {
    let build = build([versioned(
        "@t/bad",
        "1.0.0",
        vec![optional_peer("@t/base", "one-ish")],
    )]);
    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "an extension with an unsatisfied peer still registers its kinds"
)]
fn an_extension_with_an_unsatisfied_peer_still_registers_its_kinds() {
    let mut product = declare("@specforge/product", |c| {
        c.kind("Feature", |k| {
            k.keyword("feature");
        });
    });
    product
        .handshake
        .peer_dependencies
        .push(peer("@specforge/software", ">=1.0.0"));
    let build = build([product]);

    assert_eq!(codes(&build), ["E027"]);
    let feature = build.kinds.get("feature").expect("registered");
    assert_eq!(feature.source_extension, "@specforge/product");
}

#[spec(
    behavior = "registry_build_peer_dependencies",
    verify = "Registry Build Checks Peer Dependencies: peer dependency checking holds — declarations_in_load_order, peers_checked, unsatisfied_failed, malformed_warned"
)]
fn peer_dependency_checking_holds() {
    // declarations_in_load_order: software, then an extension with a peer
    // in range, one out of range, one missing and one malformed, then an
    // unnamed declaration (E030) and a cycle of passes (W145) to place the
    // peers' diagnostics among the declaration diagnostics.
    let mut checked = extension("@test/checked");
    checked.meta.peer_dependencies = vec![
        peer("@specforge/software", "^1.0.0"),
        peer("@specforge/software", "^2.0.0"),
        peer("@test/missing", "^1"),
        peer("@specforge/software", "one-ish"),
    ];
    checked.pass("a", |p| {
        p.after("b").run(no_findings);
    });
    checked.pass("b", |p| {
        p.after("a").run(no_findings);
    });
    let unnamed = declare("", |_| {});
    let build = build([software(), checked.declaration(), unnamed]);

    // peers_checked + unsatisfied_failed + malformed_warned, in peer order,
    // after E030 and before W145.
    let declared: Vec<&str> = build
        .declaration_diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(declared, ["E030", "E027", "E027", "W062", "W145"]);
    let e027 = coded(&build, "E027");
    assert!(e027[0].message.contains("^2.0.0") && e027[0].message.contains("1.0.0"));
    assert!(e027[1].message.contains("'@test/missing'"));
    assert!(e027.iter().all(|d| d.severity == Severity::Error));
    assert!(coded(&build, "W062")[0].message.contains("'one-ish'"));
    assert!(build.registry_diagnostics.is_empty());
    assert!(build.surface_diagnostics.is_empty());
}

/// Governance works without software (only ConstrainsBehavior targets a
/// software kind), so its peer on software is optional; with the peer
/// check on compile, a required one would fail governance-only projects.
#[spec(
    behavior = "ge_declare_manifest",
    verify = "peer_dependencies includes optional @specforge/software"
)]
fn governance_peer_on_software_is_optional() {
    let handshake = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../specforge-component/tests/declarations/governance/handshake.json");
    let handshake: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(handshake).unwrap()).unwrap();
    let software = handshake["peer_dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "@specforge/software")
        .expect("governance declares a peer on software");
    assert_eq!(software["version"], "^1.0");
    assert_eq!(software["optional"], true);
}
