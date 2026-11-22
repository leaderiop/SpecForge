//! The `dependency` facts minted from a CycloneDX reader's BOMs and the served packages.

use crate::*;
use specforge_extension_sdk::prelude::Fact;

fn package(id: &str) -> Fact {
    Fact::new("package", id).field("path", "")
}

fn component(bom_ref: Option<&str>, purl: Option<&str>, name: &str, scope: Scope) -> Component {
    Component {
        bom_ref: bom_ref.map(str::to_string),
        purl: purl.map(str::to_string),
        group: None,
        name: name.to_string(),
        version: Some("1.0.0".to_string()),
        scope,
    }
}

fn bom(root_purl: &str, components: Vec<Component>, direct: Option<&[&str]>) -> Bom {
    Bom {
        root: component(Some("root"), Some(root_purl), "root", Scope::Required),
        components,
        direct: direct.map(|refs| refs.iter().map(|r| r.to_string()).collect()),
    }
}

fn field<'f>(fact: &'f Fact, name: &str) -> Option<&'f serde_json::Value> {
    fact.fields.get(name)
}

#[test]
fn a_bom_root_resolves_by_its_purl_without_version_or_qualifiers() {
    let served = [package("pkg:cargo/demo"), package("pkg:npm/%40demo/board")];
    let serde = component(
        None,
        Some("pkg:cargo/serde@1.0.0"),
        "serde",
        Scope::Required,
    );
    for (root, cited) in [
        (
            "pkg:cargo/demo@0.1.0?download_url=file://.",
            "pkg:cargo/demo",
        ),
        ("pkg:cargo/demo#sub/path", "pkg:cargo/demo"),
        ("pkg:npm/%40demo/board@1.0.0", "pkg:npm/%40demo/board"),
        ("pkg:npm/@demo/board@1.0.0", "pkg:npm/%40demo/board"),
    ] {
        let facts = dependency_facts(&[bom(root, vec![serde.clone()], None)], &served)
            .unwrap_or_else(|e| panic!("{root}: {e}"));
        assert_eq!(facts.len(), 1, "{root}");
        assert_eq!(
            field(&facts[0], "package").and_then(|v| v.as_str()),
            Some(cited),
            "{root}"
        );
        assert_eq!(
            facts[0].id,
            format!("dep:{cited}/pkg%3Acargo%2Fserde%401.0.0"),
            "{root}"
        );
    }
}

#[test]
fn a_root_naming_no_package_or_several_or_none_is_an_error() {
    let served = [
        package("pkg:cargo/demo#crates/demo"),
        package("pkg:cargo/demo#tools/demo"),
    ];
    let several = dependency_facts(&[bom("pkg:cargo/demo@0.1.0", vec![], None)], &served)
        .expect_err("two packages of one name, neither the plain id");
    assert!(several.contains("pkg:cargo/demo@0.1.0"), "{several}");
    assert!(several.contains("pkg:cargo/demo#crates/demo"), "{several}");
    let none = dependency_facts(&[bom("pkg:cargo/elsewhere@1.0.0", vec![], None)], &served)
        .expect_err("no package");
    assert!(none.contains("pkg:cargo/elsewhere@1.0.0"), "{none}");
    let mut purl_less = bom("pkg:cargo/demo", vec![], None);
    purl_less.root.purl = None;
    let no_purl = dependency_facts(&[purl_less], &served).expect_err("no purl");
    assert!(no_purl.contains("'root'"), "{no_purl}");
}

#[test]
fn the_exact_package_id_wins_and_a_purl_subpath_selects_among_several() {
    let serde = || {
        component(
            None,
            Some("pkg:cargo/serde@1.0.0"),
            "serde",
            Scope::Required,
        )
    };
    let cited = |root: &str, served: &[Fact]| -> String {
        let facts = dependency_facts(&[bom(root, vec![serde()], None)], served)
            .unwrap_or_else(|e| panic!("{root}: {e}"));
        field(&facts[0], "package")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let twins = [
        package("pkg:cargo/demo"),
        package("pkg:cargo/demo#crates/demo"),
    ];
    assert_eq!(cited("pkg:cargo/demo@0.1.0", &twins), "pkg:cargo/demo");
    assert_eq!(
        cited("pkg:cargo/demo@0.1.0#crates/demo", &twins),
        "pkg:cargo/demo#crates/demo"
    );
    assert_eq!(
        cited("pkg:cargo/demo@0.1.0#/crates/demo/", &twins),
        "pkg:cargo/demo#crates/demo"
    );
    // A subpath no package has falls back to the plain id.
    assert_eq!(cited("pkg:cargo/demo#sub/path", &twins), "pkg:cargo/demo");
    let spaced = [
        package("pkg:cargo/demo#a"),
        package("pkg:cargo/demo#my%20dir"),
    ];
    assert_eq!(
        cited("pkg:cargo/demo@1#my%20dir", &spaced),
        "pkg:cargo/demo#my%20dir"
    );
    assert_eq!(cited("pkg:cargo/demo@1#a", &spaced), "pkg:cargo/demo#a");
}

#[test]
fn a_root_error_shows_its_purl_and_name_bounded_and_escaped() {
    let served = [package("pkg:cargo/demo")];
    let hostile = format!("pkg:cargo/x\u{1b}]0;{}@1", "a".repeat(5_000));
    let error = dependency_facts(&[bom(&hostile, vec![], None)], &served).expect_err("unserved");
    assert!(
        !error.contains('\u{1b}') && error.contains("\\u{1b}"),
        "{error}"
    );
    assert!(error.len() < 400, "{} bytes", error.len());
    let mut purl_less = bom("pkg:cargo/demo", vec![], None);
    purl_less.root.purl = None;
    purl_less.root.name = "r\u{7}".repeat(1_000);
    let error = dependency_facts(&[purl_less], &served).expect_err("no purl");
    assert!(!error.contains('\u{7}') && error.len() < 400, "{error}");
}

#[test]
fn direct_is_true_false_or_absent_as_the_graph_states() {
    let served = [package("pkg:cargo/demo")];
    let components = vec![
        component(Some("a"), Some("pkg:cargo/a@1"), "a", Scope::Required),
        component(Some("b"), Some("pkg:cargo/b@1"), "b", Scope::Required),
        component(None, Some("pkg:cargo/c@1"), "c", Scope::Required),
    ];
    let directness = |direct: Option<&[&str]>| -> Vec<Option<bool>> {
        dependency_facts(
            &[bom("pkg:cargo/demo", components.clone(), direct)],
            &served,
        )
        .unwrap()
        .iter()
        .map(|f| field(f, "direct").and_then(|v| v.as_bool()))
        .collect()
    };
    assert_eq!(directness(Some(&["a"])), [Some(true), Some(false), None]);
    assert_eq!(directness(None), [None, None, None]);
}

#[test]
fn a_purl_less_component_is_named_by_group_name_and_version() {
    let served = [package("pkg:cargo/demo")];
    let mut lone = component(None, None, "tool", Scope::Optional);
    lone.group = Some("acme".to_string());
    let facts = dependency_facts(&[bom("pkg:cargo/demo", vec![lone], None)], &served).unwrap();
    assert_eq!(facts[0].id, "dep:pkg:cargo/demo/@acme%2Ftool%401.0.0");
    assert_eq!(
        field(&facts[0], "name").and_then(|v| v.as_str()),
        Some("acme/tool")
    );
    assert_eq!(
        field(&facts[0], "scope").and_then(|v| v.as_str()),
        Some("optional")
    );
    assert!(field(&facts[0], "purl").is_none());
}

#[test]
fn a_repeated_purl_merges_into_one_fact_of_the_widest_scope() {
    let served = [package("pkg:cargo/demo")];
    let components = vec![
        component(Some("x1"), Some("pkg:cargo/x@1"), "x", Scope::Excluded),
        component(
            Some("x2"),
            Some("pkg:cargo/x@1?arch=y"),
            "x",
            Scope::Optional,
        ),
        component(Some("x3"), Some("pkg:cargo/x@1"), "x", Scope::Excluded),
    ];
    let facts =
        dependency_facts(&[bom("pkg:cargo/demo", components, Some(&["x2"]))], &served).unwrap();
    assert_eq!(facts.len(), 1, "{facts:?}");
    let fact = &facts[0];
    assert_eq!(fact.id, "dep:pkg:cargo/demo/pkg%3Acargo%2Fx%401");
    assert_eq!(
        field(fact, "scope").and_then(|v| v.as_str()),
        Some("optional")
    );
    assert_eq!(field(fact, "direct").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        field(fact, "purl").and_then(|v| v.as_str()),
        Some("pkg:cargo/x@1")
    );
}

#[test]
fn the_scope_names_parse_back() {
    assert_eq!(Scope::NAMES, ["required", "optional", "excluded"]);
    for name in Scope::NAMES {
        assert_eq!(Scope::parse(name).map(Scope::as_str), Some(name));
    }
    assert_eq!(Scope::parse("mandatory"), None);
}
