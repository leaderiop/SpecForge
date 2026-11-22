//! The report facts' kinds, and the `reports` pass: a report entry that names no item (W104).

use specforge_code_facts::{coverage_facts, Branch, CoverageFile, Function};

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
