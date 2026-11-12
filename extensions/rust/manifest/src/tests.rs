//! The manifest library over `fixtures/workspace`: a virtual root with two members and a standalone package.

use std::path::Path;

use specforge_code_facts::{crate_segment, Edition, ItemId, Package, PackageId, Target};
use specforge_test::prelude::*;

use crate::read_packages;

/// The fixture `name` as the host's reads serve it: each `Cargo.toml` with its text, every other file listed.
fn tree(name: &str) -> (Vec<(String, String)>, Vec<String>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    let (mut manifests, mut listing) = (Vec::new(), Vec::new());
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("the fixture is readable") {
            let path = entry.expect("the fixture is readable").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let relative: Vec<String> = path
                .strip_prefix(&root)
                .expect("under the fixture")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            let relative = relative.join("/");
            if relative.ends_with("Cargo.toml") {
                let text = std::fs::read_to_string(&path).expect("the fixture is readable");
                manifests.push((relative, text));
            } else {
                listing.push(relative);
            }
        }
    }
    (manifests, listing)
}

/// The fixture workspace's packages, by name.
fn packages() -> Vec<Package> {
    let (manifests, listing) = tree("workspace");
    read_packages(&manifests, &listing).expect("the fixture reads")
}

fn package<'a>(packages: &'a [Package], name: &str) -> &'a Package {
    packages
        .iter()
        .find(|p| p.id.name() == name)
        .unwrap_or_else(|| panic!("no package {name}"))
}

fn written(package: &Package) -> Vec<String> {
    package.targets.iter().map(Target::written).collect()
}

#[specforge_test(
    behavior = "rc_read_manifests",
    verify = "a member inherits its version, edition and rust-version from its workspace"
)]
fn a_member_inherits_its_workspace_fields() {
    let packages = packages();
    let (app, base) = (package(&packages, "app"), package(&packages, "base"));
    assert_eq!(app.version.as_deref(), Some("0.3.0"));
    assert_eq!(app.edition.as_deref(), Some("2024"));
    assert_eq!(app.rust_version.as_deref(), Some("1.80"));
    assert_eq!(app.features, ["default", "fast"]);
    assert_eq!(app.path, "crates/app");
    assert_eq!(base.edition.as_deref(), Some("2021"));
    assert_eq!(base.rust_version, None);
    let at = base.at.as_ref().expect("located");
    assert_eq!(
        (at.file.as_str(), at.start_line, at.start_col, at.end_col),
        ("crates/base/Cargo.toml", 3, 2, 9),
        "the [package] key, after the comment"
    );
}

#[specforge_test(
    behavior = "rc_read_manifests",
    verify = "targets are discovered as cargo discovers them, autotests and a build script included"
)]
fn targets_are_discovered_as_cargo_discovers_them() {
    let packages = packages();
    assert_eq!(
        written(package(&packages, "app")),
        [
            "lib:app:crates/app/src/lib.rs",
            "bin:app:crates/app/src/main.rs",
            "bin:x:crates/app/src/bin/x.rs",
            "bin:y:crates/app/src/bin/y/main.rs",
            "test:a:crates/app/tests/a.rs",
            "test:b:crates/app/tests/b/main.rs",
            "example:e:crates/app/examples/e.rs",
            "bench:b:crates/app/benches/b.rs",
        ]
    );
    assert_eq!(
        written(package(&packages, "base")),
        [
            "lib:base:crates/base/src/lib.rs",
            "test:only:crates/base/tests/only.rs",
            "custom-build:build-script-build:crates/base/build.rs",
        ],
        "autotests = false keeps tests/skipped.rs out"
    );
}

#[specforge_test(
    behavior = "rc_read_manifests",
    verify = "each target has the nextest binary id its tests run under"
)]
fn each_target_has_its_nextest_binary_id() {
    let packages = packages();
    let app = package(&packages, "app");
    let edition = Edition::cargo(app.edition.as_deref()).expect("an edition");
    let ids: Vec<Option<String>> = app
        .targets
        .iter()
        .chain(&package(&packages, "base").targets)
        .map(|target| {
            let krate = crate_segment(&app.id, target.kind, &target.name);
            ItemId::rust(&krate, &["tests".to_string()], None, "adds")
                .test_id(edition)
                .map(|id| id.to_string())
        })
        .collect();
    let expected = [
        Some("rust:app#tests::adds"),
        Some("rust:app::bin/app#tests::adds"),
        Some("rust:app::bin/x#tests::adds"),
        Some("rust:app::bin/y#tests::adds"),
        Some("rust:app::a#tests::adds"),
        Some("rust:app::b#tests::adds"),
        Some("rust:app::example/e#tests::adds"),
        Some("rust:app::bench/b#tests::adds"),
        Some("rust:app#tests::adds"),
        Some("rust:app::only#tests::adds"),
        None,
    ];
    let expected: Vec<Option<String>> = expected.iter().map(|id| id.map(str::to_string)).collect();
    assert_eq!(ids, expected, "a build script runs no tests");
}

#[specforge_test(
    behavior = "rc_read_manifests",
    verify = "a target round-trips through the text its package fact writes"
)]
fn a_target_round_trips_through_its_fact_text() {
    for package in packages() {
        let fact = package.fact();
        let field = fact
            .fields
            .get("targets")
            .expect("every package has targets");
        let texts: Vec<String> = serde_json::from_value(field.clone()).expect("a list of texts");
        let read: Vec<Target> = texts
            .iter()
            .map(|text| Target::read(text).expect("a target"))
            .collect();
        assert_eq!(read, package.targets, "{}", package.id);
    }
}

#[specforge_test(
    behavior = "rc_read_manifests",
    verify = "path dependencies name the project packages they reach, never one outside it"
)]
fn path_dependencies_name_the_project_packages_they_reach() {
    let packages = packages();
    assert_eq!(
        package(&packages, "app").depends_on,
        [PackageId::cargo("base")],
        "the inherited path is the workspace's; serde and a path out of the project are none"
    );
    assert_eq!(
        package(&packages, "base").depends_on,
        [PackageId::cargo("standalone")],
        "a build dependency counts"
    );
    assert!(package(&packages, "standalone").depends_on.is_empty());
}

#[specforge_test(
    behavior = "rc_read_manifests",
    verify = "a manifest that does not parse or complete is an error naming it"
)]
fn a_manifest_that_does_not_parse_is_an_error_naming_it() {
    for (path, text) in [
        ("crates/bad/Cargo.toml", "[package\nname = \"bad\"\n"),
        (
            "lone/Cargo.toml",
            "[package]\nname = \"lone\"\nversion.workspace = true\n",
        ),
    ] {
        let manifests = [(path.to_string(), text.to_string())];
        let error = read_packages(&manifests, &[]).expect_err(path);
        assert_eq!(error.path, path);
        assert!(!error.why.is_empty(), "{error}");
    }
}

#[specforge_test(
    behavior = "rc_read_manifests",
    verify = "a virtual manifest is no package"
)]
fn a_virtual_manifest_is_no_package() {
    let names: Vec<String> = packages().iter().map(|p| p.id.to_string()).collect();
    assert_eq!(
        names,
        ["pkg:cargo/app", "pkg:cargo/base", "pkg:cargo/standalone"]
    );
    let virtual_only = [(
        "Cargo.toml".to_string(),
        "[workspace]\nmembers = []\n".to_string(),
    )];
    assert_eq!(read_packages(&virtual_only, &[]), Ok(Vec::new()));
}
