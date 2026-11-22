use specforge_code_facts::dependency_facts;
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// A cargo-cyclonedx BOM, hand-written to CycloneDX 1.6 and the tool's shape: crate `demo` at the project root.
const CARGO: &str = include_str!("../fixtures/cargo.cdx.json");
/// A cyclonedx-npm BOM, hand-written to CycloneDX 1.6 and the tool's shape: package `@demo/board` at `web`.
const NPM: &str = include_str!("../fixtures/npm.cdx.json");
/// A CycloneDX 1.4 BOM with no dependency graph and a component with no purl, hand-written.
const NO_PURL: &str = include_str!("../fixtures/no-purl.cdx.json");
/// The served `package` facts an SBOM collector receives, as JSON.
const PACKAGES: &str = include_str!("../fixtures/packages.json");

fn served() -> Vec<Fact> {
    serde_json::from_str(PACKAGES).expect("the served facts parse")
}

fn field<'f>(fact: &'f Fact, name: &str) -> Option<&'f Value> {
    fact.fields.get(name)
}

#[test]
fn a_cargo_bom_mints_dependencies_citing_the_project_package() {
    let bom = read(CARGO).expect("the BOM reads");
    let facts = dependency_facts(&[bom], &served()).expect("the root is a served package");
    let named: Vec<(&str, Option<bool>)> = facts
        .iter()
        .map(|f| {
            assert_eq!(
                field(f, "package").and_then(Value::as_str),
                Some("pkg:cargo/demo")
            );
            assert!(
                f.id.starts_with("dep:pkg:cargo/demo/pkg%3Acargo%2F"),
                "{}",
                f.id
            );
            (
                field(f, "name").and_then(Value::as_str).unwrap_or_default(),
                field(f, "direct").and_then(Value::as_bool),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("itoa", Some(false)),
            ("serde", Some(true)),
            ("serde_derive", Some(false)),
        ]
    );
    insta::assert_json_snapshot!("cargo", facts);
}

#[test]
fn a_cargo_bom_reads_its_root_and_components_and_the_roots_direct_refs() {
    let bom = read(CARGO).expect("the BOM reads");
    assert_eq!(
        bom.root.purl.as_deref(),
        Some("pkg:cargo/demo@0.1.0?download_url=file://.")
    );
    let names: Vec<&str> = bom.components.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["serde", "serde_derive", "itoa"]);
    assert_eq!(
        bom.direct,
        Some(vec![
            "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.215".to_string()
        ])
    );
}

/// Each fact's name, scope and directness.
fn read_out(facts: &[Fact]) -> Vec<(&str, &str, Option<bool>)> {
    facts
        .iter()
        .map(|f| {
            let text = |name: &str| field(f, name).and_then(Value::as_str).unwrap_or_default();
            (
                text("name"),
                text("scope"),
                field(f, "direct").and_then(Value::as_bool),
            )
        })
        .collect()
}

#[test]
fn an_npm_bom_flattens_nesting_and_merges_a_repeated_purl() {
    let bom = read(NPM).expect("the BOM reads");
    let names: Vec<&str> = bom.components.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "runtime",
            "fsevents",
            "react",
            "loose-envify",
            "js-tokens",
            "js-tokens"
        ]
    );
    let facts = dependency_facts(&[bom], &served()).expect("the root is a served package");
    assert!(facts.iter().all(|f| {
        f.id.starts_with("dep:pkg:npm/%40demo/board/")
            && field(f, "package").and_then(Value::as_str) == Some("pkg:npm/%40demo/board")
    }));
    assert_eq!(
        read_out(&facts),
        [
            ("@babel/runtime", "required", Some(true)),
            ("fsevents", "optional", Some(true)),
            ("js-tokens", "required", Some(false)),
            ("loose-envify", "required", Some(false)),
            ("react", "required", Some(true)),
        ]
    );
    insta::assert_json_snapshot!("npm", facts);
}

#[test]
fn a_no_purl_bom_states_no_directness_and_names_a_purl_less_component() {
    let bom = read(NO_PURL).expect("the BOM reads");
    assert_eq!(bom.direct, None);
    let facts = dependency_facts(&[bom], &served()).expect("the root is a served package");
    assert!(facts.iter().all(|f| !f.fields.contains_key("direct")));
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "dep:pkg:cargo/demo/@madler%2Fzlib%401.3.1",
            "dep:pkg:cargo/demo/pkg%3Acargo%2Fanyhow%401.0.93",
        ]
    );
    assert_eq!(
        read_out(&facts),
        [
            ("madler/zlib", "excluded", None),
            ("anyhow", "required", None)
        ]
    );
    insta::assert_json_snapshot!("no_purl", facts);
}

/// A minimal BOM of `spec_version` and `format` describing `root_purl`, with one component of `scope`.
fn bom_text(spec_version: &str, format: &str, root_purl: &str, scope: &str) -> String {
    serde_json::json!({
        "bomFormat": format,
        "specVersion": spec_version,
        "metadata": {"component": {"type": "application", "name": "demo", "purl": root_purl}},
        "components": [{"type": "library", "name": "serde", "purl": "pkg:cargo/serde@1.0.0", "scope": scope}]
    })
    .to_string()
}

#[test]
fn an_error_shows_the_boms_values_bounded_and_escaped() {
    let hostile = format!("\u{1b}]0;{}", "a".repeat(5_000));
    let root = "pkg:cargo/demo@0.1.0";
    for text in [
        bom_text("1.5", &hostile, root, "required"),
        bom_text(&hostile, "CycloneDX", root, "required"),
        bom_text("1.5", "CycloneDX", root, &hostile),
    ] {
        let error = read(&text).expect_err("refused");
        assert!(
            !error.contains('\u{1b}') && error.contains("\\u{1b}"),
            "{error}"
        );
        assert!(error.len() < 400, "{} bytes", error.len());
    }
}

#[test]
fn a_bad_bom_is_an_error_naming_its_value_or_key_and_mints_nothing() {
    let root = "pkg:cargo/demo@0.1.0";
    let ok = |version: &str| bom_text(version, "CycloneDX", root, "required");
    let mut no_root: Value = serde_json::from_str(&ok("1.5")).unwrap();
    no_root["metadata"] = serde_json::json!({});
    let mut no_name: Value = serde_json::from_str(&ok("1.5")).unwrap();
    no_name["components"][0]
        .as_object_mut()
        .unwrap()
        .remove("name");
    let mut not_listed: Value = serde_json::from_str(&ok("1.5")).unwrap();
    not_listed["components"] = serde_json::json!({"name": "serde"});
    let twins = vec![
        Fact::new("package", "pkg:cargo/demo").field("path", ""),
        Fact::new("package", "pkg:cargo/demo#crates/demo").field("path", "crates/demo"),
    ];
    let rows: [(&str, String, Vec<Fact>, &[&str]); 10] = [
        (
            "specVersion 1.3",
            ok("1.3"),
            served(),
            &["specVersion", "\"1.3\""],
        ),
        (
            "specVersion 2.0",
            ok("2.0"),
            served(),
            &["specVersion", "\"2.0\""],
        ),
        (
            "bomFormat SPDX",
            bom_text("1.5", "SPDX", root, "required"),
            served(),
            &["bomFormat", "\"SPDX\""],
        ),
        (
            "no metadata.component",
            no_root.to_string(),
            served(),
            &["metadata.component"],
        ),
        (
            "a root no package names",
            bom_text("1.5", "CycloneDX", "pkg:cargo/elsewhere@1.0.0", "required"),
            served(),
            &["pkg:cargo/elsewhere@1.0.0"],
        ),
        (
            "a root two packages name",
            ok("1.5"),
            twins,
            &[root, "pkg:cargo/demo#crates/demo"],
        ),
        (
            "scope mandatory",
            bom_text("1.5", "CycloneDX", root, "mandatory"),
            served(),
            &["components[0].scope", "\"mandatory\""],
        ),
        (
            "no JSON",
            "{\"bomFormat\":".to_string(),
            served(),
            &["no JSON"],
        ),
        (
            "a component with no name",
            no_name.to_string(),
            served(),
            &["components[0].name"],
        ),
        (
            "components no array",
            not_listed.to_string(),
            served(),
            &["components: no array"],
        ),
    ];
    for (case, text, served, named) in rows {
        let minted = read(&text).and_then(|bom| dependency_facts(&[bom], &served));
        let error = minted.expect_err(case);
        for name in named {
            assert!(error.contains(name), "{case}: {error}");
        }
    }
}
