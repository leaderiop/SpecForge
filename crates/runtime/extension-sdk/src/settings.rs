//! Declaring settings, presets and secrets (ADR 0288).

use std::cell::RefCell;

use serde::Deserialize;
use serde_json::{Map, Value};
use specforge_protocol_types::{
    FieldType, PresetDescriptor, PresetEntry, SettingDescriptor, SettingsItem, ValidationSeverity,
};

use crate::FieldBuilder;

/// Builder for a [`SettingDescriptor`]: a dotted name, or a family with `*` segments.
pub struct SettingBuilder(pub(crate) SettingDescriptor);

impl SettingBuilder {
    pub(crate) fn new(name: &str) -> Self {
        Self(SettingDescriptor {
            name: name.to_string(),
            field_type: FieldType::String.as_str().to_string(),
            ..Default::default()
        })
    }
    fn typed(&mut self, t: FieldType) -> &mut Self {
        self.0.field_type = t.as_str().to_string();
        self
    }
    /// A bool: the only type that can [`gate`](Self::gates) codes.
    pub fn bool(&mut self) -> &mut Self {
        self.typed(FieldType::Bool)
    }
    /// A free string (the default type).
    pub fn string(&mut self) -> &mut Self {
        self.typed(FieldType::String)
    }
    /// A whole number, within [`minimum`](Self::minimum) and [`maximum`](Self::maximum).
    pub fn integer(&mut self) -> &mut Self {
        self.typed(FieldType::Integer)
    }
    pub fn string_list(&mut self) -> &mut Self {
        self.typed(FieldType::StringList)
    }
    /// A word from `values`, in order.
    pub fn enum_values(&mut self, values: &[&str]) -> &mut Self {
        self.0.enum_values = values.iter().map(|v| v.to_string()).collect();
        self.typed(FieldType::Enum)
    }
    /// A list of words from `values`.
    pub fn enum_list(&mut self, values: &[&str]) -> &mut Self {
        self.0.enum_values = values.iter().map(|v| v.to_string()).collect();
        self.typed(FieldType::EnumList)
    }
    /// An object of named members, declared as a field's are.
    pub fn block(&mut self) -> &mut Self {
        self.typed(FieldType::Block)
    }
    /// A list of objects of named members.
    pub fn block_list(&mut self) -> &mut Self {
        self.typed(FieldType::BlockList)
    }
    /// A member of a [`block`](Self::block) or [`block_list`](Self::block_list).
    pub fn member(&mut self, name: &str, f: impl FnOnce(&mut FieldBuilder)) -> &mut Self {
        let mut b = FieldBuilder::new(name);
        f(&mut b);
        self.0.members.push(b.0);
        self
    }
    pub fn minimum(&mut self, n: i64) -> &mut Self {
        self.0.minimum = Some(n);
        self
    }
    pub fn maximum(&mut self, n: i64) -> &mut Self {
        self.0.maximum = Some(n);
        self
    }
    /// For a family, the keys each `*` may take, in order; an empty list takes any segment.
    pub fn keys(&mut self, keys: &[&[&str]]) -> &mut Self {
        self.0.keys = keys
            .iter()
            .map(|k| k.iter().map(|s| s.to_string()).collect())
            .collect();
        self
    }
    /// The value when nothing sets it. Absent: the setting is unset.
    pub fn default(&mut self, value: impl Into<Value>) -> &mut Self {
        self.0.default = Some(value.into());
        self
    }
    pub fn describe(&mut self, text: &str) -> &mut Self {
        self.0.description = Some(text.to_string());
        self
    }
    /// Local codes reported by no source while this bool setting is false.
    pub fn gates(&mut self, codes: &[&str]) -> &mut Self {
        self.0.gates = codes.iter().map(|c| c.to_string()).collect();
        self
    }
    /// Local codes reported by no source while this bool setting is true (ADR 0305).
    pub fn gates_when_on(&mut self, codes: &[&str]) -> &mut Self {
        self.0.gates_when_on = codes.iter().map(|c| c.to_string()).collect();
        self
    }
}

impl From<SettingBuilder> for SettingDescriptor {
    fn from(b: SettingBuilder) -> Self {
        b.0
    }
}

/// Builder for a [`PresetDescriptor`]: what each value of an enum or bool setting presets.
pub struct PresetBuilder(pub(crate) PresetDescriptor);

impl PresetBuilder {
    pub(crate) fn new(source: &str) -> Self {
        Self(PresetDescriptor {
            source: source.to_string(),
            ..Default::default()
        })
    }
    /// What the source's value `value` (`true` or `false` for a bool) presets.
    pub fn when(&mut self, value: &str, f: impl FnOnce(&mut PresetEntryBuilder)) -> &mut Self {
        let mut b = PresetEntryBuilder(self.0.values.remove(value).unwrap_or_default());
        f(&mut b);
        self.0.values.insert(value.to_string(), b.0);
        self
    }
}

/// Builder for a [`PresetEntry`].
pub struct PresetEntryBuilder(PresetEntry);

impl PresetEntryBuilder {
    /// Preset one of this extension's settings.
    pub fn set(&mut self, name: &str, value: impl Into<Value>) -> &mut Self {
        self.0.settings.insert(name.to_string(), value.into());
        self
    }
    /// Raise one of this extension's codes (by its local code) to `severity`; never lowered.
    pub fn raise(&mut self, code: &str, severity: ValidationSeverity) -> &mut Self {
        self.0.severities.insert(code.to_string(), severity);
        self
    }
}

/// What the host sent with the call being answered (ADR 0288): the extension's own settings, and whether each of
/// its secrets is set. Read it with [`Settings::current`] in a pass, a validator, a command, a tool or any other
/// handler; a tool's arguments never hold it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    values: Map<String, Value>,
    secrets: Map<String, Value>,
}

thread_local! {
    static CURRENT: RefCell<Settings> = RefCell::new(Settings::default());
}

impl Settings {
    /// The settings of the call being answered: the host's, else each declared default. Outside a call: none.
    pub fn current() -> Settings {
        CURRENT.with(|current| current.borrow().clone())
    }

    /// The value of `name` (a dotted name, a family's key as written), `None` when it is unset.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    /// `name` as a bool, `None` when unset or not one.
    pub fn bool(&self, name: &str) -> Option<bool> {
        self.get(name)?.as_bool()
    }

    /// `name` as a string, `None` when unset or not one.
    pub fn string(&self, name: &str) -> Option<&str> {
        self.get(name)?.as_str()
    }

    /// `name` as a whole number, `None` when unset or not one.
    pub fn integer(&self, name: &str) -> Option<i64> {
        self.get(name)?.as_i64()
    }

    /// Every setting under `prefix.` (a family's keys), by the rest of its name.
    pub fn under<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = (&'a str, &'a Value)> {
        self.values.iter().filter_map(move |(name, value)| {
            Some((name.strip_prefix(prefix)?.strip_prefix('.')?, value))
        })
    }

    /// Whether the secret `name` is set in the environment. The value is never sent.
    pub fn secret_set(&self, name: &str) -> bool {
        self.secrets
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

/// The part of a call's input the host adds.
#[derive(Deserialize, Default)]
struct Carried {
    #[serde(default)]
    settings: Option<Map<String, Value>>,
    #[serde(default)]
    secrets: Option<Map<String, Value>>,
}

/// Holds a call's settings as the current ones until dropped.
pub(crate) struct Entered(Settings);

impl Drop for Entered {
    fn drop(&mut self) {
        CURRENT.with(|current| *current.borrow_mut() = std::mem::take(&mut self.0));
    }
}

/// Make the settings `input` carries the current ones (each `declared` default for a setting it does not carry)
/// until the guard drops.
pub(crate) fn enter(declared: &[SettingsItem], input: &[u8]) -> Entered {
    let carried: Carried = serde_json::from_slice(input).unwrap_or_default();
    let mut values = carried.settings.unwrap_or_default();
    for item in declared {
        if let SettingsItem::Setting(setting) = item
            && let Some(default) = &setting.default
            && !setting.name.contains('*')
        {
            values
                .entry(setting.name.clone())
                .or_insert_with(|| default.clone());
        }
    }
    let now = Settings {
        values,
        secrets: carried.secrets.unwrap_or_default(),
    };
    Entered(CURRENT.with(|current| std::mem::replace(&mut *current.borrow_mut(), now)))
}
