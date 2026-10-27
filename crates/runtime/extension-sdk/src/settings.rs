//! Declaring settings, presets and secrets (ADR 0288).

use serde_json::Value;
use specforge_protocol_types::{
    FieldType, PresetDescriptor, PresetEntry, SettingDescriptor, ValidationSeverity,
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
