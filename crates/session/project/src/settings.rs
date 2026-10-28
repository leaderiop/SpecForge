//! The settings `specforge.json` sets, resolved against what the loaded extensions declare (ADR 0288).
//!
//! [`ResolvedSettings::resolve`] is pure: the config text, the declarations' settings registry and the enabled
//! extensions in, each extension's effective settings (a value and where it came from), the codes its bool settings
//! gate, the severities its presets raise, and the E086, E087 and W166 found, in file order, out. The language
//! server runs it on an unsaved buffer; the environment runs it once per load.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use specforge_common::json_span;
use specforge_common::{Diagnostic, Severity, codes, find_close_match};
use specforge_diagnostics::qualify;
use specforge_graph::FieldForm;
use specforge_installed::EnabledExtension;
use specforge_protocol_types::{Family, FieldType, ValidationSeverity};
use specforge_registry::{DeclaredSetting, ExtensionSettings as Declared, SettingsRegistry};

use crate::Raises;

/// Where a setting's effective value came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingOrigin {
    /// Written in `specforge.json`.
    Explicit,
    /// Preset by the value `value` of the setting `source` (`profile`, or `@gsd/core:profile`).
    Preset { source: String, value: String },
    /// The declared default.
    Default,
    /// Nothing sets it and it has no default (or `specforge.json` writes `null`).
    Unset,
}

/// One setting of one extension, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    /// The effective value; `None`: unset.
    pub value: Option<Value>,
    pub origin: SettingOrigin,
    /// The declared name it is a key of: the setting's own, or its family (`models.*`).
    pub declared: String,
}

/// One extension's resolved settings: every setting it declares by an exact name, and each key of a family that
/// is written or preset.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtensionValues {
    values: BTreeMap<String, Setting>,
}

impl ExtensionValues {
    /// The setting `name` (a key, for a family's).
    pub fn get(&self, name: &str) -> Option<&Setting> {
        self.values.get(name)
    }

    /// Every setting, by key.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Setting)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// The keys of `family` (`models.*`) that are written or preset. An exact setting inside the family's
    /// pattern (`models.default`) is its own, not a key of it.
    pub fn family<'a>(&'a self, family: &'a str) -> impl Iterator<Item = (&'a str, &'a Setting)> {
        self.iter()
            .filter(move |(_, setting)| Family::is_family(family) && setting.declared == family)
    }
}

/// Every loaded extension's effective settings, the codes they gate and the severities they raise.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedSettings {
    by_extension: BTreeMap<String, ExtensionValues>,
    /// The codes, as they print, of every bool setting that is false.
    gated: BTreeSet<String>,
    raises: Raises,
}

impl ResolvedSettings {
    /// `extension`'s effective settings, when it is loaded.
    pub fn of(&self, extension: &str) -> Option<&ExtensionValues> {
        self.by_extension.get(extension)
    }

    /// The codes (as they print) reported by no source while their setting is false.
    pub fn gated(&self) -> &BTreeSet<String> {
        &self.gated
    }

    /// The severities the selected presets raise.
    pub fn raises(&self) -> &Raises {
        &self.raises
    }

    /// Nothing is set, gated or raised.
    pub fn is_empty(&self) -> bool {
        self.gated.is_empty()
            && self.raises.is_empty()
            && self
                .by_extension
                .values()
                .all(|v| v.values.values().all(|s| s.origin == SettingOrigin::Unset))
    }

    /// What a guest call carries for `extension`: every setting with a value, by name (a family's keys as written
    /// or preset).
    pub fn wire(&self, extension: &str) -> Map<String, Value> {
        self.of(extension)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, s)| Some((key.to_string(), s.value.clone()?)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// What settings make of one finding: `None` while its code is gated off, else its severity raised (upward
    /// only). The one place both are applied; a surface's strict comes after.
    pub fn effective(&self, diagnostic: Diagnostic) -> Option<Diagnostic> {
        if self.gated.contains(&diagnostic.code) {
            return None;
        }
        Some(self.raises.apply(diagnostic))
    }

    /// Every value of `config_text`'s `settings`, checked against `registry` (the loaded declarations), for the
    /// extensions `enabled` names: the effective settings, and the E086, E087 and W166 found, in file order.
    pub fn resolve(
        config_text: Option<&str>,
        registry: &SettingsRegistry,
        enabled: &[EnabledExtension],
    ) -> (Self, Vec<Diagnostic>) {
        let mut found = Vec::new();
        let mut explicit: BTreeMap<String, BTreeMap<String, Option<Value>>> = BTreeMap::new();
        if let Some(text) = config_text {
            read(text, registry, enabled, &mut explicit, &mut found);
        }
        (preset(registry, &explicit), found)
    }
}

/// The settings object of `text` and each extension's part of it checked, in file order.
fn read(
    text: &str,
    registry: &SettingsRegistry,
    enabled: &[EnabledExtension],
    explicit: &mut BTreeMap<String, BTreeMap<String, Option<Value>>>,
    found: &mut Vec<Diagnostic>,
) {
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let Some(Value::Object(settings)) = root.get("settings") else {
        return;
    };
    let index = json_span::index(text);
    let at =
        |range: &std::ops::Range<usize>| json_span::span(text, "specforge.json", range.clone());
    for member in index.children(&["settings"]) {
        let (Some(extension), Some(key)) = (member.path.last(), &member.key) else {
            continue;
        };
        let place = at(key);
        // An extension that is not read has no declared secrets to name, but a credential written under it is
        // still one in the file.
        let credentials = |found: &mut Vec<Diagnostic>| {
            for entry in index.children(&["settings", extension]) {
                let (Some(name), Some(range)) = (entry.path.last(), &entry.key) else {
                    continue;
                };
                if looks_like_a_credential(name) {
                    found.push(credential(extension, name).with_span(at(range)));
                }
            }
        };
        if !enabled.iter().any(|e| e.name == *extension) {
            found.push(
                Diagnostic::new(
                    codes::W166,
                    format!(
                        "settings for '{extension}', which is not in \"extensions\", are not read"
                    ),
                )
                .with_span(place)
                .with_suggestion(format!(
                    "add \"{extension}\" to \"extensions\" (`{}`), or remove its settings",
                    specforge_common::remedy::command("specforge add {}", &[extension]),
                )),
            );
            credentials(found);
            continue;
        }
        let Some(declared) = registry.of(extension) else {
            // Enabled, but it did not load: its own diagnostic says why, and its settings are kept unchecked.
            credentials(found);
            continue;
        };
        let Some(Value::Object(values)) = settings.get(extension) else {
            found.push(
                Diagnostic::new(
                    codes::E086,
                    format!("settings for '{extension}' must be an object"),
                )
                .with_span(place)
                .with_suggestion(
                    "write the extension's settings as { \"name\": value }".to_string(),
                ),
            );
            continue;
        };
        let mine = explicit.entry(extension.clone()).or_default();
        for entry in index.children(&["settings", extension]) {
            let (Some(name), Some(range)) = (entry.path.last(), &entry.key) else {
                continue;
            };
            let value = &values[name];
            let place = at(range);
            if let Some(diagnostic) = secret(extension, name, declared) {
                found.push(diagnostic.with_span(place));
            } else {
                match check(extension, name, value, declared, registry) {
                    Ok(admitted) => {
                        mine.insert(name.clone(), admitted);
                    }
                    Err(diagnostic) => found.push(diagnostic.with_span(place)),
                }
            }
        }
    }
}

/// What `specforge settings set` and `unset` admit before any write (ADR 0296), by the load's own rules: E087 when
/// `name` is a declared secret or looks like a credential (never written), then E086 when no setting of `extension`
/// takes it or `value` is not the form's. `Some(value)` sets, `None` unsets; the answer is the value to write, `None`
/// for an unset or a written `null`. An extension that declares no settings is E086 (a caller asks it is enabled first).
///
/// An unset only removes, so it is admitted whenever `name` is a declared setting or is `written` in the file (a stale
/// or misspelt key, or a secret that was committed, both reported by the load): it is the way out the load's
/// suggestion asks for. Only a name that is neither is refused.
pub fn admit(
    extension: &str,
    name: &str,
    value: Option<&Value>,
    written: bool,
    registry: &SettingsRegistry,
) -> Result<Option<Value>, Diagnostic> {
    let Some(declared) = registry.of(extension) else {
        return Err(Diagnostic::new(
            codes::E086,
            format!("'{extension}' declares no settings"),
        ));
    };
    match value {
        None if written || declared.find(name).is_some() => Ok(None),
        None => Err(secret(extension, name, declared)
            .unwrap_or_else(|| unknown(extension, name, declared, registry))),
        Some(value) => match secret(extension, name, declared) {
            Some(diagnostic) => Err(diagnostic),
            None => check(extension, name, value, declared, registry),
        },
    }
}

/// Whether `name` is a declared secret of `extension` or looks like a credential: what a set refuses (E087), and
/// whose written value a result never repeats.
pub fn is_secret(extension: &str, name: &str, registry: &SettingsRegistry) -> bool {
    registry
        .of(extension)
        .is_some_and(|declared| secret(extension, name, declared).is_some())
}

/// E087 when `key` names a declared secret or looks like a credential and is no declared setting.
fn secret(extension: &str, key: &str, declared: &Declared) -> Option<Diagnostic> {
    if let Some(secret) = declared.secret(key) {
        return Some(
            Diagnostic::new(
                codes::E087,
                format!(
                    "'{key}' is a secret: SpecForge reads it from {}, never from specforge.json; the value is ignored",
                    secret.env
                ),
            )
            .with_suggestion(format!(
                "remove it from specforge.json, set {} in the environment, and rotate the secret if it was committed",
                secret.env
            )),
        );
    }
    if declared.find(key).is_none() && looks_like_a_credential(key) {
        return Some(credential(extension, key));
    }
    None
}

/// E087 for `key`, which looks like a credential and is no declared secret or setting of `extension`.
fn credential(extension: &str, key: &str) -> Diagnostic {
    Diagnostic::new(
        codes::E087,
        format!(
            "'{key}' looks like a secret: SpecForge never reads a credential from specforge.json; the value is ignored"
        ),
    )
    .with_suggestion(format!(
        "remove it from specforge.json, and rotate the secret if it was committed (extension '{extension}' declares secrets with `secret`)"
    ))
}

/// Whether the last segment of `key` ends in a word that names a credential (`api_key`, `github_token`, `secret`
/// …), words bounded by `_` and `-`: `max_prompt_tokens` is not one, and neither is `token_budget`, which is a
/// misspelt setting and is told so (E086).
fn looks_like_a_credential(key: &str) -> bool {
    let last = key.rsplit('.').next().unwrap_or(key).to_lowercase();
    let words: Vec<&str> = last.split(['_', '-']).filter(|w| !w.is_empty()).collect();
    let single = [
        "apikey",
        "token",
        "secret",
        "password",
        "passwd",
        "credential",
        "credentials",
    ];
    words.last().is_some_and(|w| single.contains(w))
        || words.ends_with(&["api", "key"])
        || words.ends_with(&["private", "key"])
        || words.ends_with(&["secret", "key"])
        || words.ends_with(&["access", "key"])
}

/// The explicit value of the setting `name` (`None`: written `null`, unset), or the E086 about it.
fn check(
    extension: &str,
    name: &str,
    value: &Value,
    declared: &Declared,
    registry: &SettingsRegistry,
) -> Result<Option<Value>, Diagnostic> {
    let Some(setting) = declared.find(name) else {
        return Err(unknown(extension, name, declared, registry));
    };
    if value.is_null() {
        return match &setting.descriptor.default {
            Some(default) => Err(Diagnostic::new(
                codes::E086,
                format!("'{name}' cannot be unset: it has a default ({default})"),
            )
            .with_suggestion(format!("remove '{name}' to use the default"))),
            None => Ok(None),
        };
    }
    match setting.form.admit_json(value) {
        Ok(_) => Ok(Some(value.clone())),
        Err(refusal) => {
            let expected = expected(&setting.form);
            let message = if kind_fits(&setting.form, value) {
                format!("'{name}' must be {expected}")
            } else {
                format!("'{name}' must be {expected}, found {}", found_kind(value))
            };
            let diagnostic = Diagnostic::new(codes::E086, message);
            Err(match refusal.suggestion {
                Some(suggestion) => diagnostic.with_suggestion(suggestion),
                None => diagnostic,
            })
        }
    }
}

/// E086 for a key no setting of `extension` takes: another extension's, a family's key outside its list, or a typo.
fn unknown(
    extension: &str,
    name: &str,
    declared: &Declared,
    registry: &SettingsRegistry,
) -> Diagnostic {
    if let Some(family) = declared
        .settings
        .iter()
        .find(|s| Family::matches(&s.descriptor.name, name))
    {
        let keys: Vec<&str> = family
            .descriptor
            .keys
            .iter()
            .flatten()
            .map(String::as_str)
            .collect();
        return Diagnostic::new(
            codes::E086,
            format!(
                "'{name}' is not a key of '{}': use one of {}",
                family.descriptor.name,
                keys.join(", ")
            ),
        );
    }
    let diagnostic = Diagnostic::new(
        codes::E086,
        format!("'{name}' is not a setting of '{extension}'"),
    );
    if let Some((other, _)) = registry
        .iter()
        .find(|(other, d)| *other != extension && d.find(name).is_some())
    {
        return diagnostic
            .with_suggestion(format!("'{name}' is {other}'s: move it under \"{other}\""));
    }
    let names = declared
        .settings
        .iter()
        .filter(|s| !Family::is_family(&s.descriptor.name))
        .map(|s| s.descriptor.name.as_str());
    match find_close_match(name, names) {
        Some(close) => diagnostic.with_suggestion(format!("did you mean '{close}'?")),
        None => diagnostic,
    }
}

/// What a form takes, for a message: `a whole number from 1 to 3`.
fn expected(form: &FieldForm) -> String {
    let (minimum, maximum) = form.bounds();
    match form.field_type() {
        FieldType::Bool => "true or false".to_string(),
        FieldType::Integer => match (minimum, maximum) {
            (Some(lo), Some(hi)) => format!("a whole number from {lo} to {hi}"),
            (Some(lo), None) => format!("a whole number of at least {lo}"),
            (None, Some(hi)) => format!("a whole number of at most {hi}"),
            (None, None) => "a whole number".to_string(),
        },
        FieldType::Enum if !form.enum_values().is_empty() => {
            format!("one of {}", form.enum_values().join(", "))
        }
        FieldType::EnumList if !form.enum_values().is_empty() => {
            format!("a list of {}", form.enum_values().join(", "))
        }
        FieldType::StringList | FieldType::EnumList => "a list of strings".to_string(),
        FieldType::Block | FieldType::BlockList => {
            let members = form
                .members()
                .map(|m| {
                    m.iter()
                        .map(|(name, member)| {
                            if member.required() {
                                format!("{name}*")
                            } else {
                                name.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let object = if members.is_empty() {
                "an object".to_string()
            } else {
                format!("an object with {members} (* is required)")
            };
            if form.field_type() == FieldType::BlockList {
                format!("a list of {object}s")
            } else {
                object
            }
        }
        FieldType::Date => "a day, YYYY-MM-DD".to_string(),
        _ => "a string".to_string(),
    }
}

fn found_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "a whole number",
        Value::Number(_) => "a decimal number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

/// Whether `value` is of the JSON kind `form` reads, so the fault is in its content, not its kind.
fn kind_fits(form: &FieldForm, value: &Value) -> bool {
    match form.field_type() {
        FieldType::Bool => value.is_boolean(),
        FieldType::Integer => value.is_i64() || value.is_u64(),
        FieldType::StringList | FieldType::EnumList | FieldType::BlockList => value.is_array(),
        FieldType::Block => value.is_object(),
        _ => value.is_string(),
    }
}

/// The text a preset's `values` are keyed by: `true`, `false`, or the word.
fn selector(value: &Value) -> Option<String> {
    match value {
        Value::Bool(b) => Some(b.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// The effective settings: explicit, else preset, else default, else unset; the gated codes and the raises.
fn preset(
    registry: &SettingsRegistry,
    explicit: &BTreeMap<String, BTreeMap<String, Option<Value>>>,
) -> ResolvedSettings {
    // What a preset reads of its source: explicit, else default (a preset never sets a source).
    let source_value = |extension: &str, name: &str| -> Option<String> {
        let written = explicit.get(extension).and_then(|e| e.get(name));
        match written {
            Some(value) => value.as_ref().and_then(selector),
            None => registry
                .of(extension)?
                .find(name)?
                .descriptor
                .default
                .as_ref()
                .and_then(selector),
        }
    };
    let mut resolved = ResolvedSettings::default();
    for (extension, declared) in registry.iter() {
        let mut preset_values: BTreeMap<String, (Value, String, String)> = BTreeMap::new();
        for preset in &declared.presets {
            let (peer, name) = match preset.source.rsplit_once(':') {
                Some((peer, name)) => (peer, name),
                None => (extension, preset.source.as_str()),
            };
            if registry.of(peer).is_none() {
                continue;
            }
            let Some(chosen) = source_value(peer, name) else {
                continue;
            };
            let Some(entry) = preset.values.get(&chosen) else {
                continue;
            };
            for (key, value) in &entry.settings {
                preset_values.insert(
                    key.clone(),
                    (value.clone(), preset.source.clone(), chosen.clone()),
                );
            }
            for (code, severity) in &entry.severities {
                resolved.raises.raise(
                    qualify(extension, code),
                    match severity {
                        ValidationSeverity::Error => Severity::Error,
                        ValidationSeverity::Warning => Severity::Warning,
                        ValidationSeverity::Info => Severity::Info,
                    },
                );
            }
        }
        let written = explicit.get(extension);
        let mut values = BTreeMap::new();
        let mut keys: BTreeSet<&str> = declared
            .settings
            .iter()
            .filter(|s| !Family::is_family(&s.descriptor.name))
            .map(|s| s.descriptor.name.as_str())
            .collect();
        keys.extend(
            written
                .into_iter()
                .flat_map(|w| w.keys().map(String::as_str)),
        );
        keys.extend(preset_values.keys().map(String::as_str));
        for key in keys {
            let Some(DeclaredSetting { descriptor, .. }) = declared.find(key) else {
                continue;
            };
            let setting = match (written.and_then(|w| w.get(key)), preset_values.get(key)) {
                (Some(value), _) => Setting {
                    value: value.clone(),
                    origin: SettingOrigin::Explicit,
                    declared: descriptor.name.clone(),
                },
                (None, Some((value, source, chosen))) => Setting {
                    value: Some(value.clone()),
                    origin: SettingOrigin::Preset {
                        source: source.clone(),
                        value: chosen.clone(),
                    },
                    declared: descriptor.name.clone(),
                },
                (None, None) => match &descriptor.default {
                    Some(default) => Setting {
                        value: Some(default.clone()),
                        origin: SettingOrigin::Default,
                        declared: descriptor.name.clone(),
                    },
                    None => Setting {
                        value: None,
                        origin: SettingOrigin::Unset,
                        declared: descriptor.name.clone(),
                    },
                },
            };
            if setting.value == Some(Value::Bool(false)) {
                resolved
                    .gated
                    .extend(descriptor.gates.iter().map(|code| qualify(extension, code)));
            }
            values.insert(key.to_string(), setting);
        }
        resolved
            .by_extension
            .insert(extension.to_string(), ExtensionValues { values });
    }
    resolved
}
