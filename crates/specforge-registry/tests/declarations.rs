//! The registry build owns what is checked of the declarations themselves:
//! identity and shape (E030), peers (E027) and the order of the declared
//! passes (W145), from `build_registries` alone, with no runtime.

use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::{ExtensionDeclaration, PeerDependency};
use specforge_registry::build_registries;

fn declaration(name: &str, version: &str) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, version));
    c.kind("report", |k| {
        k.description("A report");
    });
    c
}

fn codes(diagnostics: &[specforge_common::Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(|d| d.code.as_str()).collect()
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a declaration with an empty name or version produces E030"
)]
fn an_empty_name_or_version_is_e030() {
    let build = build_registries(vec![
        declaration("", "1.0.0").declaration(),
        declaration("@acme/b", "").declaration(),
        declaration("@acme/c", "1.0.0").declaration(),
    ]);
    assert_eq!(codes(&build.declaration_diagnostics), ["E030", "E030"]);
    assert!(
        build.declaration_diagnostics[0]
            .message
            .contains("name is empty")
    );
    assert!(
        build.declaration_diagnostics[1]
            .message
            .contains("'@acme/b': its version is empty")
    );
    // The declarations still build: the registries hold their kinds.
    assert!(build.kinds.contains("report"));
    assert!(build.registry_diagnostics.iter().all(|d| d.code != "E030"));
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a malformed ext_short produces E030"
)]
fn a_malformed_short_name_is_e030() {
    let mut bad = declaration("@acme/reports", "1.0.0").declaration();
    // A raw handshake can carry any short; the SDK refuses one at build.
    bad.handshake.ext_short = Some("Reports Tool".to_string());
    let mut good = declaration("@acme/tools", "1.0.0").declaration();
    good.entities.clear();
    good.handshake.ext_short = Some("my-tools2".to_string());
    let build = build_registries(vec![bad, good]);
    assert_eq!(codes(&build.declaration_diagnostics), ["E030"]);
    let message = &build.declaration_diagnostics[0].message;
    assert!(
        message.contains("ext_short 'Reports Tool' is not lowercase kebab case"),
        "{message}"
    );
    assert_eq!(
        build.declaration("@acme/tools").unwrap().short(),
        "my-tools2"
    );
}

fn with_peer(name: &str, peer: &str, range: &str, optional: bool) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    c.meta.peer_dependencies.push(PeerDependency {
        name: peer.to_string(),
        version: range.to_string(),
        optional,
    });
    c.declaration()
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "peer dependencies are checked in the registry build"
)]
fn peers_are_checked_by_the_build() {
    // A required peer that is not loaded, one loaded out of range, and an
    // optional one absent (fine).
    let build = build_registries(vec![
        with_peer("@acme/a", "@acme/missing", "^1", false),
        with_peer("@acme/b", "@acme/a", "^2", false),
        with_peer("@acme/c", "@acme/also-missing", "^1", true),
    ]);
    assert_eq!(codes(&build.declaration_diagnostics), ["E027", "E027"]);
    assert_eq!(
        build.declaration_diagnostics[0].message,
        "extension '@acme/a' requires peer dependency '@acme/missing' ^1 which is not installed"
    );
    assert_eq!(
        build.declaration_diagnostics[1].message,
        "extension '@acme/b' requires peer dependency '@acme/a' ^2 but version 1.0.0 is installed"
    );
}

fn passes(name: &str, declare: impl FnOnce(&mut ContributionsBuilder)) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    declare(&mut c);
    c.declaration()
}

fn order(build: &specforge_registry::RegistryBuild) -> Vec<String> {
    build.passes.iter().map(|p| p.full_name()).collect()
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "declared passes are ordered in the registry build"
)]
fn declared_passes_are_ordered() {
    let build = build_registries(vec![
        passes("@acme/formal", |c| {
            // Declared out of order; `after` gives the run order. `resolve`
            // is a host phase, ignored.
            c.pass("event_graph_analyze", |p| {
                p.after("layering_verify");
            });
            c.pass("coverage_tracking", |p| {
                p.after("event_graph_analyze").phase("check");
            });
            c.pass("condition_check", |p| {
                p.after("resolve");
            });
            c.pass("layering_verify", |p| {
                p.after("condition_check");
            });
        }),
        passes("@acme/other", |c| {
            c.pass("second", |p| {
                p.before("first");
            });
            c.pass("first", |_| {});
            c.pass("solo", |_| {});
        }),
    ]);
    assert_eq!(
        order(&build),
        [
            "@acme/formal:condition_check",
            "@acme/formal:layering_verify",
            "@acme/formal:event_graph_analyze",
            "@acme/formal:coverage_tracking",
            // Ready passes run in declaration order (stable Kahn): `solo`
            // is ready before `first`, which waits for `second`.
            "@acme/other:second",
            "@acme/other:solo",
            "@acme/other:first",
        ]
    );
    let check: Vec<String> = build.check_passes().map(|p| p.full_name()).collect();
    assert_eq!(check, ["@acme/formal:coverage_tracking"]);
    assert_eq!(build.analyze_passes().count(), 6);
    assert!(build.declaration_diagnostics.is_empty());
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a pass constraint cycle produces W145 and keeps declaration order"
)]
fn a_pass_cycle_is_w145_in_declaration_order() {
    let build = build_registries(vec![passes("@acme/loop", |c| {
        c.pass("a", |p| {
            p.after("b");
        });
        c.pass("b", |p| {
            p.after("a");
        });
        c.pass("c", |_| {});
    })]);
    assert_eq!(
        order(&build),
        ["@acme/loop:a", "@acme/loop:b", "@acme/loop:c"]
    );
    assert_eq!(codes(&build.declaration_diagnostics), ["W145"]);
    let message = &build.declaration_diagnostics[0].message;
    assert!(message.contains("'@acme/loop'"), "{message}");
    assert!(message.contains("'a', 'b'"), "{message}");
}

/// The declarations' own diagnostics come in one fixed order: E030, then
/// W021, then E027, then W145, each extension by extension.
#[test]
fn declaration_diagnostics_come_in_a_fixed_order() {
    let mut cyclic = passes("@acme/z", |c| {
        c.pass("a", |p| {
            p.after("b");
        });
        c.pass("b", |p| {
            p.after("a");
        });
    });
    cyclic.handshake.peer_dependencies.push(PeerDependency {
        name: "@acme/nowhere".to_string(),
        version: "^1".to_string(),
        optional: false,
    });
    let mut inconsistent = declaration("@acme/y", "1.0.0");
    inconsistent.kind("note", |k| {
        k.field("about", |f| {
            f.field_type(FieldType::Reference).target_kind("nowhere");
        });
    });
    let unnamed = declaration("", "1.0.0").declaration();
    let build = build_registries(vec![cyclic, inconsistent.declaration(), unnamed]);
    assert_eq!(
        codes(&build.declaration_diagnostics),
        ["E030", "W021", "E027", "W145"]
    );
}
