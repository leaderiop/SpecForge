//! The context routes an extension declares (protocol 2.1.0, ADR 0356): for one entity of a subject kind, the ordered
//! sections a role reads, each a named selector: the core whole, the enrich sections in declared order within the
//! route's allowance (ADR 0386).

use serde::{Deserialize, Serialize};

/// The name of the section a pack lists a route's settings in; no declared section may take it.
pub const CONTEXT_SETTINGS: &str = "settings";

/// A context route: what a role reads of one entity of `subject`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextDescriptor {
    /// Kebab case, unique among the extension's context routes; qualified by the extension's short name
    /// (`gsd/executor`).
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The kind of the entity a pack is read for.
    pub subject: String,
    /// The extension's own agent that reads it: the host appends a Context section to its body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The enrich allowance: the tokens of enrich sections a read that states no cap serves beyond its floor (the
    /// subject and the core sections, served whole whatever they cost). Positive for a route with an enrich section,
    /// 0 for a route of core sections only.
    pub budget: u32,
    /// The sections, in the order they are served.
    pub sections: Vec<ContextSectionDescriptor>,
    /// Settings of the extension's own whose values the pack carries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<String>,
    /// The `context` fields of the extension's own a pack of this route is recorded in (ADR 0356 D10); with
    /// [`Self::stale`], or neither.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<ContextRecordDescriptor>,
    /// The code of the extension's own catalogue a record whose pack moved raises, local (`W900`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale: Option<String>,
}

/// A field a route's pack is recorded in, and how the route's subject is found from the entity holding it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRecordDescriptor {
    /// `kind.field`: a field of the extension's own written `context`.
    pub field: String,
    /// A chain from `self` (the recorded entity) to one entity of the route's subject kind: `self`, `self.phase`.
    pub subject: String,
    /// What a stale record's finding suggests; `{entity}`, `{subject}` and `{route}` are filled in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// One section of a context route.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextSectionDescriptor {
    /// A word, unique in the route, never [`CONTEXT_SETTINGS`].
    pub name: String,
    /// A selector in the judged rules' grammar, with its head (`self.phase<decision.phase`, `file("CLAUDE.md")`).
    pub selector: String,
    /// Absent: core.
    #[serde(default)]
    pub tier: ContextTier,
}

/// Whether a section is the floor of a pack or fills what is left.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContextTier {
    /// Never cut: served whole; a stated cap below the core sections is refused.
    #[default]
    Core,
    /// Served whole while it fits the allowance or the stated cap, in declared order.
    Enrich,
}

impl ContextTier {
    /// Its wire name: `core` or `enrich`.
    pub const fn as_str(self) -> &'static str {
        match self {
            ContextTier::Core => "core",
            ContextTier::Enrich => "enrich",
        }
    }
}
