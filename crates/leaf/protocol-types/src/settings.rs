//! The `settings` describe category (protocol 1.8.0, ADR 0288): the typed settings an extension declares, the
//! presets a setting's value selects, and the secrets it reads from the environment.
//!
//! A setting's type is a field type (ADR 0285): the host builds its `FieldForm` from the descriptor, so a
//! setting, a field and the shipped schema share one type system.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{FeatureFlagDescriptor, FieldDescriptor, ValidationSeverity};

/// One item of the `settings` describe category, tagged by `kind` on the wire. An item of an unknown kind, or one
/// that does not decode, is [`SettingsItem::Unreadable`]: it costs that item (W021), not the category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsItem {
    Setting(SettingDescriptor),
    Preset(PresetDescriptor),
    Secret(SecretDescriptor),
    Unreadable(UnreadableItem),
}

/// A `settings` item the protocol could not read, kept as it came so the declaration serializes unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableItem {
    pub raw: Value,
    pub reason: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Written<'a> {
    Setting(&'a SettingDescriptor),
    Preset(&'a PresetDescriptor),
    Secret(&'a SecretDescriptor),
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Read {
    Setting(SettingDescriptor),
    Preset(PresetDescriptor),
    Secret(SecretDescriptor),
}

impl Serialize for SettingsItem {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SettingsItem::Setting(s) => Written::Setting(s).serialize(serializer),
            SettingsItem::Preset(p) => Written::Preset(p).serialize(serializer),
            SettingsItem::Secret(s) => Written::Secret(s).serialize(serializer),
            SettingsItem::Unreadable(u) => u.raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for SettingsItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Value::deserialize(deserializer)?;
        let reason = match raw.get("kind").and_then(Value::as_str) {
            None => "the item has no kind".to_string(),
            Some(kind @ ("setting" | "preset" | "secret")) => {
                match serde_json::from_value::<Read>(raw.clone()) {
                    Ok(Read::Setting(s)) => return Ok(SettingsItem::Setting(s)),
                    Ok(Read::Preset(p)) => return Ok(SettingsItem::Preset(p)),
                    Ok(Read::Secret(s)) => return Ok(SettingsItem::Secret(s)),
                    Err(e) => format!("the {kind} does not decode: {e}"),
                }
            }
            Some(kind) => format!("'{kind}' is not a settings item kind"),
        };
        Ok(SettingsItem::Unreadable(UnreadableItem { raw, reason }))
    }
}

/// One setting: a dotted name, or a family whose `*` segments each stand for one key.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettingDescriptor {
    /// A dotted name (`workflow.tdd_mode`) or a family (`models.*`, `model_profile_overrides.*.*`). A segment is
    /// `[A-Za-z0-9_-]+`; `*` stands for one whole segment ([`valid_name`]).
    pub name: String,
    /// For a family, the keys each `*` may take, in order; an empty list takes any segment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<Vec<String>>,
    /// A field type name: `string`, `integer`, `bool`, `enum`, `string_list`, `enum_list`, `block`, `block_list`.
    /// Kept a string so an unknown type costs this item a diagnostic, not the category.
    pub field_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    /// A block's members, each a field of the block (ADR 0285).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<FieldDescriptor>,
    /// The value when nothing sets it; absent: unset. A family has none (W021): no key would receive it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Local codes reported by no source while this bool setting is false.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<String>,
    /// Local codes reported by no source while this bool setting is true: `gates` the other way round (protocol
    /// 1.12.0, ADR 0305).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates_when_on: Vec<String>,
}

/// What a setting's values select: for each value of an enum or bool setting, the settings it presets and the
/// severities it raises.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresetDescriptor {
    /// The enum or bool setting whose value selects an entry: one of the extension's own (`profile`) or a peer's
    /// (`@gsd/core:profile`).
    pub source: String,
    /// Per value of `source` (`true` and `false` for a bool).
    #[serde(default)]
    pub values: BTreeMap<String, PresetEntry>,
}

/// What one value of a preset's source presets: the declaring extension's settings, and its codes' severities
/// (raised, never lowered).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresetEntry {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub settings: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub severities: BTreeMap<String, ValidationSeverity>,
}

/// A secret: a name and the environment variable it is read from. The host never reads its value into a payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretDescriptor {
    pub name: String,
    pub env: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl FeatureFlagDescriptor {
    /// The flag as the bool setting it is read as (ADR 0288).
    pub fn as_setting(&self) -> SettingDescriptor {
        SettingDescriptor {
            name: self.name.clone(),
            field_type: "bool".to_string(),
            default: Some(Value::Bool(self.default_enabled)),
            description: self.description.clone(),
            ..Default::default()
        }
    }
}

/// Whether `name` is a setting name: dot-separated segments of `[A-Za-z0-9_-]+`, or `*` alone. The error says what
/// is wrong.
pub fn valid_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("the name is empty".to_string());
    }
    for segment in name.split('.') {
        if segment != "*" && !is_segment(segment) {
            return Err(format!(
                "'{segment}' is not a segment: use [A-Za-z0-9_-]+, or '*' alone for a family"
            ));
        }
    }
    Ok(())
}

/// Whether `segment` is one segment of a key: `[A-Za-z0-9_-]+`.
pub fn is_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// The one matcher of a family name against a written key: the registry, the resolver, the schema writer, the
/// language server and the roster checks all ask it.
pub struct Family;

impl Family {
    /// Whether `name` has a `*` segment.
    pub fn is_family(name: &str) -> bool {
        name.split('.').any(|segment| segment == "*")
    }

    /// How many `*` segments `name` has.
    pub fn stars(name: &str) -> usize {
        name.split('.').filter(|segment| *segment == "*").count()
    }

    /// Whether `key` is a key of `name`: the same number of segments, each equal or matched by a `*`.
    pub fn matches(name: &str, key: &str) -> bool {
        Family::captures(name, key).is_some()
    }

    /// The segments of `key` that each `*` of `name` stands for, in order; `None` when `key` is not a key of it.
    pub fn captures<'k>(name: &str, key: &'k str) -> Option<Vec<&'k str>> {
        let mut names = name.split('.');
        let mut keys = key.split('.');
        let mut stars = Vec::new();
        loop {
            match (names.next(), keys.next()) {
                (None, None) => return Some(stars),
                (Some("*"), Some(k)) if is_segment(k) => stars.push(k),
                (Some(n), Some(k)) if n == k && n != "*" => {}
                _ => return None,
            }
        }
    }

    /// Whether some key is a key of both names: the same number of segments, each equal or `*` on either side.
    pub fn overlap(a: &str, b: &str) -> bool {
        let (mut a, mut b) = (a.split('.'), b.split('.'));
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(x), Some(y)) if x == y || x == "*" || y == "*" => {}
                _ => return false,
            }
        }
    }

    /// The setting `key` is a key of: an exact name wins over a family that also matches.
    pub fn pick<'a>(
        settings: impl IntoIterator<Item = &'a SettingDescriptor>,
        key: &str,
    ) -> Option<&'a SettingDescriptor> {
        let mut family = None;
        for setting in settings {
            if setting.name == key {
                return Some(setting);
            }
            if family.is_none() && setting.admits_key(key) {
                family = Some(setting);
            }
        }
        family
    }
}

impl SettingDescriptor {
    /// Whether `key` is a key of this setting: its name matches and each `*` takes one of the keys listed for it
    /// (any, when none are listed).
    pub fn admits_key(&self, key: &str) -> bool {
        let Some(captured) = Family::captures(&self.name, key) else {
            return false;
        };
        captured.iter().enumerate().all(|(star, segment)| {
            self.keys
                .get(star)
                .is_none_or(|closed| closed.is_empty() || closed.iter().any(|k| k == segment))
        })
    }
}
