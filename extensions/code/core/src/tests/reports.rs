//! The report facts' kinds, and the `reports` pass: a report entry that names no item (W104).

use specforge_code_facts::{
    api_facts, coverage_facts, dependency_facts, finding_facts, vulnerability_facts, ApiEntry,
    ApiKind, Bom, Branch, Component, CoverageFile, Function, ItemKind, Level, OsvPackage,
    OsvRecord, OsvReport, OsvSource, Owner, SarifResult, Scheme, Scope, Severity, Visibility,
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

/// A `static_finding` fact stating `rule` and `level`, citing `item` or `package` when given.
fn finding(rule: &str, level: &str, item: Option<&str>, package: Option<&str>) -> PassEntity {
    let mut fact = entity(
        &format!("sarif:clippy/{rule}#src%2Flib.rs:3"),
        "static_finding",
    );
    let stated = [
        ("tool", Some("clippy")),
        ("rule", Some(rule)),
        ("level", Some(level)),
        ("item", item),
        ("package", package),
    ];
    for (name, value) in stated {
        if let Some(value) = value {
            fact.fields.insert(name.to_string(), value.to_string());
        }
    }
    fact
}

#[test]
fn a_finding_with_an_unknown_rule_or_level_is_one_w105_saying_which() {
    let package = Some("pkg:cargo/demo");
    for (rule, level, told) in [
        ("unknown", "warning", "rule"),
        ("r", "unknown", "level"),
        ("unknown", "unknown", "rule and level"),
    ] {
        let found = reported(vec![finding(rule, level, None, package)]);
        assert_eq!(found.len(), 1, "{found:?}");
        let w105 = &found[0];
        assert_eq!(
            (w105.code.as_str(), w105.severity, w105.entity.is_some()),
            ("W105", PassSeverity::Warning, true)
        );
        assert!(
            w105.message.contains(&format!("states no {told}:")),
            "{}",
            w105.message
        );
    }
    let known = reported(vec![finding("r", "none", None, package)]);
    assert!(known.is_empty(), "{known:?}");
}

#[test]
fn a_finding_citing_neither_item_nor_package_is_w104() {
    let found = reported(vec![finding("r", "warning", None, None)]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, "W104");
    assert!(
        found[0].message.contains("sarif:clippy/r"),
        "{}",
        found[0].message
    );
    let cited = reported(vec![
        finding("r", "warning", None, Some("pkg:cargo/demo")),
        finding("r", "warning", Some("rust:demo::add"), None),
    ]);
    assert!(cited.is_empty(), "{cited:?}");
    let both = reported(vec![finding("unknown", "warning", None, None)]);
    let codes: Vec<&str> = both.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["W104", "W105"]);
}

#[test]
fn static_finding_is_collected_and_found_in_an_item_or_a_package() {
    let declared = declaration();
    assert!(declared.entities.iter().all(|k| k.name != "finding"));
    let finding = kind(&declared, "static_finding");
    assert_eq!(
        (finding.writer.as_deref(), &finding.schemes[..]),
        (Some("collector"), &strings(&["sarif"])[..])
    );
    let field = |name: &str| finding.fields.iter().find(|f| f.name == name).unwrap();
    for (name, target, edge) in [
        ("item", "item", "found_in"),
        ("package", "package", "found_in_package"),
    ] {
        let reference = field(name);
        assert_eq!(
            (
                reference.field_type.as_str(),
                reference.target_kind.as_deref(),
                reference.edge.as_deref()
            ),
            ("reference", Some(target), Some(edge))
        );
        let drawn = declared.edges.iter().find(|e| e.label == edge).unwrap();
        assert_eq!(
            (drawn.source_kind.as_deref(), drawn.target_kind.as_deref()),
            (Some("static_finding"), Some(target))
        );
    }
    assert_eq!(field("level").enum_values, strings(&Level::NAMES));
    assert_eq!(field("line").field_type, "integer");
    for text in ["tool", "rule", "message", "file"] {
        assert_eq!(field(text).field_type, "string", "{text}");
    }
}

#[test]
fn every_field_a_static_finding_fact_is_minted_with_is_declared() {
    let served = [
        Fact::new("item", "rust:demo::add").at(FactLocation {
            file: "src/lib.rs".to_string(),
            start_line: 1,
            start_col: 1,
            end_line: 5,
            end_col: 2,
        }),
        Fact::new("package", "pkg:cargo/demo").field("path", ""),
    ];
    let result = |line| SarifResult {
        tool: "clippy".to_string(),
        rule: "clippy::unwrap_used".to_string(),
        level: Level::Warning,
        message: "used `unwrap()`".to_string(),
        file: Some("src/lib.rs".to_string()),
        line: Some(line),
        end_line: None,
    };
    let minted = finding_facts(&[result(2), result(9)], &served);
    assert!(minted[0].fields.contains_key("item"), "{minted:?}");
    assert!(minted[1].fields.contains_key("package"), "{minted:?}");
    let declared = declaration();
    let fields: Vec<&str> = kind(&declared, "static_finding")
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    for fact in &minted {
        assert_eq!(fact.kind, "static_finding");
        for name in fact.fields.keys() {
            assert!(fields.contains(&name.as_str()), "{name} of {}", fact.id);
        }
    }
}

/// A `dependency` fact of `name` in `pkg:cargo/demo`, with `purl` when given.
fn dependency(name: &str, purl: Option<&str>) -> PassEntity {
    let mut fact = entity(&format!("dep:pkg:cargo/demo/@{name}"), "dependency");
    let stated = [
        ("package", Some("pkg:cargo/demo")),
        ("name", Some(name)),
        ("scope", Some("required")),
        ("purl", purl),
    ];
    for (field, value) in stated {
        if let Some(value) = value {
            fact.fields.insert(field.to_string(), value.to_string());
        }
    }
    fact
}

#[test]
fn a_dependency_with_no_purl_is_one_w106_naming_it() {
    let found = reported(vec![dependency("left-pad", None)]);
    assert_eq!(found.len(), 1, "{found:?}");
    let w106 = &found[0];
    assert_eq!(
        (w106.code.as_str(), w106.severity, w106.entity.as_deref()),
        (
            "W106",
            PassSeverity::Warning,
            Some("dep:pkg:cargo/demo/@left-pad")
        )
    );
    assert!(
        w106.message.contains("dep:pkg:cargo/demo/@left-pad"),
        "{}",
        w106.message
    );
    assert!(w106.message.contains("`left-pad`"), "{}", w106.message);
    let with_purl = reported(vec![dependency("serde", Some("pkg:cargo/serde@1.0.0"))]);
    assert!(with_purl.is_empty(), "{with_purl:?}");
}

#[test]
fn dependency_is_collected_and_a_dependency_of_a_package() {
    let declared = declaration();
    let dependency = kind(&declared, "dependency");
    assert_eq!(
        (dependency.writer.as_deref(), &dependency.schemes[..]),
        (Some("collector"), &strings(&["dep"])[..])
    );
    let field = |name: &str| dependency.fields.iter().find(|f| f.name == name).unwrap();
    let package = field("package");
    assert_eq!(
        (
            package.field_type.as_str(),
            package.target_kind.as_deref(),
            package.edge.as_deref()
        ),
        ("reference", Some("package"), Some("dependency_of"))
    );
    assert_eq!(field("scope").enum_values, strings(&Scope::NAMES));
    assert_eq!(field("direct").field_type, "bool");
    for text in ["purl", "name", "version"] {
        assert_eq!(field(text).field_type, "string", "{text}");
    }
    let drawn = declared
        .edges
        .iter()
        .find(|e| e.label == "dependency_of")
        .unwrap();
    assert_eq!(
        (drawn.source_kind.as_deref(), drawn.target_kind.as_deref()),
        (Some("dependency"), Some("package"))
    );
}

#[test]
fn every_field_a_dependency_fact_is_minted_with_is_declared() {
    let component = |bom_ref: &str, purl: Option<&str>| Component {
        bom_ref: Some(bom_ref.to_string()),
        purl: purl.map(str::to_string),
        group: None,
        name: bom_ref.to_string(),
        version: Some("1.0.0".to_string()),
        scope: Scope::Optional,
    };
    let bom = Bom {
        root: component("demo", Some("pkg:cargo/demo@0.1.0")),
        components: vec![
            component("serde", Some("pkg:cargo/serde@1.0.0")),
            component("tool", None),
        ],
        direct: Some(vec!["serde".to_string()]),
    };
    let served = [Fact::new("package", "pkg:cargo/demo").field("path", "")];
    let minted = dependency_facts(&[bom], &served).expect("the root is served");
    assert_eq!(minted.len(), 2);
    let declared = declaration();
    let fields: Vec<&str> = kind(&declared, "dependency")
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    for fact in &minted {
        assert_eq!(fact.kind, "dependency");
        assert!(fact.fields.contains_key("direct"), "{fact:?}");
        for name in fact.fields.keys() {
            assert!(fields.contains(&name.as_str()), "{name} of {}", fact.id);
        }
    }
}

/// A `vulnerability` fact of `osv_id` in `pkg:cargo/demo`, of `severity`.
fn vulnerability(osv_id: &str, severity: &str) -> PassEntity {
    let mut fact = entity(&format!("vuln:pkg:cargo/demo/{osv_id}"), "vulnerability");
    for (field, value) in [
        ("package", "pkg:cargo/demo"),
        ("osv_id", osv_id),
        ("severity", severity),
    ] {
        fact.fields.insert(field.to_string(), value.to_string());
    }
    fact
}

#[test]
fn a_vulnerability_of_unknown_severity_is_one_w107_naming_it() {
    let found = reported(vec![vulnerability("RUSTSEC-2099-0002", "unknown")]);
    assert_eq!(found.len(), 1, "{found:?}");
    let w107 = &found[0];
    assert_eq!(
        (w107.code.as_str(), w107.severity, w107.entity.as_deref()),
        (
            "W107",
            PassSeverity::Warning,
            Some("vuln:pkg:cargo/demo/RUSTSEC-2099-0002")
        )
    );
    assert!(
        w107.message
            .contains("vuln:pkg:cargo/demo/RUSTSEC-2099-0002"),
        "{}",
        w107.message
    );
    assert!(
        w107.message.contains("`RUSTSEC-2099-0002`"),
        "{}",
        w107.message
    );
    for severity in Severity::NAMES.iter().filter(|name| **name != "unknown") {
        let rated = reported(vec![vulnerability("GHSA-x", severity)]);
        assert!(rated.is_empty(), "{severity}: {rated:?}");
    }
}

#[test]
fn vulnerability_is_collected_and_affects_a_package() {
    let declared = declaration();
    let vulnerability = kind(&declared, "vulnerability");
    assert_eq!(
        (vulnerability.writer.as_deref(), &vulnerability.schemes[..]),
        (Some("collector"), &strings(&["vuln"])[..])
    );
    let field = |name: &str| {
        vulnerability
            .fields
            .iter()
            .find(|f| f.name == name)
            .unwrap()
    };
    let package = field("package");
    assert_eq!(
        (
            package.field_type.as_str(),
            package.target_kind.as_deref(),
            package.edge.as_deref()
        ),
        ("reference", Some("package"), Some("affects"))
    );
    assert_eq!(field("severity").enum_values, strings(&Severity::NAMES));
    for list in ["aliases", "purl", "version"] {
        assert_eq!(field(list).field_type, "string_list", "{list}");
    }
    for text in ["osv_id", "summary"] {
        assert_eq!(field(text).field_type, "string", "{text}");
    }
    let drawn = declared
        .edges
        .iter()
        .find(|e| e.label == "affects")
        .unwrap();
    assert_eq!(
        (drawn.source_kind.as_deref(), drawn.target_kind.as_deref()),
        (Some("vulnerability"), Some("package"))
    );
}

#[test]
fn every_field_a_vulnerability_fact_is_minted_with_is_declared() {
    let record = OsvRecord {
        id: "GHSA-x".to_string(),
        aliases: vec!["CVE-1".to_string()],
        summary: Some("a flaw".to_string()),
        label: Some("HIGH".to_string()),
        scores: Vec::new(),
        purl: Some("pkg:cargo/x".to_string()),
    };
    let report = OsvReport {
        sources: vec![OsvSource {
            lockfile: "Cargo.lock".to_string(),
            packages: vec![OsvPackage {
                name: "x".to_string(),
                version: "1.0.0".to_string(),
                ecosystem: "crates.io".to_string(),
                records: vec![record],
            }],
        }],
    };
    let served = [Fact::new("package", "pkg:cargo/demo").field("path", "")];
    let minted = vulnerability_facts(&report, &served).expect("the lockfile is served");
    assert_eq!(minted.len(), 1);
    let declared = declaration();
    let fields: Vec<&str> = kind(&declared, "vulnerability")
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    for fact in &minted {
        assert_eq!(fact.kind, "vulnerability");
        assert!(fact.fields.contains_key("summary"), "{fact:?}");
        for name in fact.fields.keys() {
            assert!(fields.contains(&name.as_str()), "{name} of {}", fact.id);
        }
    }
}
