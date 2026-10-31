//! A project that sets no setting reads, calls and reports as it did before ADR 0288: no settings
//! diagnostic, and no call input holds a `settings` key.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::{Diagnostic, Severity};
use specforge_project::testing::{
    Assembled, follower_extension, settings_config, settings_declarations, settings_extension,
};
use specforge_project::{ResolvedSettings, SettingOrigin};
use specforge_registry::build_registries;

use specforge_extension_sdk::prelude::*;
use specforge_project::testing::TestProject;
use specforge_wasm::testing::InProcessRuntime;

const EXT: &str = "@pin/plain";

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.kind("gadget", |k| {
        k.keyword("gadget");
    });
    c.pass("audit", |p| {
        p.phase("check").run(|_: &PassInput| Vec::new());
    });
    c
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a project without settings reads and calls as it did before settings existed"
)]
fn a_project_without_settings_reads_and_calls_as_today() {
    let runtime = Arc::new(InProcessRuntime::new().with(extension));
    let compiled = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[EXT])
        .installing_enabled()
        .file("a.spec", "gadget a \"A\" {}\n")
        .compile_in(runtime.clone());
    for d in compiled.diagnostics() {
        assert!(
            !matches!(d.code.as_str(), "E086" | "E087" | "W166"),
            "{d:?}"
        );
    }
    assert!(compiled.environment().settings().is_empty());
    let calls = runtime.calls();
    assert!(!calls.is_empty());
    for call in calls {
        assert!(call.input.get("settings").is_none(), "{}", call.export);
    }
}

/// `settings` resolved over `declarations`, in the text of a `specforge.json` enabling both extensions.
fn resolve_text(
    declarations: Vec<specforge_protocol_types::ExtensionDeclaration>,
    text: &str,
) -> (ResolvedSettings, Vec<Diagnostic>) {
    let registry = build_registries(declarations);
    let project = Assembled::settings();
    let enabled = project.compiled().environment().enabled().to_vec();
    ResolvedSettings::resolve(Some(text), registry.settings(), &enabled)
}

fn resolve(settings: Value) -> (ResolvedSettings, Vec<Diagnostic>) {
    let text = serde_json::to_string_pretty(&settings_config(settings)).unwrap();
    resolve_text(settings_declarations(), &text)
}

fn codes_of(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(|d| d.code.as_str()).collect()
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a setting resolves explicit, else preset, else default, else unset, and records where it came from"
)]
fn settings_resolve_explicit_over_preset_over_default() {
    let of = |resolved: &ResolvedSettings, name: &str| {
        let setting = resolved.of("@t/s").unwrap().get(name).unwrap().clone();
        (setting.value, setting.origin)
    };
    let strict = SettingOrigin::Preset {
        source: "profile".into(),
        value: "strict".into(),
    };

    let (resolved, found) = resolve(json!({"@t/s": {"profile": "strict"}}));
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        of(&resolved, "profile"),
        (Some(json!("strict")), SettingOrigin::Explicit)
    );
    assert_eq!(of(&resolved, "flag"), (Some(json!(true)), strict.clone()));
    assert_eq!(
        of(&resolved, "level"),
        (Some(json!(1)), SettingOrigin::Default)
    );
    assert_eq!(of(&resolved, "mode"), (None, SettingOrigin::Unset));
    assert!(resolved.gated().is_empty());

    // Explicit beats the preset, and false gates the codes.
    let (resolved, _) = resolve(json!({"@t/s": {"profile": "strict", "flag": false}}));
    assert_eq!(
        of(&resolved, "flag"),
        (Some(json!(false)), SettingOrigin::Explicit)
    );
    assert_eq!(
        resolved
            .gated()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["t.s/W901", "t.s/W903"]
    );
    // The default (compat) presets nothing: flag is false by its default and gates.
    let (resolved, _) = resolve(json!({}));
    assert_eq!(
        of(&resolved, "flag"),
        (Some(json!(false)), SettingOrigin::Default)
    );
    assert_eq!(resolved.gated().len(), 2);

    // A value E086 refuses falls back to the default; null unsets a setting with no default.
    let (resolved, found) = resolve(json!({"@t/s": {"level": 9, "notes": null, "mode": "a"}}));
    assert_eq!(codes_of(&found), ["E086"]);
    assert_eq!(
        of(&resolved, "level"),
        (Some(json!(1)), SettingOrigin::Default)
    );
    assert_eq!(of(&resolved, "notes"), (None, SettingOrigin::Explicit));
    assert_eq!(
        of(&resolved, "mode"),
        (Some(json!("a")), SettingOrigin::Explicit)
    );

    // A family key is listed when written, and the wire carries every value.
    let (resolved, found) =
        resolve(json!({"@t/s": {"models.planning": "opus", "tiers.codex.opus": {"model": "m"}}}));
    assert!(found.is_empty(), "{found:?}");
    let values = resolved.of("@t/s").unwrap();
    assert_eq!(values.get("models.planning").unwrap().declared, "models.*");
    assert!(values.get("models.execution").is_none());
    assert_eq!(values.family("models.*").count(), 1);
    let wire = resolved.wire("@t/s");
    assert_eq!(wire["models.planning"], "opus");
    assert_eq!(wire["tiers.codex.opus"], json!({"model": "m"}));
    assert_eq!(wire["level"], 1);
    assert!(!wire.contains_key("mode"));
    assert!(resolved.wire("@t/none").is_empty());
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a preset that follows a peer extension's setting is inert while the peer is not loaded"
)]
fn a_follower_preset_reads_its_peers_setting() {
    let raised = |resolved: &ResolvedSettings| {
        resolved
            .effective(Diagnostic::untyped("t.f/W904", Severity::Warning, "x"))
            .unwrap()
            .severity
    };
    let (resolved, found) = resolve(json!({"@t/s": {"profile": "strict"}, "@t/f": {}}));
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(raised(&resolved), Severity::Error);
    assert_eq!(
        resolved
            .effective(Diagnostic::untyped("t.s/W902", Severity::Warning, "x"))
            .unwrap()
            .severity,
        Severity::Error
    );
    let (resolved, _) = resolve(json!({}));
    assert_eq!(raised(&resolved), Severity::Warning);
    // The peer is not loaded: the preset is inert.
    let text =
        serde_json::to_string(&settings_config(json!({"@t/s": {"profile": "strict"}}))).unwrap();
    let (alone, _) = resolve_text(vec![follower_extension().declaration()], &text);
    assert!(alone.raises().is_empty());
    let _ = settings_extension;
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a setting not used as written is E086 at its key with the line, the column and a suggestion"
)]
fn a_setting_not_used_as_written_is_e086_at_its_key() {
    let text = r#"{
  "extensions": ["@t/s", "@t/f"],
  "settings": {
    "@t/s": {
      "levl": 2,
      "level": "high",
      "mode": "c",
      "models.review": "opus",
      "profile": null,
      "flag": 3,
      "tiers.codex.opus": {"effort": "low"},
      "sections": [{"heading": "x"}, {}],
      "W904": true
    },
    "@t/f": []
  }
}"#;
    let (_, found) = resolve_text(settings_declarations(), text);
    let at = |d: &Diagnostic| {
        let span = d.span.as_ref().unwrap();
        (
            span.file.as_str().to_string(),
            span.start_line,
            span.start_col,
        )
    };
    let shown: Vec<(&str, (String, usize, usize))> =
        found.iter().map(|d| (d.code.as_str(), at(d))).collect();
    let file = || "specforge.json".to_string();
    assert_eq!(
        shown,
        [
            ("E086", (file(), 5, 7)),
            ("E086", (file(), 6, 7)),
            ("E086", (file(), 7, 7)),
            ("E086", (file(), 8, 7)),
            ("E086", (file(), 9, 7)),
            ("E086", (file(), 10, 7)),
            ("E086", (file(), 11, 7)),
            ("E086", (file(), 12, 7)),
            ("E086", (file(), 13, 7)),
            ("E086", (file(), 15, 5)),
        ],
        "{found:#?}"
    );
    let message = |i: usize| found[i].message.as_str();
    let suggestion = |i: usize| found[i].suggestion.clone().unwrap_or_default();
    assert_eq!(message(0), "'levl' is not a setting of '@t/s'");
    assert_eq!(suggestion(0), "did you mean 'level'?");
    assert_eq!(
        message(1),
        "'level' must be a whole number from 1 to 3, found a string"
    );
    assert_eq!(message(2), "'mode' must be one of a, b");
    assert!(suggestion(2).contains("use one of"), "{}", suggestion(2));
    assert_eq!(
        message(3),
        "'models.review' is not a key of 'models.*': use one of planning, execution"
    );
    assert!(
        message(4).starts_with("'profile' cannot be unset: it has a default"),
        "{}",
        message(4)
    );
    assert_eq!(
        message(5),
        "'flag' must be true or false, found a whole number"
    );
    assert!(
        message(6).starts_with("'tiers.codex.opus' must be an object with"),
        "{}",
        message(6)
    );
    assert_eq!(suggestion(6), "give the member 'model'");
    assert!(
        message(7).starts_with("'sections' must be a list of an object with heading*"),
        "{}",
        message(7)
    );
    assert_eq!(message(8), "'W904' is not a setting of '@t/s'");
    assert!(
        suggestion(8).contains("did you mean")
            || suggestion(8).is_empty()
            || suggestion(8).contains("move it under"),
        "{}",
        suggestion(8)
    );
    assert_eq!(message(9), "settings for '@t/f' must be an object");

    // Another extension's setting: move it.
    let text = r#"{"extensions": ["@t/s", "@t/f"], "settings": {"@t/f": {"flag": true}}}"#;
    let (_, found) = resolve_text(settings_declarations(), text);
    assert_eq!(
        found[0].suggestion.as_deref(),
        Some("'flag' is @t/s's: move it under \"@t/s\"")
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a secret written in specforge.json is E087 and no diagnostic echoes its value"
)]
fn a_secret_in_specforge_json_is_e087_and_never_echoed() {
    let (resolved, found) = resolve(json!({"@t/s": {
        "api_key": "sk-live-1234",
        "openai_token": "sk-live-5678",
        "max_prompt_tokens": 100,
        "notes": "sk-live-9999",
    }}));
    let codes: Vec<(&str, usize)> = found
        .iter()
        .map(|d| (d.code.as_str(), d.span.as_ref().unwrap().start_line))
        .collect();
    assert_eq!(found.len(), 3, "{found:#?}");
    assert_eq!(
        found.iter().filter(|d| d.code == "E087").count(),
        2,
        "{codes:?}"
    );
    assert_eq!(found.iter().filter(|d| d.code == "E086").count(), 1);
    assert!(
        found
            .iter()
            .any(|d| d.code == "E086" && d.message.contains("max_prompt_tokens"))
    );
    assert!(
        found[0].message.contains("T_API_KEY"),
        "{}",
        found[0].message
    );
    for d in &found {
        let text = format!("{d:?}");
        assert!(!text.contains("sk-live"), "{text}");
    }
    // The secret's value is ignored; the ordinary setting is kept.
    assert!(resolved.of("@t/s").unwrap().get("api_key").is_none());
    assert_eq!(resolved.wire("@t/s")["notes"], "sk-live-9999");
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings for an extension that is not enabled are W166 once and none for one that failed to load"
)]
fn settings_for_an_extension_not_enabled_are_w166() {
    let (resolved, found) = resolve(json!({"@t/other": {"x": 1, "y": 2}, "@t/s": {"flag": true}}));
    assert_eq!(codes_of(&found), ["W166"]);
    assert!(
        found[0]
            .message
            .starts_with("settings for '@t/other', which is not in \"extensions\"")
    );
    assert!(resolved.of("@t/other").is_none());
    // Enabled, but it did not load: its settings are kept unchecked, with no diagnostic of ours.
    let text = serde_json::to_string(&json!({
        "extensions": ["@t/s", "@t/f", "@t/ghost"],
        "settings": {"@t/ghost": {"anything": [1]}},
    }))
    .unwrap();
    let project = Assembled::settings().config_json(serde_json::from_str(&text).unwrap());
    let environment = project.compiled().environment();
    assert!(
        environment
            .diagnostics()
            .all(|d| d.code != "W166" && d.code != "E086"),
        "{:?}",
        environment.diagnostics().collect::<Vec<_>>()
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a secret written in specforge.json is E087 and no diagnostic echoes its value"
)]
fn a_credential_under_an_unread_extension_is_e087_and_a_misspelt_token_setting_is_e086() {
    // Not enabled: W166 for the extension, E087 for the credential (its value is never echoed).
    let (_, found) = resolve(json!({"@t/other": {"api_key": "sk-live-1", "x": 1}}));
    assert_eq!(codes_of(&found), ["W166", "E087"], "{found:#?}");
    assert!(!format!("{found:?}").contains("sk-live"));
    // Enabled (`@t/f`) but not loaded (only `@t/s` is declared): no W166, still E087.
    let text = serde_json::to_string(&settings_config(
        json!({"@t/f": {"github_token": "sk-live-2", "anything": 1}}),
    ))
    .unwrap();
    let (_, found) = resolve_text(vec![settings_extension().declaration()], &text);
    assert_eq!(codes_of(&found), ["E087"], "{found:#?}");
    // `token_budget` is no credential: a setting that is not declared, told as one.
    let (_, found) = resolve(json!({"@t/s": {"token_budget": 5, "client_secret": "x"}}));
    assert_eq!(codes_of(&found), ["E087", "E086"], "{found:#?}");
    assert!(
        found[1]
            .message
            .contains("'token_budget' is not a setting of '@t/s'"),
        "{}",
        found[1].message
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "the environment reports a setting not used as written after the registry build's diagnostics"
)]
fn the_environment_reports_settings_diagnostics_last() {
    let project = Assembled::settings().config_json(settings_config(json!({"@t/s": {"levl": 1}})));
    let environment = project.compiled().environment();
    let setup = environment.setup_diagnostics();
    let last = setup.last().unwrap();
    assert_eq!(last.part, specforge_project::SetupPart::Settings);
    assert_eq!(last.diagnostic.code, "E086");
    assert_eq!(environment.settings().wire("@t/s")["level"], 1);
}

/// `@t/s` declaring `W900` and `items` as its settings (raw JSON, so a key the wire does not know yet is written).
fn raw_settings(items: Value) -> Vec<specforge_protocol_types::ExtensionDeclaration> {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/s", "1.0.0"));
    c.code("W900", |d| {
        d.title("A gated finding")
            .explanation("A finding a setting gates.");
    });
    c.raw_category(specforge_extension_sdk::DeclaredCategory::Settings, items);
    vec![c.declaration()]
}

fn resolve_raw(items: Value, settings: Value) -> (ResolvedSettings, Vec<Diagnostic>) {
    let text = serde_json::to_string_pretty(&settings_config(settings)).unwrap();
    resolve_text(raw_settings(items), &text)
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a bool setting gates codes while it is on"
)]
fn a_bool_setting_gates_codes_while_it_is_on() {
    let items = json!([{"kind": "setting", "name": "x.enforce", "field_type": "bool", "default": false,
                        "gates_when_on": ["W900"]}]);
    let (on, found) = resolve_raw(items.clone(), json!({"@t/s": {"x.enforce": true}}));
    assert!(found.is_empty(), "{found:?}");
    assert!(
        on.gated().iter().any(|c| c.ends_with("/W900")),
        "{:?}",
        on.gated()
    );
    let (off, _) = resolve_raw(items, json!({"@t/s": {"x.enforce": false}}));
    assert!(
        !off.gated().iter().any(|c| c.ends_with("/W900")),
        "{:?}",
        off.gated()
    );
}

fn ratio_items() -> Value {
    json!([{"kind": "setting", "name": "x.ratio", "field_type": "number", "minimum": 0, "maximum": 100}])
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a number setting takes a decimal within its bounds"
)]
#[ignore = "pins K20a; T16 un-ignores"]
fn a_number_setting_takes_a_decimal() {
    let (resolved, found) = resolve_raw(ratio_items(), json!({"@t/s": {"x.ratio": 30.5}}));
    assert!(found.is_empty(), "{found:?}");
    let ratio = resolved.of("@t/s").unwrap().get("x.ratio").unwrap();
    assert_eq!(ratio.value, Some(json!(30.5)));
    let (_, found) = resolve_raw(ratio_items(), json!({"@t/s": {"x.ratio": "x"}}));
    assert_eq!(codes_of(&found), ["E086"], "{found:?}");
    assert!(
        found[0].message.contains("a number from 0 to 100"),
        "{}",
        found[0].message
    );
}

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a number setting takes a decimal within its bounds"
)]
#[ignore = "pins K20a; T16 un-ignores"]
fn a_number_setting_keeps_its_bounds() {
    let (_, found) = resolve_raw(ratio_items(), json!({"@t/s": {"x.ratio": 100.5}}));
    assert_eq!(codes_of(&found), ["E086"], "{found:?}");
    assert!(
        found[0].message.contains("a number from 0 to 100"),
        "{}",
        found[0].message
    );
}
