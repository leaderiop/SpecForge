//! What the loaded extensions declare as settings, presets and secrets, checked once (ADR 0288).
//!
//! A setting's type is a field type: [`FieldRegistryEntry`] reads the declaration with K3's rules, so a setting, a
//! field and the published schema share one type system. Every fault below is W021 (W019 for an unknown type),
//! drops the item or the attribute it names and keeps the rest.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::{Diagnostic, codes};
use specforge_diagnostics::{Codebook, Level, qualify};
use specforge_graph::FieldForm;
use specforge_protocol_types::{
    ExtensionDeclaration, Family, FieldDescriptor, FieldType, PresetDescriptor, SecretDescriptor,
    SettingDescriptor, SettingsItem, ValidationSeverity, is_segment, valid_name,
};

use super::field::{FieldProblem, FieldRegistryEntry, UnknownFieldType};

/// One checked setting: its descriptor as the registry kept it, and its type.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredSetting {
    pub descriptor: SettingDescriptor,
    pub form: FieldForm,
}

/// What one extension declared, checked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtensionSettings {
    pub settings: Vec<DeclaredSetting>,
    pub presets: Vec<PresetDescriptor>,
    pub secrets: Vec<SecretDescriptor>,
}

impl ExtensionSettings {
    /// The setting `key` is a key of: an exact name wins over a family that also matches.
    pub fn find(&self, key: &str) -> Option<&DeclaredSetting> {
        Family::pick(self.settings.iter().map(|s| &s.descriptor), key).and_then(|d| {
            self.settings
                .iter()
                .find(|s| std::ptr::eq(&s.descriptor, d))
        })
    }

    /// The secret declared as `name`.
    pub fn secret(&self, name: &str) -> Option<&SecretDescriptor> {
        self.secrets.iter().find(|s| s.name == name)
    }

    pub fn is_empty(&self) -> bool {
        self.settings.is_empty() && self.presets.is_empty() && self.secrets.is_empty()
    }
}

/// Every loaded extension's settings, in load order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettingsRegistry {
    extensions: Vec<(String, ExtensionSettings)>,
}

impl SettingsRegistry {
    /// `extension`'s settings, when it is loaded.
    pub fn of(&self, extension: &str) -> Option<&ExtensionSettings> {
        self.extensions
            .iter()
            .find(|(name, _)| name == extension)
            .map(|(_, settings)| settings)
    }

    /// Every extension and its settings, in load order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ExtensionSettings)> {
        self.extensions
            .iter()
            .map(|(name, settings)| (name.as_str(), settings))
    }

    /// Whether no loaded extension declares anything.
    pub fn is_empty(&self) -> bool {
        self.extensions.iter().all(|(_, s)| s.is_empty())
    }
}

/// The settings registry of `declarations` (load order), and its W019 and W021.
pub(crate) fn build(
    declarations: &[ExtensionDeclaration],
    book: &Codebook,
) -> (SettingsRegistry, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let extensions = declarations
        .iter()
        .map(|declaration| {
            let mut found = Found {
                extension: declaration.name(),
                book,
                diagnostics: &mut diagnostics,
            };
            (
                declaration.name().to_string(),
                declare(declaration, declarations, &mut found),
            )
        })
        .collect();
    (SettingsRegistry { extensions }, diagnostics)
}

struct Found<'a> {
    extension: &'a str,
    book: &'a Codebook,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl Found<'_> {
    fn w021(&mut self, what: String) {
        self.diagnostics.push(Diagnostic::new(
            codes::W021,
            format!("extension '{}': {what}", self.extension),
        ));
    }

    fn w019(&mut self, what: String) {
        self.diagnostics.push(Diagnostic::new(
            codes::W019,
            format!("extension '{}': {what}", self.extension),
        ));
    }
}

fn declare(
    declaration: &ExtensionDeclaration,
    all: &[ExtensionDeclaration],
    found: &mut Found,
) -> ExtensionSettings {
    let mut out = ExtensionSettings::default();
    let flags = declaration.feature_flags.iter().map(|f| f.as_setting());
    let mut named = BTreeSet::new();
    for flag in flags {
        named.insert(flag.name.clone());
        register(flag, &mut out, found);
    }
    let mut presets = Vec::new();
    let mut secrets = Vec::new();
    for item in &declaration.settings {
        match item {
            SettingsItem::Setting(setting) => {
                if named.contains(&setting.name) && !Family::is_family(&setting.name) {
                    found.w021(format!(
                        "setting '{}' is declared as a feature flag and as a setting (the flag wins)",
                        setting.name
                    ));
                } else {
                    register(setting.clone(), &mut out, found);
                }
            }
            SettingsItem::Preset(preset) => presets.push(preset),
            SettingsItem::Secret(secret) => secrets.push(secret),
            SettingsItem::Unreadable(item) => {
                found.w021(format!("a settings item is dropped: {}", item.reason));
            }
        }
    }
    out.presets = check_presets(declaration, all, &presets, &out.settings, found);
    out.secrets = check_secrets(&secrets, &out.settings, found);
    out
}

/// Check `setting` and keep it in `out` when it can be used.
fn register(mut setting: SettingDescriptor, out: &mut ExtensionSettings, found: &mut Found) {
    let name = setting.name.clone();
    if let Err(why) = valid_name(&name) {
        found.w021(format!("setting '{name}' is dropped: {why}"));
        return;
    }
    for kept in &out.settings {
        let other = &kept.descriptor.name;
        if *other == name {
            found.w021(format!(
                "setting '{name}' is declared twice (the first wins)"
            ));
            return;
        }
        if Family::is_family(&name) && Family::is_family(other) && Family::overlap(&name, other) {
            found.w021(format!(
                "setting family '{name}' overlaps '{other}': one key would match both (the first wins)"
            ));
            return;
        }
    }
    let stars = Family::stars(&name);
    if !setting.keys.is_empty() && stars == 0 {
        found.w021(format!(
            "setting '{name}' declares keys, but its name is not a family: keys are dropped"
        ));
        setting.keys.clear();
    } else if !setting.keys.is_empty() && setting.keys.len() != stars {
        found.w021(format!(
            "setting '{name}' declares {} key lists for {stars} '*' segments: keys are dropped",
            setting.keys.len()
        ));
        setting.keys.clear();
    }
    if let Some(bad) = setting.keys.iter().flatten().find(|k| !is_segment(k)) {
        found.w021(format!(
            "setting '{name}' lists the key '{bad}', which is not a segment ([A-Za-z0-9_-]+): the setting is dropped"
        ));
        return;
    }
    if Family::is_family(&name) && setting.default.take().is_some() {
        found.w021(format!(
            "setting family '{name}' declares a default: a default applies to a setting named whole, so it is dropped"
        ));
    }
    let field = FieldDescriptor {
        name: name.clone(),
        field_type: setting.field_type.clone(),
        enum_values: setting.enum_values.clone(),
        minimum: setting.minimum,
        maximum: setting.maximum,
        members: setting.members.clone(),
        ..Default::default()
    };
    let entry = match FieldRegistryEntry::new("(settings)", found.extension, field) {
        Ok(entry) => entry,
        Err(UnknownFieldType(unknown)) => {
            found.w019(format!(
                "unknown field type '{unknown}' for setting '{name}': the setting is dropped"
            ));
            return;
        }
    };
    if matches!(
        entry.field_type(),
        FieldType::Reference | FieldType::ReferenceList | FieldType::Identifier
    ) {
        found.w021(format!(
            "setting '{name}' has type '{}': settings are read before any graph exists, so the setting is dropped",
            entry.field_type().as_str()
        ));
        return;
    }
    for problem in entry.problems() {
        match problem {
            FieldProblem::UnknownMemberType { member, field_type } => found.w019(format!(
                "unknown field type '{field_type}' for member '{member}' of setting '{name}'"
            )),
            FieldProblem::Refused { field, what } => {
                found.w021(format!("setting '{field}' {what}"));
            }
        }
    }
    let kept = entry.declared();
    setting.field_type = kept.field_type.clone();
    setting.enum_values = kept.enum_values.clone();
    setting.minimum = kept.minimum;
    setting.maximum = kept.maximum;
    setting.members = kept.members.clone();
    let form = entry.form().clone();

    match &setting.default {
        Some(serde_json::Value::Null) => {
            found.w021(format!(
                "setting '{name}' declares null as its default: leave the default out for an unset setting"
            ));
            setting.default = None;
        }
        Some(value) => {
            if let Err(refusal) = form.admit_json(value) {
                found.w021(format!(
                    "setting '{name}' declares a default its type does not admit{}",
                    refusal
                        .suggestion
                        .map(|s| format!(" ({s})"))
                        .unwrap_or_default()
                ));
                setting.default = None;
            }
        }
        None => {}
    }
    for (key, codes) in [
        ("gates", &mut setting.gates),
        ("gates_when_on", &mut setting.gates_when_on),
    ] {
        if !codes.is_empty() && form.field_type() != FieldType::Bool {
            found.w021(format!(
                "setting '{name}' declares {key}, but its type is '{}': only a bool setting gates codes",
                form.field_type().as_str()
            ));
            codes.clear();
        }
        if !codes.is_empty() && Family::is_family(&name) {
            found.w021(format!(
                "setting '{name}' declares {key}, but it is a family: only a setting named whole gates codes"
            ));
            codes.clear();
        }
        let (extension, book) = (found.extension, found.book);
        let (catalogued, missing): (Vec<String>, Vec<String>) = codes
            .drain(..)
            .partition(|code| book.catalogued(extension, code));
        for code in missing {
            found.w021(format!(
                "setting '{name}' {key} '{code}', which the extension's codes do not catalog"
            ));
        }
        *codes = catalogued;
    }
    // A code gated both ways would never be reported: each is W021 and goes from `gates_when_on` (ADR 0305 D1).
    let both: Vec<String> = setting
        .gates
        .iter()
        .filter(|code| setting.gates_when_on.contains(code))
        .cloned()
        .collect();
    for code in both {
        found.w021(format!(
            "setting '{name}' gates '{code}' both while it is on and while it is off: it would never be reported"
        ));
        setting.gates_when_on.retain(|c| *c != code);
    }
    out.settings.push(DeclaredSetting {
        descriptor: setting,
        form,
    });
}

/// The values a preset source selects by, when they are known: an enum's, `true` and `false` for a bool.
enum Vocabulary {
    Known(Vec<String>),
    /// The source is a peer's that is not loaded: nothing to check against, and the preset is inert.
    Unloaded,
}

fn vocabulary_of(setting: &SettingDescriptor) -> Option<Vec<String>> {
    match FieldType::parse(&setting.field_type)? {
        FieldType::Bool => Some(vec!["true".to_string(), "false".to_string()]),
        FieldType::Enum => Some(setting.enum_values.clone()),
        _ => None,
    }
}

/// The source of a preset: `profile` (the declaring extension's) or `@gsd/core:profile` (a peer's).
fn split_source(source: &str) -> (Option<&str>, &str) {
    match source.rsplit_once(':') {
        Some((extension, name)) => (Some(extension), name),
        None => (None, source),
    }
}

fn check_presets(
    declaration: &ExtensionDeclaration,
    all: &[ExtensionDeclaration],
    presets: &[&PresetDescriptor],
    settings: &[DeclaredSetting],
    found: &mut Found,
) -> Vec<PresetDescriptor> {
    let own_sources: BTreeSet<&str> = presets
        .iter()
        .filter_map(|p| match split_source(&p.source) {
            (None, name) => Some(name),
            _ => None,
        })
        .collect();
    let mut preset_by: BTreeMap<String, usize> = BTreeMap::new();
    let mut kept = Vec::new();
    for (index, preset) in presets.iter().enumerate() {
        let source = preset.source.as_str();
        let vocabulary = match split_source(source) {
            (None, name) => {
                let Some(setting) = settings.iter().find(|s| s.descriptor.name == name) else {
                    found.w021(format!(
                        "preset source '{source}' is not a setting the extension declares: the preset is dropped"
                    ));
                    continue;
                };
                match vocabulary_of(&setting.descriptor) {
                    Some(values) => Vocabulary::Known(values),
                    None => {
                        found.w021(format!(
                            "preset source '{source}' has type '{}': only an enum or bool setting selects a preset, so the preset is dropped",
                            setting.descriptor.field_type
                        ));
                        continue;
                    }
                }
            }
            (Some(peer), name) => {
                if !declaration.peers().iter().any(|p| p.name == peer) {
                    found.w021(format!(
                        "preset source '{source}' names '{peer}', which is not a declared peer: the preset is dropped"
                    ));
                    continue;
                }
                match all.iter().find(|d| d.name() == peer) {
                    None => Vocabulary::Unloaded,
                    Some(declared) => {
                        match peer_setting(declared, name).and_then(|s| vocabulary_of(&s)) {
                            Some(values) => Vocabulary::Known(values),
                            None => {
                                found.w021(format!(
                                "preset source '{source}' is not an enum or bool setting of '{peer}': the preset is dropped"
                            ));
                                continue;
                            }
                        }
                    }
                }
            }
        };
        let mut cleaned = PresetDescriptor {
            source: preset.source.clone(),
            values: BTreeMap::new(),
        };
        for (value, entry) in &preset.values {
            if let Vocabulary::Known(values) = &vocabulary
                && !values.contains(value)
            {
                found.w021(format!(
                    "preset '{source}' has an entry for '{value}', which is not one of {}: the entry is dropped",
                    values.join(", ")
                ));
                continue;
            }
            let mut entry = entry.clone();
            entry.settings.retain(|key, v| {
                let Some(setting) = Family::pick(settings.iter().map(|s| &s.descriptor), key)
                    .and_then(|d| settings.iter().find(|s| std::ptr::eq(&s.descriptor, d)))
                else {
                    found.w021(format!(
                        "preset '{source}' presets '{key}', which the extension does not declare"
                    ));
                    return false;
                };
                if own_sources.contains(key.as_str()) {
                    found.w021(format!(
                        "preset '{source}' presets '{key}', a preset source: a preset is one level"
                    ));
                    return false;
                }
                if let Err(refusal) = setting.form.admit_json(v) {
                    found.w021(format!(
                        "preset '{source}' presets '{key}' with a value its type does not admit{}",
                        refusal
                            .suggestion
                            .map(|s| format!(" ({s})"))
                            .unwrap_or_default()
                    ));
                    return false;
                }
                if preset_by.get(key).is_some_and(|owner| *owner != index) {
                    found.w021(format!(
                        "two presets preset '{key}': the later one's value is dropped"
                    ));
                    return false;
                }
                preset_by.insert(key.clone(), index);
                true
            });
            let extension = found.extension;
            let book = found.book;
            entry.severities.retain(|code, severity| {
                if !book.catalogued(extension, code) {
                    found.w021(format!(
                        "preset '{source}' raises '{code}', which the extension's codes do not catalog"
                    ));
                    return false;
                }
                let letter = book.letter(&qualify(extension, code));
                if rank(severity) <= letter.map_or(0, level_rank) {
                    found.w021(format!(
                        "preset '{source}' raises '{code}' to {} though its level is not lower: a preset raises, never lowers",
                        severity_name(severity)
                    ));
                    return false;
                }
                true
            });
            cleaned.values.insert(value.clone(), entry);
        }
        kept.push(cleaned);
    }
    kept
}

/// The setting `name` of `declaration`, a flag counted.
fn peer_setting(declaration: &ExtensionDeclaration, name: &str) -> Option<SettingDescriptor> {
    declaration
        .settings
        .iter()
        .filter_map(|item| match item {
            SettingsItem::Setting(s) => Some(s.clone()),
            _ => None,
        })
        .chain(declaration.feature_flags.iter().map(|f| f.as_setting()))
        .find(|s| s.name == name)
}

fn rank(severity: &ValidationSeverity) -> u8 {
    match severity {
        ValidationSeverity::Info => 1,
        ValidationSeverity::Warning => 2,
        ValidationSeverity::Error => 3,
    }
}

fn level_rank(level: Level) -> u8 {
    match level {
        Level::Info => 1,
        Level::Warning => 2,
        Level::Error => 3,
        Level::SetByPass => 0,
    }
}

fn severity_name(severity: &ValidationSeverity) -> &'static str {
    match severity {
        ValidationSeverity::Info => "info",
        ValidationSeverity::Warning => "warning",
        ValidationSeverity::Error => "error",
    }
}

fn check_secrets(
    secrets: &[&SecretDescriptor],
    settings: &[DeclaredSetting],
    found: &mut Found,
) -> Vec<SecretDescriptor> {
    let mut kept: Vec<SecretDescriptor> = Vec::new();
    for secret in secrets {
        let name = &secret.name;
        if valid_name(name).is_err() || Family::is_family(name) || name.contains('.') {
            found.w021(format!(
                "secret '{name}' is dropped: a secret's name is one segment of [A-Za-z0-9_-]+"
            ));
        } else if !is_env_name(&secret.env) {
            found.w021(format!(
                "secret '{name}' is read from '{}', which is no environment variable name ([A-Z_][A-Z0-9_]*): the secret is dropped",
                secret.env
            ));
        } else if settings.iter().any(|s| s.descriptor.name == *name) {
            found.w021(format!(
                "secret '{name}' is also a setting's name: the secret is dropped"
            ));
        } else if kept.iter().any(|k| k.name == *name) {
            found.w021(format!(
                "secret '{name}' is declared twice (the first wins)"
            ));
        } else {
            kept.push((*secret).clone());
        }
    }
    kept
}

fn is_env_name(env: &str) -> bool {
    let mut chars = env.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}
