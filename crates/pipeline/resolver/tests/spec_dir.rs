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

/// A source whose name holds a `:` or a `\` (a name Linux allows) is never suggested, since no import path can name it,
/// and an import path holding one is told why it names nothing.
#[specforge_test(requirement = "FEATURE-109")]
fn an_e025_never_suggests_a_path_an_import_cannot_hold() {
    const SLASHED: &str = "an import path is `/`-separated and holds no `\\` or `:`";
    let e025 = |files: &[(&str, &str)]| -> Vec<String> {
        codes(files)
            .into_iter()
            .filter(|(code, _)| code == "E025")
            .map(|(_, hint)| hint)
            .collect()
    };
    assert_eq!(
        e025(&[("spec/main.spec", "use \"a:b\"\n"), ("spec/a:b.spec", "")]),
        [SLASHED]
    );
    assert_eq!(
        e025(&[
            ("spec/main.spec", "use \"sub\\\\b\"\n"),
            ("spec/sub/b.spec", "")
        ]),
        [format!("{SLASHED}: did you mean 'sub/b'?")]
    );
    // A path an import may hold is not pointed at a source no import can name.
    let found = e025(&[("spec/main.spec", "use \"a_b\"\n"), ("spec/a:b.spec", "")]);
    assert!(
        found.len() == 1 && found[0].starts_with("a use path names a project source"),
        "{found:?}"
    );
}

/// On Linux a `\` is part of a name, so a source may be named with one; no import names it, and none is pointed at it.
#[cfg(unix)]
#[specforge_test(requirement = "FEATURE-109")]
fn an_e025_never_suggests_a_source_named_with_a_backslash() {
    let found = codes(&[
        ("spec/main.spec", "use \"a\\\\b\"\n"),
        ("spec/a\\b.spec", ""),
    ]);
    assert_eq!(
        found,
        [(
            "E025".to_string(),
            "an import path is `/`-separated and holds no `\\` or `:`".to_string()
        )]
    );
}

/// A spec root outside the project is absolute, and its keys are the paths as the OS writes them (`ProjectPath::key`).
/// The resolver reads nothing, so the root need not exist.
#[cfg(unix)]
#[specforge_test(requirement = "FEATURE-109")]
fn an_import_under_an_absolute_spec_root_names_the_absolute_key() {
    const ROOT: &str = "/elsewhere/real";
    let keys: BTreeSet<&str> = [
        "/elsewhere/real/a.spec",
        "/elsewhere/real/types.spec",
        "/elsewhere/real/sub/b.spec",
        "/elsewhere/x.spec",
    ]
    .into();
    let named = |importing, path| import_target(&keys, Path::new(ROOT), importing, path);
    assert_eq!(
        named("/elsewhere/real/a.spec", "types"),
        Some("/elsewhere/real/types.spec")
    );
    assert_eq!(
        named("/elsewhere/real/a.spec", "./sub/b"),
        Some("/elsewhere/real/sub/b.spec")
    );
    assert_eq!(
        named("/elsewhere/real/sub/b.spec", "../types"),
        Some("/elsewhere/real/types.spec")
    );
    assert_eq!(named("/elsewhere/real/a.spec", "../x"), None);

    let parsed = parse("use \"sub/c\"\n", "/elsewhere/real/a.spec");
    let other = parse("", "/elsewhere/real/sub/b.spec");
    let found: Vec<_> = resolve_imports(
        &[
            ("/elsewhere/real/a.spec", &parsed),
            ("/elsewhere/real/sub/b.spec", &other),
        ],
        &[],
        Path::new(ROOT),
    )
    .into_iter()
    .filter(|d| d.code == "E025")
    .map(|d| d.suggestion.unwrap_or_default())
    .collect();
    // The suggestion is an import path, read from the spec root, not the absolute key.
    assert_eq!(found, ["did you mean 'sub/b'?"]);
}

/// A relative spec root nested in the project: the key a candidate names is its names joined by `/`.
#[specforge_test(requirement = "FEATURE-109")]
fn an_import_under_a_nested_spec_root_names_the_slash_joined_key() {
    let keys: BTreeSet<&str> = [
        "docs/spec/a.spec",
        "docs/spec/sub/b.spec",
        "docs/spec/sub/index.spec",
    ]
    .into();
    let named = |importing, path| import_target(&keys, Path::new("docs/spec"), importing, path);
    assert_eq!(
        named("docs/spec/a.spec", "sub/b"),
        Some("docs/spec/sub/b.spec")
    );
    assert_eq!(
        named("docs/spec/sub/b.spec", "../a"),
        Some("docs/spec/a.spec")
    );
    assert_eq!(
        named("docs/spec/a.spec", "sub"),
        Some("docs/spec/sub/index.spec")
    );
}
