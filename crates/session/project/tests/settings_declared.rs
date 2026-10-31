//! The registry checks every settings item against K3's field forms and K10's code book (ADR 0288).

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::*;
use specforge_registry::build_registries;

const EXT: &str = "@t/s";

/// `@t/s` declaring the codes W901 and W902 and `items` as its settings category (a peer `@t/p` too).
fn declare(items: Value) -> specforge_registry::RegistryBuild {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    for code in ["W901", "W902", "E903"] {
        c.code(code, |d| {
            d.title("t").explanation("e");
        });
    }
    c.peer("@t/p", "*");
    c.raw_category(DeclaredCategory::Settings, items);
    let mut peer = ContributionsBuilder::new(ExtensionMeta::new("@t/p", "1.0.0"));
    peer.setting("mode", |s| {
        s.enum_values(&["a", "b"]);
    });
    peer.setting("count", |s| {
        s.integer();
    });
    build_registries(vec![peer.declaration(), c.declaration()])
}

/// The W019 and W021 the settings of `@t/s` caused, as text.
fn faults(build: &specforge_registry::RegistryBuild) -> Vec<String> {
    build
        .diagnostics()
        .filter(|d| matches!(d.code.as_str(), "W019" | "W021"))
        .map(|d| d.message.clone())
        .collect()
}

fn setting(name: &str, extra: Value) -> Value {
    let mut item = json!({"kind": "setting", "name": name, "field_type": "string"});
    for (key, value) in extra.as_object().unwrap() {
        item[key] = value.clone();
    }
    item
}

fn base() -> Vec<Value> {
    vec![
        setting("flag", json!({"field_type": "bool", "gates": ["W901"]})),
        setting(
            "profile",
            json!({"field_type": "enum", "enum_values": ["compat", "strict"]}),
        ),
        setting(
            "level",
            json!({"field_type": "integer", "minimum": 1, "maximum": 3}),
        ),
        setting("models.*", json!({})),
    ]
}

/// The messages of `items` added to the base, which holds four good settings.
fn faults_of(items: Vec<Value>) -> (Vec<String>, Vec<String>) {
    let mut all = base();
    all.extend(items);
    let build = declare(Value::Array(all));
    let kept = build
        .settings()
        .of(EXT)
        .unwrap()
        .settings
        .iter()
        .map(|s| s.descriptor.name.clone())
        .collect();
    (faults(&build), kept)
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "the registry refuses a malformed setting, preset or secret with W021 and keeps the rest"
)]
fn the_registry_refuses_a_malformed_setting() {
    let (none, kept) = faults_of(vec![]);
    assert!(none.is_empty(), "{none:?}");
    assert_eq!(kept, ["flag", "profile", "level", "models.*"]);

    // (item, a word of the fault, whether the setting is kept)
    let cases: Vec<(Value, &str, Option<&str>)> = vec![
        (setting("a..b", json!({})), "segment", None),
        (setting("a b", json!({})), "segment", None),
        (setting("flag", json!({})), "twice", None),
        (
            setting("models.default", json!({})),
            "",
            Some("models.default"),
        ),
        (setting("*.planning", json!({})), "overlaps", None),
        (
            setting("fixed", json!({"keys": [["a"]]})),
            "not a family",
            Some("fixed"),
        ),
        (
            setting("m.*", json!({"keys": [["a"], ["b"]]})),
            "key lists",
            Some("m.*"),
        ),
        (
            setting("odd", json!({"field_type": "mystery"})),
            "unknown field type",
            None,
        ),
        (
            setting("ref", json!({"field_type": "reference"})),
            "before any graph",
            None,
        ),
        (
            setting(
                "range",
                json!({"field_type": "integer", "minimum": 5, "maximum": 1}),
            ),
            "minimum 5 above maximum 1",
            Some("range"),
        ),
        (
            setting("dflt", json!({"field_type": "integer", "default": "x"})),
            "default its type does not admit",
            Some("dflt"),
        ),
        (setting("nul", json!({"default": null})), "", Some("nul")),
        (
            setting("gates", json!({"gates": ["W901"]})),
            "only a bool setting gates",
            Some("gates"),
        ),
        (
            setting("fam.*", json!({"field_type": "bool", "gates": ["W901"]})),
            "only a setting named whole gates",
            Some("fam.*"),
        ),
        (
            setting("g2", json!({"field_type": "bool", "gates": ["W999"]})),
            "do not catalog",
            Some("g2"),
        ),
        (
            setting("noenum", json!({"field_type": "enum"})),
            "declares no values",
            Some("noenum"),
        ),
        (
            setting("kk.*", json!({"keys": [["a b"]]})),
            "not a segment",
            None,
        ),
        (
            setting("kk2.*", json!({"keys": [["a", "*"]]})),
            "not a segment",
            None,
        ),
        (
            setting("fd.*", json!({"default": "x"})),
            "declares a default",
            Some("fd.*"),
        ),
        (json!({"kind": "mystery"}), "mystery", None),
        (
            json!({"kind": "secret", "name": "k", "env": "bad-env"}),
            "no environment variable name",
            None,
        ),
        (
            json!({"kind": "secret", "name": "flag", "env": "FLAG"}),
            "also a setting's name",
            None,
        ),
        (
            json!({"kind": "secret", "name": "ok", "env": "OK_KEY"}),
            "",
            None,
        ),
    ];
    for (item, word, keeps) in cases {
        let (faults, kept) = faults_of(vec![item.clone()]);
        if !word.is_empty() {
            assert_eq!(faults.len(), 1, "{item}: {faults:?}");
            assert!(faults[0].contains(word), "{item}: {faults:?}");
        }
        let name = item["name"].as_str().unwrap_or_default();
        match keeps {
            Some(expected) => assert!(kept.iter().any(|k| k == expected), "{item}: {kept:?}"),
            None if item["kind"] == json!("setting") => {
                let twice = name == "flag";
                assert!(twice || !kept.iter().any(|k| k == name), "{item}: {kept:?}");
            }
            None => {}
        }
        // Only the item is dropped: the base stays.
        for good in ["flag", "profile", "level"] {
            assert!(kept.iter().any(|k| k == good), "{item}: lost {good}");
        }
    }

    // A feature flag and a setting of one name: the flag wins, one W021.
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.feature_flag("beta", true, "Try it");
    let mut declared = c.declaration();
    declared
        .feature_flags
        .push(specforge_protocol_types::FeatureFlagDescriptor {
            name: "alpha".into(),
            description: None,
            default_enabled: false,
        });
    declared.settings.push(declared.settings[0].clone());
    let build = build_registries(vec![declared]);
    assert_eq!(faults(&build).len(), 1, "{:?}", faults(&build));
    let registered = &build.settings().of(EXT).unwrap().settings;
    assert_eq!(registered.len(), 2, "{registered:?}");
    assert_eq!(registered[0].descriptor.name, "alpha");

    // Presets.
    let preset =
        |source: &str, values: Value| json!({"kind": "preset", "source": source, "values": values});
    let strict = |settings: Value, severities: Value| json!({"strict": {"settings": settings, "severities": severities}});
    let good = preset(
        "profile",
        strict(json!({"flag": true}), json!({"W902": "error"})),
    );
    let (ok, _) = faults_of(vec![good.clone()]);
    assert!(ok.is_empty(), "{ok:?}");
    let preset_cases = vec![
        (
            preset("nothing", strict(json!({}), json!({}))),
            "not a setting the extension declares",
        ),
        (preset("level", json!({})), "only an enum or bool"),
        (preset("@t/q:mode", json!({})), "not a declared peer"),
        (
            preset("@t/p:count", json!({})),
            "not an enum or bool setting of",
        ),
        (
            preset("@t/p:nothing", json!({})),
            "not an enum or bool setting of",
        ),
        (
            preset("profile", json!({"odd": {}})),
            "not one of compat, strict",
        ),
        (
            preset("profile", strict(json!({"nothing": 1}), json!({}))),
            "does not declare",
        ),
        (
            preset("profile", strict(json!({"level": "x"}), json!({}))),
            "its type does not admit",
        ),
        (
            preset("profile", strict(json!({"profile": "strict"}), json!({}))),
            "a preset source",
        ),
        (
            preset("profile", strict(json!({}), json!({"W999": "error"}))),
            "do not catalog",
        ),
        (
            preset("profile", strict(json!({}), json!({"W902": "warning"}))),
            "raises, never lowers",
        ),
        (
            preset("profile", strict(json!({}), json!({"W902": "info"}))),
            "raises, never lowers",
        ),
    ];
    for (item, word) in preset_cases {
        let (faults, _) = faults_of(vec![item.clone()]);
        assert_eq!(faults.len(), 1, "{item}: {faults:?}");
        assert!(faults[0].contains(word), "{item}: {faults:?}");
    }
    // Two presets that preset one setting: the later one loses it.
    let other = preset("@t/p:mode", json!({"a": {"settings": {"flag": false}}}));
    let (twice, _) = faults_of(vec![good, other]);
    assert_eq!(twice.len(), 1, "{twice:?}");
    assert!(twice[0].contains("two presets preset 'flag'"), "{twice:?}");
    // A peer's source is followed.
    let follow = preset("@t/p:mode", json!({"a": {"severities": {"W901": "error"}}}));
    let (followed, _) = faults_of(vec![follow]);
    assert!(followed.is_empty(), "{followed:?}");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "the registry refuses a malformed setting, preset or secret with W021 and keeps the rest"
)]
fn a_family_keeps_no_default() {
    let mut all = base();
    all.push(setting("fd.*", json!({"default": "x"})));
    all.push(setting("whole", json!({"default": "y"})));
    let build = declare(Value::Array(all));
    let declared = build.settings().of(EXT).unwrap();
    let default_of = |name: &str| {
        declared
            .settings
            .iter()
            .find(|s| s.descriptor.name == name)
            .unwrap()
            .descriptor
            .default
            .clone()
    };
    assert_eq!(default_of("fd.*"), None, "no key would receive it");
    assert_eq!(default_of("whole"), Some(json!("y")));
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a bool setting gates codes while it is on"
)]
fn gates_when_on_on_a_string_setting_is_w021() {
    let (found, _) = faults_of(vec![setting("txt", json!({"gates_when_on": ["W901"]}))]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("gates_when_on"), "{found:?}");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a bool setting gates codes while it is on"
)]
fn a_code_gated_both_ways_is_w021() {
    let both = setting(
        "both",
        json!({"field_type": "bool", "gates": ["W901"], "gates_when_on": ["W901"]}),
    );
    let (found, _) = faults_of(vec![both]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("W901"), "{found:?}");
}
