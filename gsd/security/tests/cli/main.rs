//! Black-box tests: the `specforge` binary on projects that load `@gsd/core` and `@gsd/security` together (ADR gsd/0002).
//! The harness is core's.

#[allow(
    dead_code,
    reason = "core's harness serves every extension; each test crate uses part of it"
)]
#[path = "../../../core/tests/cli/harness.rs"]
mod harness;

mod model;
mod settings;

use specforge_extension_sdk::testing::MockHost;

/// A project that loads both extensions: core's `empty` project plus `@gsd/security` from `GSD_SECURITY_WASM`.
fn both() -> harness::Project {
    harness::project("empty").with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

#[test]
fn both_extensions_load() {
    let p = both();
    let listed = p.run(&["extensions"]);
    assert!(
        listed.all().contains("@gsd/core") && listed.all().contains("@gsd/security"),
        "extensions: {}",
        listed.all()
    );
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    for code in ["E027", "E026", "W021"] {
        assert!(!check.all().contains(code), "{code}: {}", check.all());
    }
}

#[test]
fn the_handshake_tracks_gsd_core_1_18_0() {
    let host = MockHost::new(gsd_security::build());
    let handshake: serde_json::Value =
        serde_json::from_str(&host.handshake_json()).expect("the handshake is JSON");
    assert_eq!(handshake["name"], "@gsd/security");
    assert_eq!(handshake["ext_short"], "gsd");
    assert_eq!(handshake["version"], "1.0.0");
    assert_eq!(
        handshake["keywords"],
        serde_json::json!(["tracks:gsd-core@1.18.0"])
    );
    let peer = &handshake["peer_dependencies"][0];
    assert_eq!(peer["name"], "@gsd/core");
    // shortcut: `>=1.0.0` until plan 16 T8 releases core 1.1.0 and sets `^1.1.0`
    assert_eq!(peer["version"], ">=1.0.0");
}
