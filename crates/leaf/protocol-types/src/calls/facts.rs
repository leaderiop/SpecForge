//! What a collector observed (ADR 0327): a fact it answers, and a store the session served of them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Where a fact, or a name it holds, is written (ADR 0346 D1): a project-relative path with `/`, 1-based lines,
/// 1-based byte columns, end exclusive (the host `SourceSpan`'s convention). A pack converts a reader's 0-based row
/// and byte column by adding one to each.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FactLocation {
    pub file: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// One collected entity (ADR 0327): a kind that declares `writer: "collector"`, an id in one of its schemes, and the
/// values of its fields, by name; where it is written and where each name it holds is (ADR 0346 D1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, serde_json::Value>,
    /// Where the fact is written, once per place (a cfg twin is one fact at two places). Empty: it has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub at: Vec<FactLocation>,
    /// Where each reference a field holds is written (its name span, inside any quotes): field → id → places.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub named_at: BTreeMap<String, BTreeMap<String, Vec<FactLocation>>>,
}

impl Fact {
    /// The fact `id` of `kind`, with no title and no field.
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Fact {
            kind: kind.into(),
            id: id.into(),
            title: None,
            fields: BTreeMap::new(),
            at: Vec::new(),
            named_at: BTreeMap::new(),
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

    /// One more place it is written.
    pub fn at(mut self, place: FactLocation) -> Self {
        self.at.push(place);
        self
    }

    /// `id` named in its field `field` (appended to the field's array once) and written at `place`: a rename of the
    /// entity `id` names rewrites the text there (ADR 0346 D7).
    pub fn name(mut self, field: &str, id: &str, place: FactLocation) -> Self {
        let value = self
            .fields
            .entry(field.to_string())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        if !value.is_array() {
            *value = serde_json::Value::Array(Vec::new());
        }
        let items = value.as_array_mut().expect("made an array");
        if !items.iter().any(|item| item.as_str() == Some(id)) {
            items.push(serde_json::Value::String(id.to_string()));
        }
        self.named_at
            .entry(field.to_string())
            .or_default()
            .entry(id.to_string())
            .or_default()
            .push(place);
        self
    }
}

/// A reference an entity writes in a fact scheme that names no fact the session served (ADR 0327 D6): its field (the
/// declared path), the id written and where it is written (protocol 1.15.0, ADR 0329 D5: inside its quotes). The
/// graph holds no edge for it and reports nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnheldReference {
    pub field: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<super::PassSpan>,
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

/// Why a fact's reference is dangling (ADR 0346 D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DanglingReason {
    /// It names no entity.
    Unresolved,
    /// Its short name is the suffix of several entities' paths.
    Ambiguous,
    /// It names an entity of a kind its field does not take (`actual`).
    Mistyped,
}

/// A fact's reference that names no entity, names one of another kind, or names several (ADR 0346 D4): no core
/// diagnostic; the extension declaring the holder's kind judges it. Its field (the declared path), the id written, where
/// it is written (its name span), why, and what the kernel would have suggested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DanglingReference {
    pub field: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<super::PassSpan>,
    pub reason: DanglingReason,
    /// The kind of the entity it names, when `mistyped`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub did_you_mean: Option<String>,
    /// The paths it could name: the entities an ambiguous short name matches, or that bear its name elsewhere.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(line: usize) -> FactLocation {
        FactLocation {
            file: "src/a.rs".into(),
            start_line: line,
            start_col: 5,
            end_line: line,
            end_col: 8,
        }
    }

    #[test]
    fn fact_name_appends_the_id_once_and_records_each_place() {
        let fact = Fact::new("item", "rust:a::f")
            .name("names", "TR1", place(1))
            .name("names", "TR2", place(2))
            .name("names", "TR1", place(3));

        assert_eq!(fact.fields["names"], serde_json::json!(["TR1", "TR2"]));
        assert_eq!(fact.named_at["names"]["TR1"], [place(1), place(3)]);
        assert_eq!(fact.named_at["names"]["TR2"], [place(2)]);
        assert!(fact.at.is_empty());
        assert_eq!(fact.at(place(9)).at, [place(9)]);
    }
}
