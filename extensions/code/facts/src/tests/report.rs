//! The report kinds, safe id parts and repeat suffixes.

use crate::*;

#[test]
fn an_id_part_encodes_every_delimiter_whitespace_and_control() {
    assert_eq!(
        id_part("a/b#c:d~e@f?g h%\u{1b}"),
        "a%2Fb%23c%3Ad%7Ee%40f%3Fg%20h%25%1B"
    );
    assert_eq!(id_part("Az09.-_é"), "Az09.-_é");
    assert_eq!(id_part("tab\tnl\n\u{a0}"), "tab%09nl%0A%C2%A0");
}

#[test]
fn shown_text_is_bounded_and_escapes_control_characters() {
    assert_eq!(shown("a\u{1b}]0;x\u{7}\"é"), "a\\u{1b}]0;x\\u{7}\\\"é");
    let long = "x".repeat(10_000);
    assert_eq!(shown(&long), format!("{}…", "x".repeat(80)));
    assert_eq!(shown(&"y".repeat(80)), "y".repeat(80));
    assert_eq!(
        shown_json(&serde_json::json!("2.0\u{1b}")),
        "\"2.0\\u{1b}\""
    );
    assert_eq!(shown_json(&serde_json::json!(2)), "2");
    assert!(shown_json(&serde_json::json!([long])).len() < 100);
}

#[test]
fn a_repeated_id_gets_a_numbered_suffix() {
    let mut repeats = Repeats::default();
    let ids: Vec<String> = ["x", "x", "y", "x"]
        .into_iter()
        .map(|base| repeats.id(base.to_string()))
        .collect();
    assert_eq!(ids, ["x", "x~2", "y", "x~3"]);
}

/// Each vocabulary enum's name comes from an exhaustive match, usable in a const: a variant added without a name fails
/// to compile, never reads as a default name or the empty string.
const NAMED: [&str; 6] = [
    Scope::Excluded.as_str(),
    Severity::Unknown.as_str(),
    Level::None.as_str(),
    ApiKind::ExternCrate.as_str(),
    ApiKind::Item(ItemKind::Namespace).as_str(),
    Visibility::Restricted.as_str(),
];

#[test]
fn every_vocabulary_variant_is_named_and_listed_once() {
    assert_eq!(
        NAMED,
        [
            "excluded",
            "unknown",
            "none",
            "extern_crate",
            "namespace",
            "restricted"
        ]
    );
    // The match lists each variant: a new one fails to compile here until it is checked as listed below.
    let scope = |s: Scope| match s {
        Scope::Required | Scope::Optional | Scope::Excluded => s.as_str(),
    };
    let severity = |s: Severity| match s {
        Severity::Critical
        | Severity::High
        | Severity::Medium
        | Severity::Low
        | Severity::Unknown => s.as_str(),
    };
    let level = |l: Level| match l {
        Level::Error | Level::Warning | Level::Note | Level::None | Level::Unknown => l.as_str(),
    };
    let visibility = |v: Visibility| match v {
        Visibility::Private | Visibility::Restricted | Visibility::Crate | Visibility::Public => {
            v.as_str()
        }
    };
    for name in [Scope::Required, Scope::Optional, Scope::Excluded].map(scope) {
        assert!(Scope::NAMES.contains(&name), "{name}");
    }
    for name in [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
        Severity::Unknown,
    ]
    .map(severity)
    {
        assert!(Severity::NAMES.contains(&name), "{name}");
    }
    for name in [
        Level::Error,
        Level::Warning,
        Level::Note,
        Level::None,
        Level::Unknown,
    ]
    .map(level)
    {
        assert!(Level::NAMES.contains(&name), "{name}");
    }
    for name in [
        Visibility::Private,
        Visibility::Restricted,
        Visibility::Crate,
        Visibility::Public,
    ]
    .map(visibility)
    {
        assert!(Visibility::NAMES.contains(&name), "{name}");
    }
    let item = |k: ItemKind| match k {
        ItemKind::Module
        | ItemKind::Function
        | ItemKind::Method
        | ItemKind::Field
        | ItemKind::Struct
        | ItemKind::Enum
        | ItemKind::Union
        | ItemKind::Trait
        | ItemKind::Type
        | ItemKind::Const
        | ItemKind::Static
        | ItemKind::Macro
        | ItemKind::Class
        | ItemKind::Interface
        | ItemKind::Variable
        | ItemKind::Namespace => ItemKind::ALL.contains(&k),
    };
    assert!(ItemKind::ALL.into_iter().all(item));
    for kind in ItemKind::ALL {
        assert!(ApiKind::NAMES.contains(&ApiKind::Item(kind).as_str()));
    }
    for kind in [
        ApiKind::Variant,
        ApiKind::Reexport,
        ApiKind::ExternCrate,
        ApiKind::Other,
    ] {
        assert!(ApiKind::NAMES.contains(&kind.as_str()), "{kind:?}");
    }
    // No two variants share a name.
    for names in [
        &Scope::NAMES[..],
        &Severity::NAMES[..],
        &Level::NAMES[..],
        &Visibility::NAMES[..],
        &ItemKind::NAMES[..],
        &ApiKind::NAMES[..],
    ] {
        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "{names:?}");
        assert!(!unique.contains(""), "{names:?}");
    }
}

#[test]
fn the_report_kinds_have_their_schemes() {
    let named: Vec<(&str, &str)> = [
        COVERAGE,
        API_ITEM,
        STATIC_FINDING,
        DEPENDENCY,
        VULNERABILITY,
    ]
    .iter()
    .map(|k| (k.kind, k.scheme))
    .collect();
    assert_eq!(
        named,
        [
            ("coverage", "cov"),
            ("api_item", "api"),
            ("static_finding", "sarif"),
            ("dependency", "dep"),
            ("vulnerability", "vuln"),
        ]
    );
    assert_eq!(UNKNOWN, "unknown");
}
