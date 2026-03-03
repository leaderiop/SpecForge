use crate::{EdgeType, EntityKind, Module, ValidationCode};

/// A manifest describing a plugin's contributions to the entity model.
#[derive(Debug, Clone)]
pub struct PluginManifest {
    /// The npm-style package name (e.g., `@specforge/product`).
    pub package: String,
    /// Which module this plugin represents.
    pub module: Module,
    /// Entity kinds contributed by this plugin.
    pub entity_kinds: Vec<EntityKind>,
    /// Edge types contributed by this plugin.
    pub edge_types: Vec<EdgeType>,
    /// Validation codes contributed by this plugin.
    pub validation_codes: Vec<ValidationCode>,
    /// Entity kinds that are testable (require test coverage).
    pub testable_kinds: Vec<EntityKind>,
}

impl PluginManifest {
    /// Manifest for the `@specforge/product` plugin.
    pub fn product() -> Self {
        Self {
            package: "@specforge/product".to_string(),
            module: Module::Product,
            entity_kinds: vec![
                EntityKind::Capability,
                EntityKind::Deliverable,
                EntityKind::Roadmap,
                EntityKind::Library,
                EntityKind::Glossary,
            ],
            edge_types: vec![
                EdgeType::TracesTo,
                EdgeType::Bundles,
                EdgeType::BuiltFrom,
                EdgeType::DependsOn,
                EdgeType::Provides,
                EdgeType::DefinesPort,
                EdgeType::Schedules,
            ],
            validation_codes: vec![
                ValidationCode::E007,
                ValidationCode::E008,
                ValidationCode::E009,
                ValidationCode::W002,
                ValidationCode::W008,
                ValidationCode::W009,
                ValidationCode::W010,
                ValidationCode::W011,
                ValidationCode::W018,
                ValidationCode::I006,
            ],
            testable_kinds: vec![EntityKind::Capability],
        }
    }

    /// Manifest for the `@specforge/governance` plugin.
    pub fn governance() -> Self {
        Self {
            package: "@specforge/governance".to_string(),
            module: Module::Governance,
            entity_kinds: vec![
                EntityKind::Decision,
                EntityKind::Constraint,
                EntityKind::FailureMode,
            ],
            edge_types: vec![
                EdgeType::Protects,
                EdgeType::ShapedBy,
                EdgeType::Constrains,
                EdgeType::Mitigates,
            ],
            validation_codes: vec![
                ValidationCode::E005,
                ValidationCode::W005,
                ValidationCode::W006,
                ValidationCode::W019,
                ValidationCode::I001,
            ],
            testable_kinds: vec![EntityKind::Constraint],
        }
    }

    /// Look up the manifest for a given module. Returns `None` for `Module::Core`.
    pub fn for_module(module: Module) -> Option<Self> {
        match module {
            Module::Core => None,
            Module::Product => Some(Self::product()),
            Module::Governance => Some(Self::governance()),
        }
    }

    /// Look up the manifest by package name.
    pub fn from_package_name(name: &str) -> Option<Self> {
        Module::from_package_name(name).and_then(Self::for_module)
    }

    /// Number of entity kinds in this plugin.
    pub fn entity_count(&self) -> usize {
        self.entity_kinds.len()
    }

    /// Number of edge types in this plugin.
    pub fn edge_count(&self) -> usize {
        self.edge_types.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_manifest() {
        let m = PluginManifest::product();
        assert_eq!(m.package, "@specforge/product");
        assert_eq!(m.module, Module::Product);
        assert_eq!(m.entity_count(), 5);
        assert_eq!(m.edge_count(), 7);
        assert_eq!(m.testable_kinds, vec![EntityKind::Capability]);
    }

    #[test]
    fn governance_manifest() {
        let m = PluginManifest::governance();
        assert_eq!(m.package, "@specforge/governance");
        assert_eq!(m.module, Module::Governance);
        assert_eq!(m.entity_count(), 3);
        assert_eq!(m.edge_count(), 4);
        assert_eq!(m.testable_kinds, vec![EntityKind::Constraint]);
    }

    #[test]
    fn for_module_lookup() {
        assert!(PluginManifest::for_module(Module::Core).is_none());
        assert!(PluginManifest::for_module(Module::Product).is_some());
        assert!(PluginManifest::for_module(Module::Governance).is_some());
    }

    #[test]
    fn from_package_name_lookup() {
        let m = PluginManifest::from_package_name("@specforge/product").unwrap();
        assert_eq!(m.module, Module::Product);
        assert!(PluginManifest::from_package_name("@specforge/unknown").is_none());
    }

    #[test]
    fn testable_kinds_are_subset_of_entity_kinds() {
        for manifest in [PluginManifest::product(), PluginManifest::governance()] {
            for kind in &manifest.testable_kinds {
                assert!(
                    manifest.entity_kinds.contains(kind),
                    "{kind} is testable but not in entity_kinds for {}",
                    manifest.package
                );
            }
        }
    }

    #[test]
    fn testable_kinds_match_is_testable() {
        for manifest in [PluginManifest::product(), PluginManifest::governance()] {
            for kind in &manifest.entity_kinds {
                if kind.is_testable() {
                    assert!(
                        manifest.testable_kinds.contains(kind),
                        "{kind} is_testable() but not in testable_kinds for {}",
                        manifest.package
                    );
                }
            }
        }
    }
}
