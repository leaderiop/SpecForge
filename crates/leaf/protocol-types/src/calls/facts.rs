//! What a collector observed (ADR 0327): a fact it answers, and a store the session served of them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One collected entity (ADR 0327): a kind that declares `writer: "collector"`, an id in one of its schemes, and the
/// values of its fields, by name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl Fact {
    /// The fact `id` of `kind`, with no title and no field.
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Fact {
            kind: kind.into(),
            id: id.into(),
            title: None,
            fields: BTreeMap::new(),
        }
    }

    /// Its title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The value of its field `name`.
    pub fn field(mut self, name: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.fields.insert(name.into(), value.into());
        self
    }
}

/// A reference an entity writes in a fact scheme that names no fact the session served (ADR 0327 D6): its field (the
/// declared path) and the id written. The graph holds no edge for it and reports nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnheldReference {
    pub field: String,
    pub id: String,
}

/// One fact store the session served (ADR 0327 D5, D6): whose it is, the key it is filed under, the schemes of the
/// kinds of its facts and how many facts it holds. A pass reads it to tell a reference no fact holds from one no
/// store was served for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactStore {
    pub collector: String,
    pub key: String,
    #[serde(default)]
    pub schemes: Vec<String>,
    #[serde(default)]
    pub facts: usize,
}
