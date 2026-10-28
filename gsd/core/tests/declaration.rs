//! What `@gsd/core` declares, pinned through the SDK's runtime-free `MockHost`.

use gsd_core::build;
use specforge_extension_sdk::testing::MockHost;

#[test]
fn the_handshake_names_gsd_core_and_tracks_1_18_0() {
    let host = MockHost::new(build());
    let handshake: serde_json::Value =
        serde_json::from_str(&host.handshake_json()).expect("the handshake is JSON");
    assert_eq!(handshake["name"], "@gsd/core");
    assert_eq!(handshake["ext_short"], "gsd");
    assert_eq!(handshake["version"], "0.1.0");
    assert_eq!(
        handshake["keywords"],
        serde_json::json!(["tracks:gsd-core@1.18.0"])
    );
}

#[test]
fn the_tracked_release_is_the_manifests() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains(&format!("tracks = \"{}\"", gsd_core::TRACKS)),
        "Cargo.toml tracks a different release"
    );
}
