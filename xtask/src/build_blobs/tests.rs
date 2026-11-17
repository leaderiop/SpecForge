use super::*;

#[test]
fn dep_info_splits_on_unescaped_spaces_only() {
    let text = "/r/out.wasm: /r/a.rs /r/dir\\ with\\ space/b.rs /r/c.json\n";
    assert_eq!(
        parse_dep_info(text),
        vec![
            PathBuf::from("/r/a.rs"),
            PathBuf::from("/r/dir with space/b.rs"),
            PathBuf::from("/r/c.json"),
        ]
    );
}

#[test]
fn dev_dependency_tables_are_stripped_and_nothing_else() {
    let manifest = "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n\n\
         [dev-dependencies]\ninsta = \"1\"\n\n[dev-dependencies.foo]\npath = \"../foo\"\n\n\
         [target.'cfg(unix)'.dev-dependencies]\nlibc = \"0\"\n\n[features]\ndefault = []\n";
    assert_eq!(
        staged_manifest(manifest),
        "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n\n[features]\ndefault = []\n"
    );
    let plain = "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n";
    assert_eq!(staged_manifest(plain), plain);
}

#[test]
fn a_stage_lock_must_be_a_subset_of_the_extensions() {
    let extension = "[[package]]\nname = \"a\"\nversion = \"1.0.0\"\nsource = \"registry+x\"\n\
         checksum = \"c\"\ndependencies = [\"b\"]\n\n[[package]]\nname = \"b\"\nversion = \"2.0.0\"\n";
    let pruned = "[[package]]\nname = \"b\"\nversion = \"2.0.0\"\n";
    assert_eq!(lock_subset(pruned, extension), Ok(()));
    let moved = "[[package]]\nname = \"b\"\nversion = \"2.1.0\"\n";
    assert_eq!(lock_subset(moved, extension), Err("b 2.1.0".to_string()));
}

/// `--verify` fails, never skips, on a blob another rustc built, and on a pinned rustc that is not
/// installed (ADR 0373 D1).
#[test]
fn a_blob_another_rustc_built_fails_naming_the_pin() {
    let pinned = "rustc 1.99.1 (58cf8f9c3 2026-10-13)";
    assert_eq!(blob_inputs::BLOB_RUSTC, "1.99.1");
    assert_eq!(rustc_verdict(pinned, pinned), None);

    let old = "rustc 1.98.1 (48a229cea 2026-09-01)";
    let message = rustc_verdict(old, pinned).expect("another rustc fails");
    for named in [old, pinned, "BLOB_RUSTC", "rustup toolchain install 1.99.1"] {
        assert!(message.contains(named), "{named} not in: {message}");
    }

    let missing = "rustc 1.100.0 (0123456789 2026-12-01)";
    let message = rustc_verdict(missing, missing).expect("another pin fails");
    assert!(
        message.contains("BLOB_RUSTC") && message.contains("rustup toolchain install 1.99.1"),
        "{message}"
    );
    // A version that only shares the pin's prefix is another rustc.
    assert!(rustc_verdict("rustc 1.99.10 (x)", "rustc 1.99.10 (x)").is_some());
}

/// The stager's and `revendor`'s cargo and rustc run on the pin: `RUSTUP_TOOLCHAIN` names it and
/// no `RUSTC` override reaches them (ADR 0373 D1).
#[test]
fn the_blob_commands_run_on_the_pinned_rustc() {
    let mut command = Command::new("cargo");
    blob_inputs::on_blob_rustc(&mut command);
    let envs: BTreeMap<String, Option<String>> = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    assert_eq!(
        envs.get("RUSTUP_TOOLCHAIN"),
        Some(&Some(blob_inputs::BLOB_RUSTC.to_string()))
    );
    assert_eq!(envs.get("RUSTC"), Some(&None), "RUSTC is removed");
    assert_eq!(
        envs.get("CARGO_BUILD_RUSTC"),
        Some(&None),
        "CARGO_BUILD_RUSTC is removed"
    );
}

#[test]
fn a_stage_mirrors_the_repo_layout() {
    let stage = Path::new("/r/target/blob-stage/extensions/product");
    assert_eq!(
        stage_key(stage, &stage.join("crates/x/src/lib.rs")),
        Some("crates/x/src/lib.rs".to_string())
    );
    assert_eq!(stage_key(stage, Path::new("/elsewhere/lib.rs")), None);
}

/// The committed blobs that are builtins. The fixtures record their inputs at the round's re-vendor; the
/// stage builds them like the builtins, but these two lists are the builtins' (ADR 0039, 0050).
fn builtins() -> impl Iterator<Item = &'static CommittedBlob> {
    COMMITTED
        .iter()
        .filter(|blob| blob.crate_dir.starts_with("extensions/"))
}

/// The crates a builtin blob compiles besides its own sources: the guest wire, the SDK and its
/// macros, the coverage rule, the cycle answer and the checks kit `@specforge/checks` links (ADR 0039, 0050, 0196,
/// 0338), and any extension library it links (ADR 0328 D3). A host-only module in one of them, or a new path dependency of a guest, shows here as an input outside
/// this list.
const GUEST_CRATES: &[&str] = &[
    "crates/leaf/protocol-types/",
    "crates/runtime/extension-sdk/",
    "crates/runtime/extension-sdk-macros/",
    "crates/leaf/coverage/",
    "crates/leaf/cycles/",
    "crates/leaf/checks-kit/",
];

/// Whether `key` (a repo-relative input) lies in an extension library: a crate
/// `extensions/<family>/<dir>` with no `wasm/`, which no blob is built from and builtins link.
fn in_extension_library(root: &Path, key: &str) -> bool {
    let mut segments = key.split('/');
    let (Some("extensions"), Some(family), Some(dir), Some(_)) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return false;
    };
    let library = root.join("extensions").join(family).join(dir);
    library.join("Cargo.toml").is_file() && !library.join("wasm").exists()
}

#[test]
fn an_input_under_an_extension_library_is_a_guest_crate_input() {
    /// A directory removed when dropped, so a failing assert leaves nothing behind (xtask has no `tempfile`).
    struct TempDir(PathBuf);
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let root = TempDir(std::env::temp_dir().join(format!("xtask-library-{}", std::process::id())));
    for dir in ["extensions/code/junit", "extensions/code/core/wasm"] {
        std::fs::create_dir_all(root.0.join(dir)).unwrap();
    }
    std::fs::write(root.0.join("extensions/code/junit/Cargo.toml"), "").unwrap();
    std::fs::write(root.0.join("extensions/code/core/Cargo.toml"), "").unwrap();
    let found = [
        "extensions/code/junit/src/lib.rs",
        "extensions/code/core/src/lib.rs",
        "extensions/code/junit",
        "extensions/product/src/lib.rs",
    ]
    .map(|key| in_extension_library(&root.0, key));
    assert_eq!(found, [true, false, false, false]);
}

#[test]
fn a_builtin_compiles_only_its_own_sources_and_the_guest_crates() {
    let root = specforge_repo_root::repo_root();
    for builtin in builtins() {
        let name = builtin.crate_dir;
        let record = BlobRecord::read(root, name).unwrap();
        let own = format!("{name}/");
        let inputs = &record.inputs;
        for key in inputs.keys() {
            assert!(
                key == WORKSPACE_INHERITED
                    || key.starts_with(&own)
                    || GUEST_CRATES.iter().any(|guest| key.starts_with(guest))
                    || in_extension_library(root, key),
                "{name} compiles {key}, outside its own sources, the guest crates and the extension libraries"
            );
        }
        assert!(
            !inputs
                .keys()
                .any(|k| k.ends_with("/peers.rs") || k.ends_with("/package.rs")),
            "{name}: the package and peer rules are host-side"
        );
    }
}

/// A blob compiles no test code: a test module is declared out of line (`#[cfg(test)] mod
/// tests;`), which a release build never reads, so a test-only edit re-vendors nothing; and
/// the SDK's test helpers are behind its `testing` feature (ADR 0050).
#[test]
fn a_builtin_compiles_no_test_code() {
    let root = specforge_repo_root::repo_root();
    for builtin in builtins() {
        let name = builtin.crate_dir;
        let record = BlobRecord::read(root, name).unwrap();
        for key in record.inputs.keys() {
            assert_ne!(
                key, "crates/runtime/extension-sdk/src/testing.rs",
                "{name} compiles the SDK's test helpers"
            );
            let Ok(text) = std::fs::read_to_string(root.join(key)) else {
                continue;
            };
            let lines: Vec<&str> = text.lines().collect();
            for pair in lines.windows(2) {
                let inline_module = pair[1]
                    .trim_start()
                    .strip_prefix("pub ")
                    .map_or_else(|| pair[1].trim_start(), str::trim_start);
                assert!(
                    !(pair[0].trim() == "#[cfg(test)]"
                        && inline_module.starts_with("mod ")
                        && inline_module.trim_end().ends_with('{')),
                    "{name}: {key} holds an inline test module `{}`; declare it out of line",
                    pair[1].trim()
                );
            }
        }
    }
}

#[test]
fn a_staged_member_carries_no_workspace_or_profile_table() {
    let manifest = "[package]\nname = \"x\"\n\n[workspace]\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n\
         [dependencies]\nserde = \"1\"\n\n[profile.release]\nopt-level = \"z\"\n\n\
         [workspace.lints.rust]\nx = \"deny\"\n";
    assert_eq!(
        staged_manifest(manifest),
        "[package]\nname = \"x\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nserde = \"1\"\n\n"
    );
}

/// The stage of a gsd crate builds against the SDK the gsd workspace patches in, read from the
/// repository's own `gsd/.cargo/config.toml` and re-rooted at the repository (ADR 0366 D2).
#[test]
fn the_gsd_stage_replays_the_sdk_patch_from_the_root() {
    let root = specforge_repo_root::repo_root();
    let config = blob_inputs::cargo_config_of("gsd/core").expect("a gsd crate reads a config");
    let table = patch::stage_patch(&root.join(config), blob_inputs::cargo_dir("gsd/core")).unwrap();
    assert_eq!(
        table,
        "[patch.crates-io]\nspecforge-extension-sdk = { path = \"crates/runtime/extension-sdk\" }\n"
    );
    assert!(blob_inputs::cargo_config_of("extensions/product").is_none());
}

/// A config file written to a fresh temporary directory, for the patch reader's refusals.
fn patch_of(config: &str) -> Result<String, String> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, config).unwrap();
    patch::stage_patch(&path, "gsd")
}

#[test]
fn a_patch_path_that_leaves_the_repository_is_refused() {
    let err =
        patch_of("[patch.crates-io]\nforeign = { path = \"../../elsewhere/sdk\" }\n").unwrap_err();
    assert!(
        err.contains("foreign") && err.contains("leaves the repository"),
        "{err}"
    );
    let err = patch_of("[patch.crates-io]\nrooted = { path = \"/opt/sdk\" }\n").unwrap_err();
    assert!(
        err.contains("rooted") && err.contains("leaves the repository"),
        "{err}"
    );
}

#[test]
fn a_patch_entry_that_is_not_a_path_is_refused() {
    let err = patch_of(
        "[patch.crates-io]\nfrom-git = { git = \"https://example.com/sdk\", branch = \"main\" }\n",
    )
    .unwrap_err();
    assert!(
        err.contains("from-git") && err.contains("not a lone `path`"),
        "{err}"
    );
    let err = patch_of("[patch.crates-io]\npinned = { path = \"../crates/x\", version = \"1\" }\n")
        .unwrap_err();
    assert!(
        err.contains("pinned") && err.contains("not a lone `path`"),
        "{err}"
    );
    assert_eq!(patch_of("[build]\njobs = 2\n"), Ok(String::new()));
}
