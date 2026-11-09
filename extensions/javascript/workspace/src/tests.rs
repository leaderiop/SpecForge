use serde_json::{json, Value};
use specforge_test::prelude::*;

use super::*;

/// The input the host sends with these `(path, text)` package files read; a `None` text is a file it could not read.
fn input(packages: &[(&str, Option<&str>)]) -> CollectInput {
    let facts: Vec<Value> = packages
        .iter()
        .map(|(path, text)| match text {
            Some(text) => json!({"path": path, "exists": true, "text": text}),
            None => json!({"path": path, "exists": true, "error": "unreadable"}),
        })
        .collect();
    serde_json::from_value(json!({"files": {PACKAGES: {"facts": facts}}})).unwrap()
}

/// The workspace over these read `(path, text)` package files.
fn workspace(packages: &[(&str, &str)]) -> Workspace {
    let read: Vec<(&str, Option<&str>)> = packages.iter().map(|(p, t)| (*p, Some(*t))).collect();
    Workspace::of(&input(&read))
}

/// The owner of `file` as `(package name, path under it)`.
fn owner<'w>(workspace: &'w Workspace, file: &'w str) -> Option<(&'w str, &'w str)> {
    workspace
        .owner(file)
        .map(|(package, under)| (package.name.as_str(), under))
}

#[specforge_test(
    behavior = "js_resolve_owning_package",
    verify = "a file is owned by its nearest named package"
)]
fn a_file_is_owned_by_its_nearest_named_package() {
    let w = workspace(&[
        ("package.json", r#"{"name":"app"}"#),
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
        ("packages/core/sub/package.json", r#"{"name":"@acme/sub"}"#),
    ]);
    assert_eq!(
        owner(&w, "packages/core/src/a.ts"),
        Some(("@acme/core", "src/a.ts"))
    );
    assert_eq!(
        owner(&w, "packages/core/sub/b.js"),
        Some(("@acme/sub", "b.js"))
    );
}

#[specforge_test(
    behavior = "js_resolve_owning_package",
    verify = "an unnamed package.json leaves the file to the one above"
)]
fn an_unnamed_package_json_leaves_the_file_to_the_one_above() {
    let w = workspace(&[
        (
            "package.json",
            r#"{"name":"app","workspaces":["packages/*"]}"#,
        ),
        ("packages/cli/package.json", r#"{"private":true}"#),
        ("packages/web/package.json", r#"{"name":42}"#),
    ]);
    assert_eq!(
        owner(&w, "packages/cli/bin/cli.mjs"),
        Some(("app", "packages/cli/bin/cli.mjs"))
    );
    assert_eq!(
        owner(&w, "packages/web/x.js"),
        Some(("app", "packages/web/x.js"))
    );
    assert_eq!(w.packages().len(), 1);
}

#[specforge_test(
    behavior = "js_resolve_owning_package",
    verify = "an unread or non-JSON package.json is no package"
)]
fn an_unread_or_non_json_package_json_is_no_package() {
    let w = Workspace::of(&input(&[
        ("package.json", Some(r#"{"name":"app"}"#)),
        ("a/package.json", None),
        ("b/package.json", Some("{ name: 'b' }")),
    ]));
    assert_eq!(
        w.packages()
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["app"]
    );
    assert_eq!(owner(&w, "a/x.js"), Some(("app", "a/x.js")));
    assert_eq!(owner(&w, "b/x.js"), Some(("app", "b/x.js")));
}

#[specforge_test(
    behavior = "js_resolve_owning_package",
    verify = "a file under no named package has no owner"
)]
fn a_file_under_no_named_package_has_no_owner() {
    let w = workspace(&[
        ("package.json", r#"{"private":true}"#),
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
    ]);
    assert_eq!(owner(&w, "src/a.js"), None);
    assert_eq!(owner(&w, "packages/x.js"), None);
    assert_eq!(Workspace::of(&CollectInput::default()).owner("a.js"), None);
}

#[specforge_test(
    behavior = "js_resolve_owning_package",
    verify = "a sibling with a shared prefix does not own a file"
)]
fn a_sibling_with_a_shared_prefix_does_not_own_a_file() {
    let w = workspace(&[
        ("package.json", r#"{"name":"app"}"#),
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
    ]);
    assert_eq!(
        owner(&w, "packages/core-utils/x.js"),
        Some(("app", "packages/core-utils/x.js"))
    );
}

#[specforge_test(
    behavior = "js_resolve_owning_package",
    verify = "the root package owns what no nested one claims"
)]
fn the_root_package_owns_what_no_nested_one_claims() {
    let w = workspace(&[
        ("packages/core/package.json", r#"{"name":"@acme/core"}"#),
        ("package.json", r#"{"name":"app"}"#),
    ]);
    assert_eq!(owner(&w, "index.js"), Some(("app", "index.js")));
    assert_eq!(
        owner(&w, "scripts/build.cjs"),
        Some(("app", "scripts/build.cjs"))
    );
    assert_eq!(
        owner(&w, "packages/core/index.js"),
        Some(("@acme/core", "index.js"))
    );
}

#[specforge_test(
    behavior = "js_resolve_owning_package",
    verify = "a package keeps its version, its engines and the names it depends on"
)]
fn a_package_keeps_its_version_and_the_names_it_depends_on() {
    let w = workspace(&[(
        "packages/core/package.json",
        r#"{
            "name": "@acme/core",
            "version": "1.2.0",
            "engines": {"node": ">=20", "npm": 9},
            "dependencies": {"lodash": "^4", "app": "*"},
            "devDependencies": {"typescript": "^5", "lodash": "^4"},
            "peerDependencies": {"react": "^19"},
            "optionalDependencies": {"fsevents": "^2"}
        }"#,
    )]);
    assert_eq!(
        w.packages(),
        [Package {
            dir: "packages/core".to_string(),
            name: "@acme/core".to_string(),
            version: Some("1.2.0".to_string()),
            engines: vec!["node >=20".to_string()],
            depends_on: ["app", "fsevents", "lodash", "react", "typescript"]
                .map(str::to_string)
                .to_vec(),
        }]
    );
    let unversioned = workspace(&[("package.json", r#"{"name":"app","version":1}"#)]);
    assert_eq!(unversioned.packages()[0].version, None);
    assert_eq!(unversioned.packages()[0].dir, "");
}
