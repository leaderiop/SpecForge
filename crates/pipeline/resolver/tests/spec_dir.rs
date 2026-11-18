//! An import path is read from the spec root among keys named from the project root (ADR 0242).

use specforge_parser::{SpecFile, parse};
use specforge_resolver::{import_target, resolve_imports};
use specforge_test_macros::test as specforge_test;
use std::collections::BTreeSet;
use std::path::Path;

const SPEC_DIR: &str = "spec";

fn codes(files: &[(&str, &str)]) -> Vec<(String, String)> {
    let parsed: Vec<(&str, SpecFile)> = files.iter().map(|(k, t)| (*k, parse(t, k))).collect();
    let files: Vec<(&str, &SpecFile)> = parsed.iter().map(|(k, f)| (*k, f)).collect();
    resolve_imports(&files, &[], Path::new(SPEC_DIR))
        .into_iter()
        .map(|d| (d.code.to_string(), d.suggestion.unwrap_or_default()))
        .collect()
}

#[specforge_test(requirement = "FEATURE-109")]
fn an_import_path_is_read_from_the_spec_dir_among_prefixed_keys() {
    let keys: BTreeSet<&str> = [
        "spec/a.spec",
        "spec/types.spec",
        "spec/sub/b.spec",
        "x.spec",
    ]
    .into();
    let named = |importing, path| import_target(&keys, Path::new(SPEC_DIR), importing, path);
    assert_eq!(named("spec/a.spec", "types"), Some("spec/types.spec"));
    assert_eq!(named("spec/a.spec", "./sub/b"), Some("spec/sub/b.spec"));
    assert_eq!(
        named("spec/sub/b.spec", "../types"),
        Some("spec/types.spec")
    );
    // `..` out of the spec root leaves it, even with a key `x.spec` beside it.
    assert_eq!(named("spec/a.spec", "../x"), None);

    let found = codes(&[
        ("spec/a.spec", "use \"types\"\nuse \"../x\"\nuse \"typs\"\n"),
        ("spec/types.spec", ""),
        ("x.spec", ""),
    ]);
    let e025: Vec<_> = found.iter().filter(|(code, _)| code == "E025").collect();
    assert_eq!(e025.len(), 2, "{found:?}");
    // A suggestion stays an import path, not a key.
    assert!(
        e025.iter().any(|(_, hint)| hint == "did you mean 'types'?"),
        "{found:?}"
    );
}

/// An import path is `/`-separated text on every OS: a `\` (a separator only on Windows) or a `:` (a drive only on
/// Windows) names no source on any host, so a spec that checks clean on one checks clean on all.
#[specforge_test(requirement = "FEATURE-109")]
fn an_import_path_with_a_backslash_or_a_drive_names_nothing_on_every_os() {
    let keys: BTreeSet<&str> = ["spec/a.spec", "spec/sub/b.spec", "spec/types.spec"].into();
    let named = |path| import_target(&keys, Path::new(SPEC_DIR), "spec/a.spec", path);
    assert_eq!(named("sub/b"), Some("spec/sub/b.spec"));
    for path in ["sub\\b", ".\\sub\\b", "C:types", "C:/types", "\\types"] {
        assert_eq!(named(path), None, "{path}");
    }

    let found = codes(&[
        ("spec/a.spec", "use \"sub\\\\b\"\n"),
        ("spec/sub/b.spec", ""),
    ]);
    assert_eq!(
        found.iter().filter(|(code, _)| code == "E025").count(),
        1,
        "{found:?}"
    );
}
