use super::*;
use specforge_repo_root::repo_root;

/// The builtins a change feeds; the fixtures that record their inputs are left out.
fn fed(paths: &[&str]) -> Vec<String> {
    let mut fed = fed_by(repo_root(), &ChangeSet::of(paths.iter().copied())).unwrap();
    fed.retain(|crate_dir| crate_dir.starts_with("extensions/"));
    fed
}

/// The blobs that record their inputs today: the builtins. The fixtures join at the round's re-vendor.
const BUILTIN_CRATES: [&str; 14] = [
    "extensions/cargo-test",
    "extensions/checks",
    "extensions/code/core",
    "extensions/formal",
    "extensions/governance",
    "extensions/javascript/node-test",
    "extensions/javascript/source",
    "extensions/product",
    "extensions/rust/cargo",
    "extensions/rust/source",
    "extensions/rust/test",
    "extensions/software",
    "extensions/testing",
    "extensions/vitest",
];

/// The builtins that link the `@code` extension libraries.
const CODE_LINKERS: [&str; 6] = [
    "extensions/code/core",
    "extensions/javascript/node-test",
    "extensions/javascript/source",
    "extensions/rust/cargo",
    "extensions/rust/source",
    "extensions/rust/test",
];

#[test]
fn a_record_round_trips_byte_for_byte() {
    let scratch = tempfile::tempdir().unwrap();
    for builtin in blobs(repo_root()).unwrap() {
        let record = BlobRecord::read(repo_root(), &builtin).unwrap();
        record.write(scratch.path(), &builtin).unwrap();
        let original = std::fs::read(repo_root().join(shown(&builtin))).unwrap();
        let written = std::fs::read(scratch.path().join(shown(&builtin))).unwrap();
        assert_eq!(original, written, "{builtin}");
    }
}

#[test]
fn a_change_to_the_junit_library_feeds_the_builtins_that_link_it() {
    assert_eq!(fed(&["extensions/code/junit/src/lib.rs"]), CODE_LINKERS);
}

#[test]
fn a_change_to_the_code_facts_library_feeds_the_builtins_that_link_it() {
    assert_eq!(fed(&["extensions/code/facts/src/ids.rs"]), CODE_LINKERS);
}

#[test]
fn a_change_to_the_manifest_library_feeds_only_rust_cargo() {
    assert_eq!(
        fed(&["extensions/rust/manifest/src/lib.rs"]),
        ["extensions/rust/cargo"]
    );
}

#[test]
fn a_change_to_the_workspace_library_feeds_both_javascript_builtins() {
    assert_eq!(
        fed(&["extensions/javascript/workspace/src/lib.rs"]),
        [
            "extensions/javascript/node-test",
            "extensions/javascript/source"
        ]
    );
}

#[test]
fn a_change_to_coverage_feeds_the_blobs_that_record_it() {
    assert_eq!(
        fed(&["crates/leaf/coverage/src/lib.rs"]),
        [
            "extensions/formal",
            "extensions/product",
            "extensions/testing"
        ]
    );
}

#[test]
fn protocol_types_and_the_root_manifest_feed_every_builtin() {
    assert_eq!(
        fed(&["crates/leaf/protocol-types/src/lib.rs"]),
        BUILTIN_CRATES
    );
    assert_eq!(fed(&["Cargo.toml"]), BUILTIN_CRATES);
    assert_eq!(
        fed(&["crates/runtime/extension-sdk/src/lib.rs"]),
        BUILTIN_CRATES
    );
}

#[test]
fn a_test_only_sdk_file_feeds_no_blob() {
    assert!(fed(&["crates/runtime/extension-sdk/src/testing.rs"]).is_empty());
    assert!(fed(&["crates/surfaces/lsp/src/lib.rs"]).is_empty());
}

#[test]
fn a_change_to_the_checks_kit_feeds_the_checks_blob() {
    let fed_blobs = fed(&["crates/leaf/checks-kit/src/digits.rs"]);
    assert!(
        fed_blobs.iter().any(|blob| blob == "extensions/checks"),
        "{fed_blobs:?}"
    );
}

#[test]
fn blobs_lists_the_recorded_blobs() {
    let listed = blobs(repo_root()).unwrap();
    assert!(
        BUILTIN_CRATES
            .iter()
            .all(|builtin| listed.iter().any(|l| l == builtin))
    );
    assert!(listed.iter().all(|l| l.starts_with("extensions/")
        || l.starts_with("fixtures/")
        || l.starts_with("examples/")));
}

#[test]
fn inherited_tables_ignore_members_and_profiles() {
    let a = "[workspace]\nmembers = [\"x\"]\n[workspace.package]\nedition = \"2024\"\n\
             [workspace.dependencies]\nserde = \"1\"\n[profile.release]\nlto = true\n";
    let b = "[workspace]\nmembers = [\"x\", \"y\"]\n[workspace.package]\nedition = \"2024\"\n\
             [workspace.dependencies]\nserde = \"1\"\n[profile.release]\nlto = false\n";
    assert_eq!(inherited_tables(a), inherited_tables(b));

    let c = a.replace(
        "serde = \"1\"",
        "serde = { version = \"1\", features = [\"rc\"] }",
    );
    assert_ne!(inherited_tables(a), inherited_tables(&c));
}

#[test]
fn inherited_tables_keep_the_blob_profile() {
    let a = "[workspace]\nmembers = []\n[profile.blob]\nopt-level = \"z\"\n[profile.release]\nlto = true\n";
    let b = a.replace("\"z\"", "\"s\"");
    assert_ne!(inherited_tables(a), inherited_tables(&b));
    let c = a.replace("lto = true", "lto = false");
    assert_eq!(inherited_tables(a), inherited_tables(&c));
}

#[test]
fn inherited_tables_keep_the_lint_tables() {
    let a = "[workspace]\nmembers = []\n[workspace.lints.rust]\nunsafe_code = \"deny\"\n\
             [workspace.lints.clippy]\nunwrap_used = \"deny\"\n[profile.dev]\ndebug = 1\n";
    let kept = inherited_tables(a);
    assert!(kept.contains("unsafe_code = \"deny\""));
    assert!(kept.contains("unwrap_used = \"deny\""));
    assert!(!kept.contains("debug = 1"));
    assert_ne!(
        kept,
        inherited_tables(&a.replace("unwrap_used = \"deny\"", "unwrap_used = \"warn\""))
    );
}

#[test]
fn a_key_names_the_repo_path_it_hashes() {
    assert_eq!(path_of(WORKSPACE_INHERITED), "Cargo.toml");
    assert_eq!(path_of("crates/x/src/lib.rs"), "crates/x/src/lib.rs");
}

#[test]
fn a_change_to_an_examples_extension_feeds_its_blob() {
    assert_eq!(
        fed_by(
            repo_root(),
            &ChangeSet::of(["examples/marketing/extension/src/lib.rs"])
        )
        .unwrap(),
        ["examples/marketing/extension"]
    );
}
