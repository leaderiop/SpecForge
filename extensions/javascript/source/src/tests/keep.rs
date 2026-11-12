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

#[specforge_test(
    behavior = "js_select_source_files",
    verify = "the keep rules judge a path inside its package, a package.json too"
)]
fn the_keep_rules_judge_a_path_inside_its_package_a_package_json_too() {
    // A named package under tests/ or build/ of the root package is no package the pack reads, nor are its files.
    let fixture = run(
        host(&[
            APP,
            ("tests/fixture/package.json", r#"{"name":"fixture"}"#),
            ("tests/fixture/src/a.js", "export const a = 1;\n"),
            ("build/copy/package.json", r#"{"name":"copy"}"#),
            ("build/copy/src/b.js", "export const b = 1;\n"),
            ("src/c.js", "export const c = 1;\n"),
        ]),
        &[],
    )
    .0
    .unwrap();
    assert_eq!(packages(&fixture), ["pkg:npm/app"]);
    assert_eq!(items(&fixture), ["js:app/src/c.js::c"]);
    // A dependency names the shallowest package of its name the pack reads, never a fixture's plain id.
    let named = run(
        host(&[
            (
                "package.json",
                r#"{"name":"app","dependencies":{"lib":"*"}}"#,
            ),
            ("tests/lib/package.json", r#"{"name":"lib"}"#),
            ("packages/x/lib/package.json", r#"{"name":"lib"}"#),
        ]),
        &[],
    )
    .0
    .unwrap();
    assert_eq!(
        packages(&named),
        ["pkg:npm/app", "pkg:npm/lib#packages/x/lib"]
    );
    let app = named.iter().find(|f| f.id == "pkg:npm/app").unwrap();
    assert_eq!(
        app.fields["depends_on"],
        json!(["pkg:npm/lib#packages/x/lib"])
    );
    // With no package above it (an unnamed root), a package.json is judged on its path in the project: a fixture
    // under tests/ is still none, a package under packages/ judges its own files inside it.
    let unnamed = run(
        host(&[
            ("package.json", r#"{"private":true}"#),
            ("tests/fixtures/x/package.json", r#"{"name":"x"}"#),
            ("tests/fixtures/x/src/a.js", "export const a = 1;\n"),
            ("packages/tool/package.json", r#"{"name":"tool"}"#),
            ("packages/tool/src/a.js", "export const a = 1;\n"),
            ("packages/tool/test/a.test.js", "export const t = 1;\n"),
        ]),
        &[],
    )
    .0
    .unwrap();
    assert_eq!(packages(&unnamed), ["pkg:npm/tool"]);
    assert_eq!(items(&unnamed), ["js:tool/src/a.js::a"]);
}
