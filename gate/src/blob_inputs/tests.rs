use super::*;
use specforge_repo_root::repo_root;

/// The builtins a change feeds; the fixtures that record their inputs are left out.
fn fed(paths: &[&str]) -> Vec<String> {
    let mut fed = fed_by(repo_root(), &ChangeSet::of(paths.iter().copied())).unwrap();
    fed.retain(|crate_dir| crate_dir.starts_with("extensions/"));
    fed
}

/// The blobs that record their inputs today: the builtins. The fixtures join at the round's re-vendor.
const BUILTIN_CRATES: [&str; 26] = [
    "extensions/cargo-test",
    "extensions/checks",
    "extensions/code/core",
    "extensions/formal",
    "extensions/governance",
    "extensions/javascript/api",
    "extensions/javascript/coverage",
    "extensions/javascript/deps",
    "extensions/javascript/jest",
    "extensions/javascript/lint",
    "extensions/javascript/node-test",
    "extensions/javascript/playwright",
    "extensions/javascript/source",
    "extensions/javascript/vitest",
    "extensions/product",
    "extensions/rust/api",
    "extensions/rust/cargo",
    "extensions/rust/coverage",
    "extensions/rust/deps",
    "extensions/rust/lint",
    "extensions/rust/proof",
    "extensions/rust/source",
    "extensions/rust/test",
    "extensions/software",
    "extensions/testing",
    "extensions/vitest",
];

/// The builtins that link code-facts. code-facts links code-junit, so these are junit's linkers too.
const CODE_FACTS_LINKERS: [&str; 18] = [
    "extensions/code/core",
    "extensions/javascript/api",
    "extensions/javascript/coverage",
    "extensions/javascript/deps",
    "extensions/javascript/jest",
    "extensions/javascript/lint",
    "extensions/javascript/node-test",
    "extensions/javascript/playwright",
    "extensions/javascript/source",
    "extensions/javascript/vitest",
    "extensions/rust/api",
    "extensions/rust/cargo",
    "extensions/rust/coverage",
    "extensions/rust/deps",
    "extensions/rust/lint",
    "extensions/rust/proof",
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
    assert_eq!(
        fed(&["extensions/code/junit/src/lib.rs"]),
        CODE_FACTS_LINKERS
    );
}

#[test]
fn a_change_to_the_code_facts_library_feeds_the_builtins_that_link_it() {
    assert_eq!(
        fed(&["extensions/code/facts/src/ids.rs"]),
        CODE_FACTS_LINKERS
    );
}

/// A report reader is a library each report pack links, so editing one re-vendors exactly the packs that
/// read its format (ADR 0389).
#[test]
fn a_change_to_a_report_reader_feeds_the_packs_that_link_it() {
    let readers: [(&str, &[&str]); 5] = [
        (
            "lcov",
            &["extensions/javascript/coverage", "extensions/rust/coverage"],
        ),
        ("api", &["extensions/javascript/api", "extensions/rust/api"]),
        ("sarif", &["extensions/javascript/lint"]),
        (
            "cyclonedx",
            &["extensions/javascript/deps", "extensions/rust/deps"],
        ),
        (
            "osv",
            &["extensions/javascript/deps", "extensions/rust/deps"],
        ),
    ];
    for (reader, packs) in readers {
        let lib = format!("extensions/code/{reader}/src/lib.rs");
        assert_eq!(fed(&[lib.as_str()]), packs, "{lib}");
    }
}

/// The Kani reader is linked by `@rust/proof` alone (ADR 0389).
#[test]
fn a_change_to_the_kani_reader_feeds_only_rust_proof() {
    assert_eq!(
        fed(&["extensions/code/kani/src/lib.rs"]),
        ["extensions/rust/proof"]
    );
}

#[test]
fn a_change_to_the_manifest_library_feeds_only_rust_cargo() {
    assert_eq!(
        fed(&["extensions/rust/manifest/src/lib.rs"]),
        ["extensions/rust/cargo"]
    );
}

#[test]
fn a_change_to_the_workspace_library_feeds_the_javascript_builtins_that_link_it() {
    assert_eq!(
        fed(&["extensions/javascript/workspace/src/lib.rs"]),
        [
            "extensions/javascript/jest",
            "extensions/javascript/node-test",
            "extensions/javascript/playwright",
            "extensions/javascript/source",
            "extensions/javascript/vitest",
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
        || l.starts_with("examples/")
        || l == "gsd/core"
        || l == "gsd/import"));
}

/// A crate of the gsd workspace builds as that workspace does: its lock, its cargo directory and its SDK patch; any
/// other crate with its own lock, at the root (ADR 0366 D2).
#[test]
fn a_gsd_crate_builds_with_the_gsd_lock_and_config() {
    assert_eq!(lock_of("gsd/core"), "gsd/Cargo.lock");
    assert_eq!(cargo_dir("gsd/core"), "gsd");
    assert_eq!(
        workspace_inputs("gsd/core"),
        ["gsd/Cargo.lock", "gsd/.cargo/config.toml"]
    );
    assert_eq!(
        lock_of("extensions/product"),
        "extensions/product/Cargo.lock"
    );
    assert_eq!(cargo_dir("extensions/product"), ".");
    assert_eq!(
        workspace_inputs("extensions/product"),
        ["extensions/product/Cargo.lock"]
    );
    // A directory that only begins with the workspace's name is not in it.
    assert_eq!(lock_of("gsdx/core"), "gsdx/core/Cargo.lock");
    for key in workspace_inputs("gsd/core") {
        assert!(repo_root().join(&key).is_file(), "{key} is missing");
    }
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

/// Every committed blob was built on the pinned rustc, so a pin moved without a re-vendor fails here (ADR 0373 D1).
#[test]
fn every_record_was_built_with_the_pinned_rustc() {
    let pinned = format!("rustc {BLOB_RUSTC} ");
    let listed = blobs(repo_root()).unwrap();
    assert!(!listed.is_empty());
    for crate_dir in listed {
        let record = BlobRecord::read(repo_root(), &crate_dir).unwrap();
        assert!(
            record.built_with.starts_with(&pinned),
            "{crate_dir} was built with {}, not the pinned rustc {BLOB_RUSTC}: re-vendor it on the pin",
            record.built_with
        );
    }
}

/// The bootstrap job installs the pinned rustc itself, since a rustup that does not install a missing toolchain
/// implicitly would otherwise stop the job (ADR 0373 D1).
#[test]
fn the_bootstrap_job_installs_the_pinned_rustc() {
    let path = repo_root().join(".github/workflows/bootstrap.yml");
    let workflow = std::fs::read_to_string(&path).unwrap();
    let install = format!("rustup toolchain install {BLOB_RUSTC} ");
    let line = workflow
        .lines()
        .find(|line| line.contains(&install))
        .unwrap_or_else(|| panic!("{} does not run `{install}`", path.display()));
    assert!(line.contains("wasm32-wasip2"), "{line}");
}

/// The re-vendoring guide installs the pinned rustc with the command the tools print, so a pin bump that forgets the
/// doc fails here (ADR 0373 D1).
#[test]
fn the_revendor_guide_installs_the_pinned_rustc() {
    let path = repo_root().join("docs/agents/revendor.md");
    let guide = std::fs::read_to_string(&path).unwrap();
    let install = blob_rustc_install();
    assert!(
        guide.lines().any(|line| line.trim() == install),
        "{} does not show `{install}` on a line of its own",
        path.display()
    );
}

/// The bootstrap job is bounded, as every `ci.yml` job is: it builds cold with no cache, so a hung rebuild or a stalled
/// toolchain install would otherwise hold a runner for the platform's six-hour default.
#[test]
fn the_bootstrap_job_has_a_timeout() {
    let path = repo_root().join(".github/workflows/bootstrap.yml");
    let workflow = std::fs::read_to_string(&path).unwrap();
    let minutes: u32 = workflow
        .lines()
        .skip_while(|line| line.trim() != "bootstrap:")
        .take_while(|line| line.trim() != "steps:")
        .find_map(|line| line.trim().strip_prefix("timeout-minutes:"))
        .unwrap_or_else(|| panic!("{}: job `bootstrap` has no timeout-minutes", path.display()))
        .trim()
        .parse()
        .unwrap();
    assert!((1..=60).contains(&minutes), "{minutes}");
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

/// The patterns of the bootstrap workflow's `pull_request` `paths:` list, read line by line (no YAML dependency).
fn bootstrap_paths(workflow: &str) -> Vec<String> {
    let mut lines = workflow
        .lines()
        .skip_while(|line| line.trim() != "pull_request:");
    assert!(
        lines.next().is_some(),
        "the bootstrap workflow has no pull_request trigger"
    );
    lines
        .skip_while(|line| line.trim() != "paths:")
        .skip(1)
        .map_while(|line| line.trim().strip_prefix("- "))
        .map(|pattern| pattern.trim_matches('"').to_owned())
        .collect()
}

/// A pattern ending `/**` matches a path under that directory; any other pattern matches itself.
fn covers(pattern: &str, path: &str) -> bool {
    match pattern.strip_suffix("/**") {
        Some(dir) => path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/')),
        None => pattern == path,
    }
}

/// A change to anything a committed blob compiles from runs the cold-clone job on a pull request: every input key of
/// every record is under one of the job's `paths:` (ADR 0366 D2 and D4, ADR 0373 D2).
#[test]
fn the_bootstrap_paths_cover_every_recorded_input() {
    let path = repo_root().join(".github/workflows/bootstrap.yml");
    let workflow = std::fs::read_to_string(&path).unwrap();
    let patterns = bootstrap_paths(&workflow);
    assert!(!patterns.is_empty(), "{} lists no paths", path.display());
    let mut uncovered = Vec::new();
    for crate_dir in blobs(repo_root()).unwrap() {
        let record = BlobRecord::read(repo_root(), &crate_dir).unwrap();
        for key in record.inputs.keys() {
            let input = path_of(key);
            if !patterns.iter().any(|pattern| covers(pattern, input)) {
                uncovered.push(format!("{crate_dir}: {key}"));
            }
        }
    }
    assert!(
        uncovered.is_empty(),
        "{} does not run on a change to these recorded inputs; add their paths:\n  {}",
        path.display(),
        uncovered.join("\n  ")
    );
}

/// The job runs the equality test the ignored way: it is `#[ignore]`d so the gate's `tests` check does not build the
/// blob (ADR 0373 D2).
#[test]
fn the_bootstrap_job_runs_the_gsd_core_equality_test() {
    let path = repo_root().join(".github/workflows/bootstrap.yml");
    let workflow = std::fs::read_to_string(&path).unwrap();
    let line = workflow
        .lines()
        .find(|line| line.contains("committed_gsd_core_blob_is_the_stagers_build"))
        .unwrap_or_else(|| panic!("{} does not run the gsd/core equality test", path.display()));
    assert!(
        line.contains("cargo nextest run") && line.contains("--run-ignored only"),
        "{line}"
    );
}
