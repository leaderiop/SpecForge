use crate::{EdgeType, EntityKind};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Which module owns an entity or edge type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Module {
    Core,
    Product,
    Governance,
}

impl Module {
    /// The npm-style package name for plugin modules.
    /// Core has no package name since it's built-in.
    pub fn package_name(&self) -> Option<&'static str> {
        match self {
            Self::Core => None,
            Self::Product => Some("@specforge/product"),
            Self::Governance => Some("@specforge/governance"),
        }
    }

    /// Parse a plugin package name into a Module.
    pub fn from_package_name(name: &str) -> Option<Self> {
        match name {
            "@specforge/product" => Some(Self::Product),
            "@specforge/governance" => Some(Self::Governance),
            _ => None,
        }
    }

    /// Returns true if this module is a plugin (not core).
    pub fn is_plugin(&self) -> bool {
        !matches!(self, Self::Core)
    }

    /// Number of entity kinds owned by this module.
    pub fn entity_count(&self) -> usize {
        EntityKind::ALL.iter().filter(|k| k.module() == *self).count()
    }

    /// Number of edge types owned by this module.
    pub fn edge_count(&self) -> usize {
        EdgeType::ALL.iter().filter(|e| e.module() == *self).count()
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core => f.write_str("core"),
            Self::Product => f.write_str("@specforge/product"),
            Self::Governance => f.write_str("@specforge/governance"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_roundtrip() {
        assert_eq!(Module::from_package_name("@specforge/product"), Some(Module::Product));
        assert_eq!(
            Module::from_package_name("@specforge/governance"),
            Some(Module::Governance),
        );
        assert_eq!(Module::from_package_name("@specforge/unknown"), None);
    }

    #[test]
    fn core_is_not_plugin() {
        assert!(!Module::Core.is_plugin());
        assert!(Module::Product.is_plugin());
        assert!(Module::Governance.is_plugin());
    }

    #[test]
    fn entity_counts() {
        assert_eq!(Module::Core.entity_count(), 8);
        assert_eq!(Module::Product.entity_count(), 5);
        assert_eq!(Module::Governance.entity_count(), 3);
    }

    #[test]
    fn edge_counts() {
        assert_eq!(Module::Core.edge_count(), 10);
        assert_eq!(Module::Product.edge_count(), 7);
        assert_eq!(Module::Governance.edge_count(), 4);
    }

    #[test]
    fn testable_kinds_are_subset_of_module_kinds() {
        use crate::EntityKind;
        for kind in EntityKind::ALL {
            if kind.is_testable() {
                // Every testable kind should belong to some module
                let module = kind.module();
                let module_kinds: Vec<_> =
                    EntityKind::ALL.iter().filter(|k| k.module() == module).collect();
                assert!(
                    module_kinds.contains(&&kind),
                    "{kind} is testable but not in module {module}'s entity kinds",
                );
            }
        }
    }
}
