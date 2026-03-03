use serde::{Deserialize, Serialize};
use std::fmt;

/// All 20 edge types across core + two official plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    // Core (9)
    References,
    Implements,
    Produces,
    Consumes,
    UsesType,
    UsesPort,
    Enforces,
    Imports,
    LinksTo,

    // @specforge/product (7)
    TracesTo,
    Bundles,
    BuiltFrom,
    DependsOn,
    Provides,
    DefinesPort,
    Schedules,

    // @specforge/governance (4)
    Protects,
    ShapedBy,
    Constrains,
    Mitigates,

    // Plugin-contributed edge type; label stored in GraphEdge::enhanced_label
    Enhanced,
}

impl EdgeType {
    /// Parse a field name into the corresponding edge type, if it maps to a reference edge.
    /// Returns `None` for fields that don't create edges (e.g., `contract`, `status`).
    pub fn from_field_name(field: &str) -> Option<Self> {
        match field {
            "invariants" => Some(Self::References),
            "behaviors" => Some(Self::Implements),
            "produces" | "events" => Some(Self::Produces),
            "consumers" => Some(Self::Consumes),
            "types" => Some(Self::UsesType),
            "ports" => Some(Self::UsesPort),
            "enforced_by" => Some(Self::Enforces),
            "refs" | "links" => Some(Self::LinksTo),
            "features" => Some(Self::TracesTo),
            "capabilities" => Some(Self::Bundles),
            "libraries" => Some(Self::BuiltFrom),
            "depends_on" => Some(Self::DependsOn),
            "provides" => Some(Self::Provides),
            "ports_defined" => Some(Self::DefinesPort),
            "adrs" => Some(Self::ShapedBy),
            "protects" => Some(Self::Protects),
            "constrains" => Some(Self::Constrains),
            "mitigates" | "invariant" => Some(Self::Mitigates),
            _ => None,
        }
    }

    /// Human-readable label for this edge type.
    pub fn label(&self) -> &'static str {
        match self {
            Self::References => "references",
            Self::Implements => "implements",
            Self::Produces => "produces",
            Self::Consumes => "consumes",
            Self::UsesType => "uses_type",
            Self::UsesPort => "uses_port",
            Self::Enforces => "enforces",
            Self::Imports => "imports",
            Self::LinksTo => "links_to",
            Self::TracesTo => "traces_to",
            Self::Bundles => "bundles",
            Self::BuiltFrom => "built_from",
            Self::DependsOn => "depends_on",
            Self::Provides => "provides",
            Self::DefinesPort => "defines_port",
            Self::Schedules => "schedules",
            Self::Protects => "protects",
            Self::ShapedBy => "shaped_by",
            Self::Constrains => "constrains",
            Self::Mitigates => "mitigates",
            Self::Enhanced => "enhanced",
        }
    }

    /// Which module owns this edge type.
    pub fn module(&self) -> super::Module {
        match self {
            Self::References
            | Self::Implements
            | Self::Produces
            | Self::Consumes
            | Self::UsesType
            | Self::UsesPort
            | Self::Enforces
            | Self::Imports
            | Self::LinksTo => super::Module::Core,

            Self::TracesTo
            | Self::Bundles
            | Self::BuiltFrom
            | Self::DependsOn
            | Self::Provides
            | Self::DefinesPort
            | Self::Schedules => super::Module::Product,

            Self::Protects | Self::ShapedBy | Self::Constrains | Self::Mitigates => {
                super::Module::Governance
            }

            Self::Enhanced => super::Module::Core,
        }
    }

    /// Returns the expected target entity kind for this edge type.
    /// Used by the resolver to determine what type of entity a reference
    /// in a given field should resolve to.
    pub fn target_kind(&self) -> Option<super::EntityKind> {
        use super::EntityKind;
        match self {
            Self::References => Some(EntityKind::Invariant),
            Self::Implements => Some(EntityKind::Behavior),
            Self::Produces => Some(EntityKind::Event),
            Self::Consumes => Some(EntityKind::Behavior),
            Self::UsesType => Some(EntityKind::TypeDef),
            Self::UsesPort => Some(EntityKind::Port),
            Self::Enforces => Some(EntityKind::Behavior),
            Self::Imports => None,  // file-level, not entity references
            Self::LinksTo => Some(EntityKind::Ref),
            Self::TracesTo => Some(EntityKind::Feature),
            Self::Bundles => Some(EntityKind::Capability),
            Self::BuiltFrom => Some(EntityKind::Library),
            Self::DependsOn => Some(EntityKind::Library),
            Self::Provides => Some(EntityKind::Feature),
            Self::DefinesPort => Some(EntityKind::Port),
            Self::Schedules => None, // can target feature or deliverable
            Self::Protects => Some(EntityKind::Invariant),
            Self::ShapedBy => Some(EntityKind::Decision),
            Self::Constrains => None, // can target behavior or invariant
            Self::Mitigates => Some(EntityKind::Invariant),
            Self::Enhanced => None, // target kind stored in enhancement metadata
        }
    }

    /// Returns true if this is a soft cross-module edge (I004).
    pub fn is_soft_reference(&self) -> bool {
        matches!(self, Self::ShapedBy)
    }

    /// All edge types, in declaration order.
    pub const ALL: [EdgeType; 21] = [
        Self::References,
        Self::Implements,
        Self::Produces,
        Self::Consumes,
        Self::UsesType,
        Self::UsesPort,
        Self::Enforces,
        Self::Imports,
        Self::LinksTo,
        Self::TracesTo,
        Self::Bundles,
        Self::BuiltFrom,
        Self::DependsOn,
        Self::Provides,
        Self::DefinesPort,
        Self::Schedules,
        Self::Protects,
        Self::ShapedBy,
        Self::Constrains,
        Self::Mitigates,
        Self::Enhanced,
    ];
}

impl fmt::Display for EdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_name_mapping() {
        assert_eq!(EdgeType::from_field_name("invariants"), Some(EdgeType::References));
        assert_eq!(EdgeType::from_field_name("behaviors"), Some(EdgeType::Implements));
        assert_eq!(EdgeType::from_field_name("types"), Some(EdgeType::UsesType));
        assert_eq!(EdgeType::from_field_name("adrs"), Some(EdgeType::ShapedBy));
        assert_eq!(EdgeType::from_field_name("contract"), None);
    }

    #[test]
    fn all_has_21_variants() {
        assert_eq!(EdgeType::ALL.len(), 21);
    }

    #[test]
    fn soft_reference() {
        assert!(EdgeType::ShapedBy.is_soft_reference());
        assert!(!EdgeType::References.is_soft_reference());
    }

    #[test]
    fn target_kind_mapping() {
        use crate::EntityKind;
        assert_eq!(EdgeType::References.target_kind(), Some(EntityKind::Invariant));
        assert_eq!(EdgeType::Implements.target_kind(), Some(EntityKind::Behavior));
        assert_eq!(EdgeType::TracesTo.target_kind(), Some(EntityKind::Feature));
        assert_eq!(EdgeType::ShapedBy.target_kind(), Some(EntityKind::Decision));
        assert_eq!(EdgeType::Mitigates.target_kind(), Some(EntityKind::Invariant));
        // Multi-target edges return None
        assert_eq!(EdgeType::Schedules.target_kind(), None);
        assert_eq!(EdgeType::Constrains.target_kind(), None);
    }

    #[test]
    fn enforced_by_maps_to_enforces() {
        assert_eq!(
            EdgeType::from_field_name("enforced_by"),
            Some(EdgeType::Enforces)
        );
    }

    #[test]
    fn enforces_targets_behavior() {
        use crate::EntityKind;
        assert_eq!(EdgeType::Enforces.target_kind(), Some(EntityKind::Behavior));
    }
}
