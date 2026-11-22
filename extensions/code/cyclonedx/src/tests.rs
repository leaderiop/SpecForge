use specforge_code_facts::dependency_facts;
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// A cargo-cyclonedx BOM, hand-written to CycloneDX 1.6 and the tool's shape: crate `demo` at the project root.
const CARGO: &str = include_str!("../fixtures/cargo.cdx.json");
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
