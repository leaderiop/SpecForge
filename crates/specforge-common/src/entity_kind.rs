use serde::{Deserialize, Serialize};
use std::fmt;

/// All 16 built-in entity types across core + two official plugins,
/// plus user-defined custom entities via `define` blocks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    // Core (8)
    Spec,
    Invariant,
    Behavior,
    Feature,
    Event,
    TypeDef,
    Port,
    Ref,

    // @specforge/product (5)
    Capability,
    Deliverable,
    Roadmap,
    Library,
    Glossary,

    // @specforge/governance (3)
    Decision,
    Constraint,
    FailureMode,

    // User-defined via `define` blocks
    Custom(String),
}

impl EntityKind {
    /// Parse a block keyword from the DSL into an EntityKind.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "spec" => Some(Self::Spec),
            "invariant" => Some(Self::Invariant),
            "behavior" => Some(Self::Behavior),
            "feature" => Some(Self::Feature),
            "event" => Some(Self::Event),
            "type" => Some(Self::TypeDef),
            "port" => Some(Self::Port),
            "ref" => Some(Self::Ref),
            "capability" => Some(Self::Capability),
            "deliverable" => Some(Self::Deliverable),
            "roadmap" => Some(Self::Roadmap),
            "library" => Some(Self::Library),
            "glossary" => Some(Self::Glossary),
            "decision" => Some(Self::Decision),
            "constraint" => Some(Self::Constraint),
            "failure_mode" => Some(Self::FailureMode),
            _ => None,
        }
    }

    /// Return the DSL keyword for this entity kind.
    pub fn keyword(&self) -> &str {
        match self {
            Self::Spec => "spec",
            Self::Invariant => "invariant",
            Self::Behavior => "behavior",
            Self::Feature => "feature",
            Self::Event => "event",
            Self::TypeDef => "type",
            Self::Port => "port",
            Self::Ref => "ref",
            Self::Capability => "capability",
            Self::Deliverable => "deliverable",
            Self::Roadmap => "roadmap",
            Self::Library => "library",
            Self::Glossary => "glossary",
            Self::Decision => "decision",
            Self::Constraint => "constraint",
            Self::FailureMode => "failure_mode",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// Which module owns this entity kind.
    pub fn module(&self) -> super::Module {
        match self {
            Self::Spec
            | Self::Invariant
            | Self::Behavior
            | Self::Feature
            | Self::Event
            | Self::TypeDef
            | Self::Port
            | Self::Ref => super::Module::Core,

            Self::Capability
            | Self::Deliverable
            | Self::Roadmap
            | Self::Library
            | Self::Glossary => super::Module::Product,

            Self::Decision | Self::Constraint | Self::FailureMode => super::Module::Governance,

            Self::Custom(_) => super::Module::Core,
        }
    }

    /// Returns true if this is a singleton entity (at most one per project).
    pub fn is_singleton(&self) -> bool {
        matches!(self, Self::Spec | Self::Glossary)
    }

    /// Returns true if this entity uses a named identifier (not singleton, not ref).
    pub fn uses_identifier(&self) -> bool {
        !self.is_singleton() && !self.uses_scheme_ref()
    }

    /// Returns true if this entity uses scheme-based IDs.
    pub fn uses_scheme_ref(&self) -> bool {
        matches!(self, Self::Ref)
    }

    /// Returns true if this entity kind can have tests declared (verify/scenario).
    ///
    /// For custom entities, this returns false — use `CompilerConfig::is_testable_with_config`
    /// to check custom entity testability via the define block's `testable` field.
    pub fn is_testable(&self) -> bool {
        matches!(
            self,
            Self::Behavior | Self::Invariant | Self::Event | Self::Constraint | Self::Capability
        )
    }

    /// Returns true if this entity kind can contain scenario blocks.
    pub fn supports_scenario(&self) -> bool {
        matches!(self, Self::Behavior | Self::Capability)
    }

    /// Returns true if this is a custom (user-defined) entity kind.
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }

    /// All entity kinds, in declaration order.
    pub const ALL: [EntityKind; 16] = [
        Self::Spec,
        Self::Invariant,
        Self::Behavior,
        Self::Feature,
        Self::Event,
        Self::TypeDef,
        Self::Port,
        Self::Ref,
        Self::Capability,
        Self::Deliverable,
        Self::Roadmap,
        Self::Library,
        Self::Glossary,
        Self::Decision,
        Self::Constraint,
        Self::FailureMode,
    ];
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.keyword())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Module;

    #[test]
    fn keyword_roundtrip() {
        for kind in EntityKind::ALL {
            let keyword = kind.keyword();
            let parsed = EntityKind::from_keyword(keyword).unwrap();
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn unknown_keyword_returns_none() {
        assert_eq!(EntityKind::from_keyword("thingamajig"), None);
        assert_eq!(EntityKind::from_keyword(""), None);
    }

    #[test]
    fn core_entities_are_core_module() {
        let core_kinds = [
            EntityKind::Spec,
            EntityKind::Invariant,
            EntityKind::Behavior,
            EntityKind::Feature,
            EntityKind::Event,
            EntityKind::TypeDef,
            EntityKind::Port,
            EntityKind::Ref,
        ];
        for kind in core_kinds {
            assert_eq!(kind.module(), Module::Core, "{kind} should be core");
        }
    }

    #[test]
    fn product_entities_are_product_module() {
        let product_kinds = [
            EntityKind::Capability,
            EntityKind::Deliverable,
            EntityKind::Roadmap,
            EntityKind::Library,
            EntityKind::Glossary,
        ];
        for kind in product_kinds {
            assert_eq!(kind.module(), Module::Product, "{kind} should be product");
        }
    }

    #[test]
    fn governance_entities_are_governance_module() {
        let gov_kinds = [
            EntityKind::Decision,
            EntityKind::Constraint,
            EntityKind::FailureMode,
        ];
        for kind in gov_kinds {
            assert_eq!(kind.module(), Module::Governance, "{kind} should be governance");
        }
    }

    #[test]
    fn singleton_entities() {
        assert!(EntityKind::Spec.is_singleton());
        assert!(EntityKind::Glossary.is_singleton());
        assert!(!EntityKind::Invariant.is_singleton());
    }

    #[test]
    fn identifier_entities() {
        // All non-singleton, non-ref entities now use identifiers
        assert!(EntityKind::TypeDef.uses_identifier());
        assert!(EntityKind::Port.uses_identifier());
        assert!(EntityKind::Behavior.uses_identifier());
        assert!(EntityKind::Invariant.uses_identifier());
        assert!(EntityKind::Feature.uses_identifier());
        assert!(EntityKind::Event.uses_identifier());
        assert!(EntityKind::Decision.uses_identifier());
        // Singletons and ref don't use identifiers
        assert!(!EntityKind::Spec.uses_identifier());
        assert!(!EntityKind::Glossary.uses_identifier());
        assert!(!EntityKind::Ref.uses_identifier());
    }

    #[test]
    fn all_has_16_variants() {
        assert_eq!(EntityKind::ALL.len(), 16);
    }

    #[test]
    fn testable_entities() {
        let testable = [
            EntityKind::Behavior,
            EntityKind::Invariant,
            EntityKind::Event,
            EntityKind::Constraint,
            EntityKind::Capability,
        ];
        for kind in testable {
            assert!(kind.is_testable(), "{kind} should be testable");
        }
        let declarative = [
            EntityKind::Feature,
            EntityKind::TypeDef,
            EntityKind::Port,
            EntityKind::Spec,
            EntityKind::Ref,
            EntityKind::Deliverable,
            EntityKind::Library,
            EntityKind::Roadmap,
            EntityKind::Decision,
            EntityKind::FailureMode,
            EntityKind::Glossary,
        ];
        for kind in declarative {
            assert!(!kind.is_testable(), "{kind} should not be testable");
        }
    }

    #[test]
    fn scenario_supporting_entities() {
        assert!(EntityKind::Behavior.supports_scenario());
        assert!(EntityKind::Capability.supports_scenario());
        assert!(!EntityKind::Invariant.supports_scenario());
        assert!(!EntityKind::Event.supports_scenario());
        assert!(!EntityKind::Constraint.supports_scenario());
    }
}
