//! What the pack refuses to lose: a glob's walk stopped short, a file the fs port withholds, and which `package.json`
//! names a package the pack reads.

use super::*;

/// The `package` facts' ids.
fn packages(facts: &[Fact]) -> Vec<&str> {
    facts
        .iter()
        .filter(|f| f.kind == "package")
        .map(|f| f.id.as_str())
        .collect()
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "a short walk is the collector's error whatever the answer's cap"
)]
fn a_short_walk_is_the_collectors_error_whatever_the_answers_cap() {
    // The `.js` listing is truncated, so its glob is parsed; the walk stops before `src/c.js`.
    let files = [
        APP,
        ("src/a.js", "export const a = 1;\n"),
        ("src/b.js", "export const b = 1;\n"),
        ("src/b.test.js", "export const t = 1;\n"),
        ("src/c.js", "export const c = 1;\n"),
    ];
    // The cap stops the answer at a file the pack keeps (`src/b.js`), or only at one it does not (the test file).
    for cap in [1, 2] {
        let short = Host {
            short: true,
            cap: Some(cap),
            ..host(&files)
        };
        let error = run(short, &["js"]).0.unwrap_err();
        assert!(
            error.contains("**/*.js") && error.contains("more files than the host walks"),
            "cap {cap}: {error}"
        );
    }
}

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "a kept file the fs port withholds is the collector's error"
)]
fn a_kept_file_the_fs_port_withholds_is_the_collectors_error() {
    let files = [
        APP,
        ("src/a.js", "export const a = 1;\n"),
        ("src/held.js", "export const h = 1;\n"),
    ];
    let withheld = Host {
        denied: vec!["src/held.js"],
        ..host(&files)
    };
    let error = run(withheld, &[]).0.unwrap_err();
    assert_eq!(
        error,
        "src/held.js: the host's reader did not parse it (denied); javascript-source reads every kept source \
         file or none"
    );
}

#[test]
fn a_package_json_under_tests_is_a_package_and_one_under_build_is_never_read() {
    let fixture = run(
        host(&[
            APP,
            ("tests/fixture/package.json", r#"{"name":"fixture"}"#),
            ("tests/fixture/src/a.js", "export const a = 1;\n"),
        ]),
        &[],
    )
    .0
    .unwrap();
    assert_eq!(packages(&fixture), ["pkg:npm/app", "pkg:npm/fixture"]);
    assert!(items(&fixture).is_empty());
    let built = run(
        host(&[
            ("build/tool/package.json", r#"{"name":"tool"}"#),
            ("build/tool/src/a.js", "export const a = 1;\n"),
        ]),
        &[],
    )
    .0
    .unwrap();
    assert_eq!(packages(&built), ["pkg:npm/tool"]);
    assert!(
        items(&built).is_empty(),
        "the package's own files are never read"
    );
}
