//! The order the registry build reads its declarations in.

use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::DeclaredPass;

use crate::support::{build, coded, codes, peer};

/// A pass's handler that finds nothing.
fn no_findings(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

/// `name`, declaring the kind `Widget` (keyword `widget`) and a pass named
/// after its last segment, with `peers`.
fn widget(name: &str, peers: Vec<PeerDependency>) -> ExtensionDeclaration {
    let pass = name.rsplit('/').next().unwrap().to_string();
    let mut c = crate::support::extension(name);
    c.meta.peer_dependencies = peers;
    c.kind("Widget", |k| {
        k.keyword("widget");
    });
    c.pass(&pass, |p| {
        p.run(no_findings);
    });
    c.declaration()
}

/// `name`, with one required peer `peer_name`.
fn cyclic(name: &str, peer_name: &str) -> ExtensionDeclaration {
    let mut c = crate::support::extension(name);
    c.meta.peer_dependencies = vec![peer(peer_name, "^1.0")];
    c.kind(if name.ends_with('a') { "Alpha" } else { "Beta" }, |k| {
        k.keyword(if name.ends_with('a') { "alpha" } else { "beta" });
    });
    c.declaration()
}

/// Pinned until T4 (ADR 0041): the build reads the declarations in the order
/// it is given them.
#[test]
fn pin_the_build_keeps_entry_order() {
    let build = build([
        widget("@t/dep", vec![peer("@t/base", "^1.0")]),
        widget("@t/base", vec![]),
    ]);
    let names: Vec<&str> = build.declarations().iter().map(|d| d.name()).collect();
    assert_eq!(names, ["@t/dep", "@t/base"]);
    assert_eq!(
        build.kinds.get("widget").unwrap().source_extension,
        "@t/dep"
    );
    let e026 = coded(&build, "E026");
    assert_eq!(e026.len(), 1);
    assert_eq!(
        e026[0].message,
        "entity kind 'widget' registered by '@t/base' conflicts with '@t/dep' (first registration wins)"
    );
    let passes: Vec<String> = build.passes.iter().map(DeclaredPass::full_name).collect();
    assert_eq!(passes, ["@t/dep:dep", "@t/base:base"]);
}

/// Pinned until T5: no compile reports a cycle among required peers.
#[test]
fn pin_a_required_peer_cycle_is_not_reported() {
    let build = build([cyclic("@t/cyca", "@t/cycb"), cyclic("@t/cycb", "@t/cyca")]);
    assert!(codes(&build).is_empty(), "{:?}", codes(&build));
}
