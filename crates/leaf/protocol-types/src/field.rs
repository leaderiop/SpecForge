//! The descriptor of a field on an entity kind, or of a member of a block field.

use serde::{Deserialize, Serialize};

/// Describes a field on an entity kind.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: String,
    /// A [`FieldType`] name (kept a string so an unknown name costs this
    /// field a diagnostic, not the whole describe payload).
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub edge: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub file_reference: bool,
    /// What an absent value counts as: for an enum, one of its `enum_values`
    /// (W021 otherwise). The host never writes it into the graph; the schema,
    /// the model, the guide and the hover show it.
    #[serde(default)]
    pub default_value: Option<String>,
    /// An enum's values, in order (W021 when an enum declares none).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    #[serde(default)]
    pub inverse_of: Option<String>,
    /// The field states what the entity promises (a contract, a guarantee)
    /// rather than prose: the review document quotes it whole, and with
    /// `headline` it is the entity's statement (ADR 0236).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub normative: bool,
    /// Set on an entity (`true`, or a non-empty value), the entity owes no
    /// obligations of its own: W004, the coverage rule and stats leave it
    /// out (ADR 0004, D2-b). E.g. a specification-only `abstract` flag.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exempts_obligations: bool,
    /// The context export carries this field at the node's top level, as
    /// the line an agent reads first (a contract, a status), instead of
    /// among the normative `fields`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub headline: bool,
    /// The host fills this reference field's edges from type names the
    /// entity writes elsewhere: `type_expressions` (its field types) or
    /// `method_signatures` (its method parameter and return types).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<String>,
    /// What the prove pass reads this field as: `bound` (a fact the solver
    /// assumes) or `claim` (a statement that must follow from the bounds).
    /// Kept a string, as `derived_from`, so an unknown value costs this
    /// field a diagnostic (ADR 0009, A).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_role: Option<String>,
    /// A `block` or `block_list` field's members, each a field of the block (ADR 0285), nested to any depth. Empty
    /// on a `block`: an open map of named text entries, as before. A member's `required` is its presence in each
    /// written item.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<FieldDescriptor>,
    /// The kinds a `reference` or `reference_list` may name when it names more than one (ADR 0285); exclusive
    /// with `target_kind`, which names one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_kinds: Vec<String>,
    /// An `integer` field's least value, inclusive (E061 below it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    /// An `integer` field's greatest value, inclusive (E061 above it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    /// Where a `file_reference` field's relative paths start: a [`FileBase`](crate::FileBase) name, `spec` (the
    /// spec root, the default) or `project` (the directory holding `specforge.json`). Kept a string, as
    /// `derived_from`, so an unknown base costs this field its base (W021).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_base: Option<String>,
    /// Who writes the field (protocol 1.8.0, ADR 0287): absent, its author; `computed`, a compute pass of the
    /// declaring extension, never a `.spec` file; `harness`, only a host write a command or collector plan
    /// makes, recorded in `specforge-harness`. Kept a string, as `proof_role`, so an unknown writer costs this
    /// field a diagnostic (W021), not the describe payload ([`FieldWriter`](crate::FieldWriter)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writer: Option<String>,
    /// A value set at the baseline never changes (ADR 0287): an immutable field unset there may be set once.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub immutable: bool,
    /// The baseline list is a prefix of the current one (ADR 0287), items compared as resolved values.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub append_only: bool,
}

impl FieldDescriptor {
    /// This field and, depth first, every member of it, each with its declared member path (`requires`,
    /// `requires.phase`): `path` is this field's own (ADR 0285).
    pub fn for_each_path<'a>(&'a self, path: &str, f: &mut dyn FnMut(String, &'a FieldDescriptor)) {
        f(path.to_string(), self);
        for member in &self.members {
            member.for_each_path(&format!("{path}.{}", member.name), f);
        }
    }
}
