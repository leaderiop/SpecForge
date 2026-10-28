//! The edit plan (ADR 0289, protocol 1.9.0): what a writing command or collector answers to write. The host
//! checks it, plans it with the typed edit engine and applies it all or nothing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A file's revision: `sha256:<64 hex>` of its bytes as the host compiled them, or `absent`. Empty means the op
/// named none (the host refuses it `invalid_plan`, not as a decode error).
pub type Revision = String;

/// What a writing command or collector answers to write: operations the host checks, plans and applies in order,
/// each over the texts the ones before it left.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditPlan {
    pub ops: Vec<PlanOp>,
    /// One line the CLI prints above the diff; absent, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// A `verify` statement a `create` op writes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanObligation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub text: String,
}

/// What a verdict cites: an entity, a file or a test. An object naming more than one is no cite: it is not read as
/// the first it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Cite {
    Entity { entity: String },
    File { file: String },
    Test { test: String },
}

/// One operation of an [`EditPlan`]. `field` is a path: `status`, `completion.at`, `checks[0].status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanOp {
    Create {
        kind: String,
        id: String,
        title: String,
        file: String,
        #[serde(default)]
        revision: Revision,
        #[serde(default)]
        fields: BTreeMap<String, Value>,
        #[serde(default)]
        verify: Vec<PlanObligation>,
    },
    Set {
        entity: String,
        field: String,
        value: Value,
        #[serde(default)]
        revision: Revision,
    },
    Unset {
        entity: String,
        field: String,
        #[serde(default)]
        revision: Revision,
    },
    Append {
        entity: String,
        field: String,
        value: Value,
        #[serde(default)]
        revision: Revision,
    },
    Remove {
        entity: String,
        field: String,
        index: usize,
        #[serde(default)]
        revision: Revision,
    },
    Link {
        entity: String,
        field: String,
        target: String,
        #[serde(default)]
        revision: Revision,
    },
    Unlink {
        entity: String,
        field: String,
        target: String,
        #[serde(default)]
        revision: Revision,
    },
    Retitle {
        entity: String,
        title: String,
        #[serde(default)]
        revision: Revision,
    },
    AddVerify {
        entity: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        text: String,
        #[serde(default)]
        revision: Revision,
    },
    RemoveVerify {
        entity: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        text: String,
        #[serde(default)]
        revision: Revision,
    },
    Delete {
        entity: String,
        #[serde(default)]
        revision: Revision,
        #[serde(default)]
        unlink_references: bool,
    },
    /// The declaration and every reference; the move record is written with the ledger.
    Rename {
        entity: String,
        to: String,
        #[serde(default)]
        revision: Revision,
    },
    /// A judged rule's verdict. It has no file revision: its `basis` is its revision.
    SetVerdict {
        entity: String,
        rule: String,
        outcome: String,
        judge: String,
        model: String,
        #[serde(default)]
        findings: Vec<Value>,
        #[serde(default)]
        cited: Vec<Cite>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        values: BTreeMap<String, Value>,
        #[serde(default)]
        basis: BTreeMap<String, String>,
    },
}
