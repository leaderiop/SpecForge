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
        strip_dev_dependencies(manifest),
        "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n\n[features]\ndefault = []\n"
    );
    let plain = "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n";
    assert_eq!(strip_dev_dependencies(plain), plain);
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

#[test]
fn a_stage_mirrors_the_repo_layout() {
    let stage = Path::new("/r/target/blob-stage/product");
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

#[test]
fn every_stage_name_is_unique() {
    let mut names = BTreeSet::new();
    for blob in COMMITTED {
        assert!(
            names.insert(stage_name(blob)),
            "{}: stage name {} is taken",
            blob.name,
            stage_name(blob)
        );
    }
}

/// The crates a builtin blob compiles besides its own sources: the guest wire, the SDK and its
/// macros, the coverage rule and the cycle answer (ADR 0039, 0050, 0196). A host-only module in one of them, or a new
/// path dependency of a guest, shows here as an input outside this list.
const GUEST_CRATES: &[&str] = &[
    "crates/leaf/protocol-types/",
    "crates/runtime/extension-sdk/",
    "crates/runtime/extension-sdk-macros/",
    "crates/leaf/coverage/",
    "crates/leaf/cycles/",
];

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
                    || GUEST_CRATES.iter().any(|guest| key.starts_with(guest)),
                "{name} compiles {key}, outside its own sources and the guest crates"
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
