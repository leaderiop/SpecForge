use crate::EntityId;
use serde::{Deserialize, Serialize};

/// Map of field names to values within an entity block.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldMap {
    pub entries: Vec<(String, FieldValue)>,
}

impl FieldMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, value: FieldValue) {
        self.entries.push((key.into(), value));
    }

    pub fn get(&self, key: &str) -> Option<&FieldValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &FieldValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// A field value in an entity block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldValue {
    /// A plain string: `"text"` or `"""multiline"""`
    String(String),

    /// An integer: `42`, `90`
    Integer(i64),

    /// A boolean: `true`, `false`
    Bool(bool),

    /// An enum/identifier value: `high`, `outbound`, `in_progress`
    Enum(String),

    /// A single entity reference: `BEH-SF-010`, `INV-SF-1`
    Reference(EntityId),

    /// A list of entity references: `[INV-SF-1, INV-SF-2]`
    ReferenceList(Vec<EntityId>),

    /// A list of strings: `["test1", "test2"]`
    StringList(Vec<String>),

    /// A list of enum/identifier values: `[web, cli, api]`
    EnumList(Vec<String>),

    /// A nested block: `post_mitigation { ... }`, `payload { ... }`
    Block(FieldMap),

    /// A verify statement: `verify unit "description"`
    VerifyList(Vec<VerifyStatement>),

    /// A list of scenario blocks: `scenario "title" { given/when/then }`
    ScenarioList(Vec<Scenario>),
}

/// A verify statement within a behavior block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifyStatement {
    pub kind: VerifyKind,
    pub description: String,
}

/// The kind of verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VerifyKind {
    Unit,
    Integration,
    Property,
    Load,
    E2e,
}

impl VerifyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Integration => "integration",
            Self::Property => "property",
            Self::Load => "load",
            Self::E2e => "e2e",
        }
    }
}

/// A scenario block within a behavior or capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub title: String,
    pub steps: Vec<ScenarioStep>,
    pub span: crate::SourceSpan,
}

/// A single step in a scenario (given, when, or then).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub kind: ScenarioStepKind,
    pub description: String,
    pub span: crate::SourceSpan,
}

/// The kind of scenario step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScenarioStepKind {
    Given,
    When,
    Then,
}

impl std::str::FromStr for VerifyKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unit" => Ok(Self::Unit),
            "integration" => Ok(Self::Integration),
            "property" => Ok(Self::Property),
            "load" => Ok(Self::Load),
            "e2e" => Ok(Self::E2e),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_map_get() {
        let mut map = FieldMap::new();
        map.insert("risk", FieldValue::Enum("high".to_string()));
        map.insert("guarantee", FieldValue::String("must hold".to_string()));

        assert_eq!(
            map.get("risk"),
            Some(&FieldValue::Enum("high".to_string())),
        );
        assert_eq!(map.get("missing"), None);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn verify_kind_roundtrip() {
        let kinds = ["unit", "integration", "property", "load", "e2e"];
        for name in kinds {
            let kind: VerifyKind = name.parse().unwrap();
            assert_eq!(kind.as_str(), name);
        }
        assert!("unknown".parse::<VerifyKind>().is_err());
    }
}
