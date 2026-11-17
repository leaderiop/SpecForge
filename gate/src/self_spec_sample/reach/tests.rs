use super::*;
use crate::repo::Dependency;

use DependencyKind::{Build, Dev, Normal};

fn member(
    name: &str,
    dir: &str,
    dependencies: &[(&str, DependencyKind)],
    bins: &[(&str, &str)],
) -> Member {
    Member {
        name: name.into(),
        dir: dir.into(),
        dependencies: dependencies
            .iter()
            .map(|&(name, kind)| Dependency {
                name: name.into(),
                dir: format!("crates/{name}"),
                kind,
            })
            .collect(),
        bins: bins
            .iter()
            .map(|&(bin, root)| (bin.to_string(), format!("{dir}/{root}")))
            .collect(),
        ..Default::default()
    }
}

/// The CLI links `lib` (which a dev-dependency `lib_dev` never reaches) and `tool`, a library with its own binary; the
/// converter's package holds a second binary; the cited package `cited` builds with its dev-dependency `support`,
/// whose own dev-dependency `deep` no test builds.
fn literal() -> Repo {
    Repo {
        root: "/r".into(),
        target_dir: "/r/target".into(),
        members: vec![
            member(
                "specforge-cli",
                "crates/specforge-cli",
                &[("lib", Normal), ("tool", Build), ("cli_dev", Dev)],
                &[("specforge", "src/main.rs")],
            ),
            member("lib", "crates/lib", &[("lib_dev", Dev)], &[]),
            member("lib_dev", "crates/lib_dev", &[], &[]),
            member("cli_dev", "crates/cli_dev", &[], &[]),
            member("tool", "crates/tool", &[], &[("tool", "src/bin/tool.rs")]),
            member(
                "xtask",
                "xtask",
                &[],
                &[("absorb", "src/absorb.rs"), ("revendor", "src/revendor.rs")],
            ),
            member("gate", "gate", &[], &[("gate", "src/main.rs")]),
            member(
                "cited",
                "crates/cited",
                &[("support", Dev)],
                &[("helper", "src/bin/helper.rs")],
            ),
            member("support", "crates/support", &[("deep", Dev)], &[]),
            member("deep", "crates/deep", &[], &[]),
            member("elsewhere", "crates/elsewhere", &[], &[]),
        ],
    }
}

#[test]
fn the_run_builds_from_its_programs_and_cited_tests_with_their_dependencies() {
    let repo = literal();
    let cited = BTreeSet::from(["cited".to_string()]);
    let built = built_from(&repo, &cited);
    assert_eq!(
        built,
        [
            "cited",
            "gate",
            "lib",
            "specforge-cli",
            "support",
            "tool",
            "xtask"
        ]
        .map(String::from)
        .into(),
        "a program's dev-dependency, a dependency's dev-dependency and an unrelated member build nothing in"
    );
    let reaches = |path: &str| built_into_the_run(&repo, &built, &cited, path);
    for path in [
        "xtask/src/absorb.rs",
        "xtask/src/absorb/scope.rs",
        "xtask/src/lib.rs",
        "xtask/src/links.rs",
        "xtask/Cargo.toml",
        "crates/specforge-cli/src/main.rs",
        "crates/tool/src/lib.rs",
        "crates/cited/src/bin/helper.rs",
        "crates/cited/tests/t.rs",
        "crates/support/src/lib.rs",
        "gate/src/main.rs",
    ] {
        assert!(reaches(path), "{path}");
    }
    for path in [
        "xtask/src/revendor.rs",
        "xtask/src/revendor/tests.rs",
        "crates/tool/src/bin/tool.rs",
        "crates/cli_dev/src/lib.rs",
        "crates/lib_dev/src/lib.rs",
        "crates/deep/src/lib.rs",
        "crates/elsewhere/src/lib.rs",
        "docs/guide.md",
    ] {
        assert!(!reaches(path), "{path}");
    }
}

#[test]
fn a_cited_test_names_its_package_at_any_depth_of_the_ledger() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = dir.path().join("ledger.json");
    std::fs::write(
        &ledger,
        r#"{"entries": [
            {"why": {"collapsed": {"into": ["rust:specforge-ops::operations#a::b", "rust:specforge-wasm#c"]}}},
            {"value": "rust:no test id", "other": ["plain"]}
        ]}"#,
    )
    .unwrap();
    assert_eq!(
        cited_packages(&ledger).unwrap(),
        ["specforge-ops", "specforge-wasm"].map(String::from).into()
    );
    let missing = cited_packages(&dir.path().join("none.json")).unwrap_err();
    assert!(missing.contains("the sample's ledger"), "{missing}");
}

/// A test the summary cites whose package the ledger does not name is one the trigger would not reach.
#[test]
fn a_cited_test_the_ledger_does_not_name_is_unreached() {
    let reached = BTreeSet::from(["specforge-ops".to_string()]);
    let cited = [
        "rust:specforge-ops::operations#a::b",
        "rust:specforge-wasm#c",
        "rust:no test id",
        "unit:x#y",
    ]
    .map(String::from);
    assert_eq!(
        unreached(&cited, &reached),
        ["rust:specforge-wasm#c", "rust:no test id", "unit:x#y"]
    );
    assert!(unreached(&cited[..1], &reached).is_empty());
    assert!(
        unreached(&[], &BTreeSet::new()).is_empty(),
        "none cited, none unreached"
    );
    assert_eq!(package_of("rust:a::b#m::t"), Some("a"));
    assert_eq!(package_of("rust:a#n::u"), Some("a"));
    assert_eq!(package_of("rust:a"), None);
}

/// Every file of the converter's module tree reaches the sample, wherever it is added: what the hand-written list
/// left out (the `xtask` library absorb links, with its links module) now reaches it, while another `xtask` binary and
/// a file no program builds do not. absorb cites no test since ADR 0370 D8, so the run builds no test's package, and
/// the test support outside `crates/`, which only a cited test built, no longer reaches it.
#[test]
fn the_sample_is_reached_by_every_source_its_run_builds_in_this_repository() {
    let repo = Repo::discover().unwrap();
    let selection = |path: &str| reached(&repo, &ChangeSet::of([path])).unwrap();
    let mut absorb = vec!["xtask/src/absorb.rs".to_string()];
    let mut dirs = vec![repo.root.join("xtask/src/absorb")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let relative = path.strip_prefix(&repo.root).unwrap();
                absorb.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    assert!(
        absorb.len() > 10,
        "the converter's tree was read: {absorb:?}"
    );
    for path in absorb.iter().map(String::as_str).chain([
        "xtask/src/lib.rs",
        "xtask/src/links.rs",
        "xtask/Cargo.toml",
        "gate/src/run.rs",
        "gate/golden/self-spec-sample/ledger.json",
        "crates/app/ops/src/lib.rs",
        "crates/runtime/component/tests/declarations/rust_test/describe_capabilities.json",
        "spec/governance/constraints.spec",
        "rust-toolchain.toml",
        "extensions/rust/test/src/lib.rs",
    ]) {
        assert_eq!(selection(path), Selection::Everything, "{path}");
    }
    for path in [
        "xtask/src/revendor.rs",
        "xtask/src/build_blobs/tests.rs",
        "integrations/rust/specforge-test/src/lib.rs",
        "integrations/rust/specforge-test-macros/src/lib.rs",
        "integrations/rust/spec/a.spec",
        "integrations/vscode/src/extension.ts",
        "examples/shop/spec/a.spec",
        "docs/guide.md",
    ] {
        assert_eq!(selection(path), Selection::Skip, "{path}");
    }
}
