//! The report facts' kinds, and the `reports` pass: a report entry that names no item (W104).

use specforge_code_facts::{
    api_facts, coverage_facts, ApiEntry, ApiKind, Branch, CoverageFile, Function, ItemKind, Owner,
    Scheme, Visibility,
};

use super::*;

/// A `coverage` fact of `file`, citing `item` when there is one.
fn coverage(id: &str, file: &str, item: Option<&str>) -> PassEntity {
    let mut fact = entity(id, "coverage");
    fact.fields.insert("file".to_string(), file.to_string());
    if let Some(item) = item {
        fact.fields.insert("item".to_string(), item.to_string());
    }
    fact
}

fn reported(entities: Vec<PassEntity>) -> Vec<PassDiagnostic> {
    crate::reports::reports(&PassInput {
        entities,
        ..PassInput::default()
    })
}

#[test]
fn a_coverage_naming_no_item_is_w104() {
    let found = reported(vec![coverage("cov:@src%2Fbuild.rs", "src/build.rs", None)]);
    assert_eq!(found.len(), 1, "{found:?}");
    let w104 = &found[0];
    assert_eq!(
        (w104.code.as_str(), w104.severity, w104.entity.as_deref()),
        ("W104", PassSeverity::Warning, Some("cov:@src%2Fbuild.rs"))
    );
    assert!(
        w104.message.contains("cov:@src%2Fbuild.rs"),
        "{}",
        w104.message
    );
    assert!(w104.message.contains("src/build.rs"), "{}", w104.message);
}

#[test]
fn a_coverage_citing_an_item_or_another_kind_is_not_w104() {
    let mut pending = coverage("cov:@src%2Fa.rs", "src/a.rs", None);
    pending.pending = true;
    let found = reported(vec![
        coverage("cov:rust:demo::add", "src/lib.rs", Some("rust:demo::add")),
        entity("rust:demo::add", "item"),
        entity("t1", "truth"),
        pending,
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn coverage_is_collected_and_covers_an_item() {
    let declared = declaration();
    let coverage = kind(&declared, "coverage");
    assert_eq!(
        (coverage.writer.as_deref(), &coverage.schemes[..]),
        (Some("collector"), &strings(&["cov"])[..])
    );
    let item = coverage.fields.iter().find(|f| f.name == "item").unwrap();
    assert_eq!(
        (
            item.field_type.as_str(),
            item.target_kind.as_deref(),
            item.edge.as_deref()
        ),
        ("reference", Some("item"), Some("covers"))
    );
    for count in [
        "functions_found",
        "functions_hit",
        "lines_found",
        "lines_hit",
        "branches_found",
        "branches_hit",
    ] {
        let field = coverage.fields.iter().find(|f| f.name == count).unwrap();
        assert_eq!(field.field_type, "integer", "{count}");
    }
    let covers = declared.edges.iter().find(|e| e.label == "covers").unwrap();
    assert_eq!(
        (covers.source_kind.as_deref(), covers.target_kind.as_deref()),
        (Some("coverage"), Some("item"))
    );
}

#[test]
fn every_field_a_coverage_fact_is_minted_with_is_declared() {
    let served = [Fact::new("item", "rust:demo::add").at(FactLocation {
        file: "src/lib.rs".to_string(),
        start_line: 1,
        start_col: 1,
        end_line: 5,
        end_col: 2,
    })];
    let files = [CoverageFile {
        file: "src/lib.rs".to_string(),
        functions: vec![Function {
            line: 1,
            name: "add".to_string(),
            hits: 1,
        }],
        lines: vec![(2, 1), (9, 0)],
        branches: vec![Branch {
            line: 2,
            block: "0".to_string(),
            branch: "0".to_string(),
            taken: None,
        }],
    }];
    let minted = coverage_facts(&files, &served);
    assert_eq!(minted.len(), 2, "one citing the item, one of the file");
    let declared = declaration();
    let fields: Vec<&str> = kind(&declared, "coverage")
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    for fact in &minted {
        assert_eq!(fact.kind, "coverage");
        for name in fact.fields.keys() {
            assert!(fields.contains(&name.as_str()), "{name} of {}", fact.id);
        }
    }
}

/// An `api_item` fact of `kind` at `path`, citing `item` when there is one.
fn api_item(id: &str, kind: &str, item: Option<&str>) -> PassEntity {
    let mut fact = entity(id, "api_item");
    let path = id.strip_prefix("api:rust:").unwrap_or(id);
    fact.fields.insert("path".to_string(), path.to_string());
    fact.fields.insert("kind".to_string(), kind.to_string());
    if let Some(item) = item {
        fact.fields.insert("item".to_string(), item.to_string());
    }
    fact
}

#[test]
fn an_item_capable_api_item_naming_no_item_is_w104() {
    let found = reported(vec![api_item(
        "api:rust:demo::from_macro",
        "function",
        None,
    )]);
    assert_eq!(found.len(), 1, "{found:?}");
    let w104 = &found[0];
    assert_eq!(
        (w104.code.as_str(), w104.severity, w104.entity.as_deref()),
        (
            "W104",
            PassSeverity::Warning,
            Some("api:rust:demo::from_macro")
        )
    );
    assert!(
        w104.message.contains("api:rust:demo::from_macro"),
        "{}",
        w104.message
    );
    assert!(
        w104.message.contains("demo::from_macro"),
        "{}",
        w104.message
    );
}

#[test]
fn an_api_item_no_item_can_hold_or_citing_one_is_not_w104() {
    let mut entities: Vec<PassEntity> = ["variant", "reexport", "extern_crate", "other"]
        .iter()
        .map(|kind| api_item(&format!("api:rust:demo::{kind}"), kind, None))
        .collect();
    entities.push(api_item(
        "api:rust:demo::add",
        "function",
        Some("rust:demo::add"),
    ));
    // The crate root, which no item is.
    entities.push(api_item("api:rust:demo", "module", None));
    let found = reported(entities);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn api_item_is_collected_and_is_the_api_of_an_item() {
    let declared = declaration();
    let api = kind(&declared, "api_item");
    assert_eq!(
        (api.writer.as_deref(), &api.schemes[..]),
        (Some("collector"), &strings(&["api"])[..])
    );
    let field = |name: &str| api.fields.iter().find(|f| f.name == name).unwrap();
    let item = field("item");
    assert_eq!(
        (
            item.field_type.as_str(),
            item.target_kind.as_deref(),
            item.edge.as_deref()
        ),
        ("reference", Some("item"), Some("api_of"))
    );
    assert_eq!(field("kind").enum_values, strings(&ApiKind::NAMES));
    assert_eq!(field("visibility").enum_values, strings(&Visibility::NAMES));
    for text in ["path", "signature", "tag"] {
        assert_eq!(field(text).field_type, "string", "{text}");
    }
    let api_of = declared.edges.iter().find(|e| e.label == "api_of").unwrap();
    assert_eq!(
        (api_of.source_kind.as_deref(), api_of.target_kind.as_deref()),
        (Some("api_item"), Some("item"))
    );
}

#[test]
fn every_field_an_api_item_fact_is_minted_with_is_declared() {
    let entry = |scheme, container: &str, path: &[&str], owner, kind, tag: Option<&str>| ApiEntry {
        scheme,
        container: container.to_string(),
        path: strings(path),
        owner,
        kind,
        visibility: Visibility::Public,
        signature: "as written".to_string(),
        tag: tag.map(str::to_string),
    };
    let entries = [
        entry(
            Scheme::Rust,
            "demo",
            &["demo", "S", "fmt"],
            Some(Owner {
                self_ty: "demo::S".to_string(),
                trait_ty: Some("core::fmt::Debug".to_string()),
            }),
            ApiKind::Item(ItemKind::Method),
            None,
        ),
        entry(
            Scheme::Js,
            "@demo/widget",
            &["Widget"],
            None,
            ApiKind::Item(ItemKind::Class),
            Some("Public"),
        ),
    ];
    let served = [
        Fact::new("item", "rust:demo::<S as Debug>::fmt"),
        Fact::new("item", "js:@demo/widget/src/widget.ts::Widget"),
    ];
    let minted = api_facts(&entries, &served);
    assert_eq!(minted.len(), 2);
    let declared = declaration();
    let fields: Vec<&str> = kind(&declared, "api_item")
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    for fact in &minted {
        assert_eq!(fact.kind, "api_item");
        assert!(fact.fields.contains_key("item"), "{fact:?}");
        for name in fact.fields.keys() {
            assert!(fields.contains(&name.as_str()), "{name} of {}", fact.id);
        }
    }
}
