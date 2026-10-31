//! Judged rules (ADR 0290, protocol 1.9.0): a rule whose check is `judged` holds while a fresh verdict from its
//! judge says so. The declaration an extension makes, and the state the host reports for one rule on one entity.

use serde::{Deserialize, Serialize};

/// A judged rule's declaration, on a rule whose `check` is `judged`. The rule's `code` is its name; its
/// `target_kind`, `field`, `edge_type`, `constraint` and `wasm_function` are not read (W147).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JudgedDescriptor {
    /// The agent contribution of this extension that judges (a word).
    pub judge: String,
    /// The outcomes a verdict may record (words, each once), in order.
    pub outcomes: Vec<String>,
    /// The outcomes that pass: a non-empty subset of `outcomes`.
    pub passing: Vec<String>,
    /// Per target kind, its inputs and exemptions.
    pub targets: Vec<JudgedTarget>,
    /// `kind.field`: `verdict`-written fields set from the verdict's `values` block.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    /// `kind.field`: `verdict`-written fields that hold the verdict's outcome (their enum is `outcomes`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcome_fields: Vec<String>,
}

/// What a judged rule reads of the entities of one kind.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JudgedTarget {
    pub kind: String,
    /// Named inputs, in order; `cited` is reserved.
    pub inputs: Vec<JudgedInput>,
    /// An entity that writes any of these fields is not judged: no verdict is owed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unless_set: Vec<String>,
}

/// One named input: what a selector (ADR 0290) picks from the project.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JudgedInput {
    pub name: String,
    pub selector: String,
}

/// One judged rule's state on one entity, as passes, a command's graph and the outputs read it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Judgement {
    /// The rule's qualified code.
    pub rule: String,
    /// A [`JudgementState`](crate::JudgementState) name.
    pub state: String,
    /// Fresh, recorded and its outcome passing.
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// The inputs that changed since the verdict (stale only), by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<String>,
    /// The files its `file()` inputs and `cited` read, with their current digests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<JudgedFile>,
    /// The tests the verdict cites (`cited_tests`), as written; a pass reads them with the project's recorded tests
    /// (protocol 1.12.0, ADR 0304).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cited_tests: Vec<String>,
}

/// A file a judgement read, and its digest now.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct JudgedFile {
    pub path: String,
    pub digest: String,
}
