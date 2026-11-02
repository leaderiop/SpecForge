//! The builtin table (`specforge_component::builtins::BUILTINS`) is the one list of builtins: it is
//! the `extensions/` directory, by the naming rule, and the (name, bytes) view the extension load
//! reads is its rows.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_component::builtins::{BUILTIN_EXTENSIONS, BUILTINS};
use specforge_component::committed::{COMMITTED, EXAMPLES, FIXTURES};
use specforge_test_macros::test as specforge_test;
use std::path::Path;

/// A builtin crate as the naming rule reads it: the row `BUILTINS` must hold for it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Named {
    crate_dir: String,
    name: String,
    artifact: String,
}

impl Named {
    fn new(crate_dir: &str, name: &str, artifact: &str) -> Self {
        Named {
            crate_dir: crate_dir.to_string(),
            name: name.to_string(),
            artifact: artifact.to_string(),
        }
    }
}

/// What the naming rule (ADR 0328 D3) makes of `<root>/extensions/`.
#[derive(Debug, Default)]
struct Layout {
    /// Each builtin crate, sorted by crate dir.
    builtins: Vec<Named>,
    /// Each place that breaks the rule, with why.
    problems: Vec<String>,
}

/// The sorted subdirectories of `dir` (none when it can't be read).
fn subdirs(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// One crate of the walk: a builtin, or (in a family) an extension library.
fn add_crate(found: &mut Layout, root: &Path, crate_dir: String, family: Option<&str>, dir: &str) {
    let path = root.join(&crate_dir);
    for inner in subdirs(&path) {
        if path.join(&inner).join("Cargo.toml").is_file() {
            found
                .problems
                .push(format!("{crate_dir}/{inner}: a crate dir holds no crate"));
        }
    }
    let (name, stem) = match family {
        None => (format!("@specforge/{dir}"), dir.to_string()),
        Some(family) => (format!("@{family}/{dir}"), format!("{family}_{dir}")),
    };
    if path.join("wasm").is_dir() {
        let artifact = format!("specforge_ext_{}.wasm", stem.replace('-', "_"));
        found.builtins.push(Named {
            crate_dir,
            name,
            artifact,
        });
        return;
    }
    let Some(family) = family else {
        found.problems.push(format!(
            "{crate_dir}: a crate at extensions/<dir> is a builtin, and holds wasm/"
        ));
        return;
    };
    // shortcut: line matching, not a TOML parse (this crate has no `toml` dependency and the test adds none);
    // a library manifest writes `name` and `crate-type` on lines of their own, as `gsd/kit` does.
    let manifest = std::fs::read_to_string(path.join("Cargo.toml")).unwrap_or_default();
    let package = format!("name = \"specforge-{family}-{dir}\"");
    if !manifest.lines().any(|line| line.trim() == package) {
        found.problems.push(format!(
            "{crate_dir}: an extension library's package is specforge-{family}-{dir}"
        ));
    }
    if !manifest
        .lines()
        .any(|line| line.replace(' ', "") == "crate-type=[\"rlib\"]")
    {
        found.problems.push(format!(
            "{crate_dir}: a crate without wasm/ is an extension library, crate-type = [\"rlib\"]"
        ));
    }
}

/// Walk `<root>/extensions/` by the rule. Each entry is a crate (it holds `Cargo.toml`) or a family (it holds
/// crates and is none); a crate holds no crate; there is no `specforge` family. A crate with `wasm/` is a builtin:
/// `extensions/<dir>` is `@specforge/<dir>` and `extensions/<family>/<pack>` is `@<family>/<pack>`, its artifact
/// `specforge_ext_<…>.wasm` with `/` and `-` as `_`. A family crate without `wasm/` is an extension library: an rlib
/// named `specforge-<family>-<dir>`.
fn layout(root: &Path) -> Layout {
    let extensions = root.join("extensions");
    let mut found = Layout::default();
    for entry in subdirs(&extensions) {
        if extensions.join(&entry).join("Cargo.toml").is_file() {
            add_crate(
                &mut found,
                root,
                format!("extensions/{entry}"),
                None,
                &entry,
            );
            continue;
        }
        if entry == "specforge" {
            found.problems.push(format!(
                "extensions/{entry}: the specforge family is refused (a @specforge/ builtin lives at extensions/<dir>)"
            ));
            continue;
        }
        let packs = subdirs(&extensions.join(&entry));
        if packs.is_empty() {
            found
                .problems
                .push(format!("extensions/{entry}: neither a crate nor a family"));
        }
        for pack in packs {
            if extensions
                .join(&entry)
                .join(&pack)
                .join("Cargo.toml")
                .is_file()
            {
                add_crate(
                    &mut found,
                    root,
                    format!("extensions/{entry}/{pack}"),
                    Some(&entry),
                    &pack,
                );
            } else {
                found.problems.push(format!(
                    "extensions/{entry}/{pack}: a family holds crates only"
                ));
            }
        }
    }
    found.builtins.sort();
    found
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "every builtin is listed once, by the directory its crate lives in"
)]
fn the_table_is_the_extensions_directory_by_the_naming_rule() {
    let on_disk = layout(specforge_repo_root::repo_root());
    assert!(on_disk.problems.is_empty(), "{:#?}", on_disk.problems);
    let mut listed: Vec<Named> = BUILTINS
        .iter()
        .map(|b| Named::new(b.crate_dir, b.name, b.artifact))
        .collect();
    listed.sort();
    assert_eq!(
        listed, on_disk.builtins,
        "BUILTINS lists every builtin crate under extensions/, once, named by the rule"
    );
    for builtin in BUILTINS {
        assert!(
            !builtin.bytes.is_empty(),
            "{} embeds its component",
            builtin.crate_dir
        );
    }

    assert_eq!(BUILTIN_EXTENSIONS.len(), BUILTINS.len());
    for (view, builtin) in BUILTIN_EXTENSIONS.iter().zip(BUILTINS) {
        assert_eq!(view.0, builtin.name);
        assert_eq!(view.1, builtin.bytes);
    }
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "a layout under extensions/ that breaks the naming rule is named, and a family's builtins and libraries are read by it"
)]
fn a_layout_that_breaks_the_naming_rule_is_named() {
    let root = tempfile::tempdir().unwrap();
    let write = |file: &str, text: &str| {
        let path = root.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    let wasm = |dir: &str| std::fs::create_dir_all(root.path().join(dir).join("wasm")).unwrap();
    // Within the rule: a depth-1 builtin, a family builtin and a family library.
    write("extensions/rust/Cargo.toml", "");
    wasm("extensions/rust");
    write("extensions/javascript/node-test/Cargo.toml", "");
    wasm("extensions/javascript/node-test");
    write(
        "extensions/code/junit/Cargo.toml",
        "[package]\nname = \"specforge-code-junit\"\n\n[lib]\ncrate-type = [\"rlib\"]\n",
    );
    // Against it: a crate in a crate, the specforge family, a cdylib without wasm/, an empty entry.
    write("extensions/rust/x/Cargo.toml", "");
    write("extensions/specforge/x/Cargo.toml", "");
    write(
        "extensions/code/lib/Cargo.toml",
        "[package]\nname = \"specforge-code-lib\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n",
    );
    std::fs::create_dir_all(root.path().join("extensions/empty")).unwrap();

    let found = layout(root.path());
    assert_eq!(
        found.builtins,
        [
            Named::new(
                "extensions/javascript/node-test",
                "@javascript/node-test",
                "specforge_ext_javascript_node_test.wasm"
            ),
            Named::new(
                "extensions/rust",
                "@specforge/rust",
                "specforge_ext_rust.wasm"
            ),
        ]
    );
    assert_eq!(
        found.problems,
        [
            "extensions/code/lib: a crate without wasm/ is an extension library, crate-type = [\"rlib\"]",
            "extensions/empty: neither a crate nor a family",
            "extensions/rust/x: a crate dir holds no crate",
            "extensions/specforge: the specforge family is refused (a @specforge/ builtin lives at extensions/<dir>)",
        ]
    );
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "every fixture crate is listed once, by its directory under fixtures/"
)]
fn the_fixtures_are_the_fixtures_directory() {
    let fixtures = specforge_repo_root::repo_root().join("fixtures");
    let mut on_disk: Vec<String> = std::fs::read_dir(&fixtures)
        .expect("fixtures/")
        .flatten()
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| format!("fixtures/{}", entry.file_name().to_string_lossy()))
        .collect();
    on_disk.sort();
    let mut listed: Vec<&str> = FIXTURES.iter().map(|b| b.crate_dir).collect();
    listed.sort();
    assert_eq!(
        listed, on_disk,
        "FIXTURES lists every crate under fixtures/, once"
    );

    // Each committed blob's wasm/ holds its artifact, and `inputs.json` once it is recorded.
    for blob in COMMITTED {
        let wasm = blob.path().parent().expect("a file path").to_path_buf();
        let mut held: Vec<String> = std::fs::read_dir(&wasm)
            .unwrap_or_else(|e| panic!("{}: {e}", wasm.display()))
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name != "inputs.json")
            .collect();
        held.sort();
        assert_eq!(held, [blob.artifact], "{}: its wasm/ directory", blob.name);
    }
}

#[specforge_test(
    behavior = "load_extension_manifests",
    verify = "every example's extension crate is listed once, by its directory under examples/"
)]
fn every_examples_extension_is_a_committed_blob() {
    let examples = specforge_repo_root::repo_root().join("examples");
    let mut on_disk: Vec<String> = std::fs::read_dir(examples)
        .expect("examples/")
        .flatten()
        .filter(|entry| entry.path().join("extension/Cargo.toml").is_file())
        .map(|entry| format!("examples/{}/extension", entry.file_name().to_string_lossy()))
        .collect();
    on_disk.sort();
    let mut listed: Vec<&str> = EXAMPLES.iter().map(|b| b.crate_dir).collect();
    listed.sort();
    assert_eq!(
        listed, on_disk,
        "EXAMPLES lists every examples/*/extension crate, once"
    );
}
