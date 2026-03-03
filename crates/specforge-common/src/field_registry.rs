use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::EdgeType;

/// A field enhancement declared by a plugin for an existing entity kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEnhancement {
    pub target_entity: String,
    pub field_name: String,
    pub field_type: EnhancedFieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: String,
}

/// The type of an enhanced field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EnhancedFieldType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "enum")]
    Enum { values: Vec<std::string::String> },
    #[serde(rename = "string_list")]
    StringList,
    #[serde(rename = "reference")]
    Reference {
        edge_label: std::string::String,
        target_kind: Option<std::string::String>,
    },
    #[serde(rename = "reference_list")]
    ReferenceList {
        edge_label: std::string::String,
        target_kind: Option<std::string::String>,
    },
}

impl EnhancedFieldType {
    /// Returns true if this field type creates graph edges.
    pub fn is_reference(&self) -> bool {
        matches!(self, Self::Reference { .. } | Self::ReferenceList { .. })
    }

    /// Returns the edge label for reference types, or None for data types.
    pub fn edge_label(&self) -> Option<&str> {
        match self {
            Self::Reference { edge_label, .. } | Self::ReferenceList { edge_label, .. } => {
                Some(edge_label)
            }
            _ => None,
        }
    }

    /// Human-readable kind name for display.
    pub fn display_kind(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Bool => "bool",
            Self::Enum { .. } => "enum",
            Self::StringList => "string_list",
            Self::Reference { .. } => "reference",
            Self::ReferenceList { .. } => "reference_list",
        }
    }
}

/// Dynamic edge type contributed by a plugin enhancement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicEdgeType {
    pub label: std::string::String,
    #[serde(default)]
    pub soft: bool,
}

/// Conflict between two plugins on the same (entity, field).
#[derive(Debug, Clone)]
pub struct EnhancementConflict {
    pub entity_kind: std::string::String,
    pub field_name: std::string::String,
    pub first_plugin: std::string::String,
    pub second_plugin: std::string::String,
    pub resolution: ConflictResolution,
}

/// How a conflict was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Not resolved — will produce E017 error.
    Unresolved,
    /// User explicitly chose a winner in specforge.json `enhancement_overrides`.
    ExplicitOverride { winner: std::string::String },
    /// First plugin in `plugins` array wins (policy = "priority").
    LoadOrder { winner: std::string::String },
    /// Fields auto-prefixed with plugin short name (policy = "namespace").
    Namespaced {
        first_qual: std::string::String,
        second_qual: std::string::String,
    },
}

/// Enhancement conflict resolution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnhancementPolicy {
    /// Conflicts are hard errors; user must add overrides.
    #[default]
    Error,
    /// First plugin in `plugins` array wins; emits W023 warning.
    Priority,
    /// Conflicting fields auto-prefixed with plugin short name.
    Namespace,
}

/// A registered enhancement with its source plugin identity.
#[derive(Debug, Clone)]
pub struct RegisteredEnhancement {
    pub enhancement: FieldEnhancement,
    pub source_plugin: std::string::String,
}

/// Lookup result: either a built-in edge type or an enhanced field.
pub enum FieldLookup<'a> {
    Builtin(EdgeType),
    Enhanced(&'a RegisteredEnhancement),
}

/// Central registry: built-in field→EdgeType + plugin-enhanced fields.
///
/// The registry is built at startup and threaded through the pipeline
/// (resolve → build_graph → validate).
pub struct FieldRegistry {
    /// Global field_name → EdgeType for built-in reference fields.
    builtin: HashMap<std::string::String, EdgeType>,
    /// (entity_kind, field_name) → registered enhancement.
    enhanced: HashMap<(std::string::String, std::string::String), RegisteredEnhancement>,
    /// edge_label → dynamic edge type from plugins.
    dynamic_edges: HashMap<std::string::String, DynamicEdgeType>,
    /// Detected conflicts during registration.
    conflicts: Vec<EnhancementConflict>,
}

impl FieldRegistry {
    /// Create a registry pre-loaded with all built-in field→EdgeType mappings.
    pub fn with_builtins() -> Self {
        let mut builtin = HashMap::new();

        // All 21 field_name → EdgeType mappings from EdgeType::from_field_name
        let mappings: &[(&str, EdgeType)] = &[
            ("invariants", EdgeType::References),
            ("behaviors", EdgeType::Implements),
            ("produces", EdgeType::Produces),
            ("events", EdgeType::Produces),
            ("consumers", EdgeType::Consumes),
            ("types", EdgeType::UsesType),
            ("ports", EdgeType::UsesPort),
            ("enforced_by", EdgeType::Enforces),
            ("refs", EdgeType::LinksTo),
            ("links", EdgeType::LinksTo),
            ("features", EdgeType::TracesTo),
            ("capabilities", EdgeType::Bundles),
            ("libraries", EdgeType::BuiltFrom),
            ("depends_on", EdgeType::DependsOn),
            ("provides", EdgeType::Provides),
            ("ports_defined", EdgeType::DefinesPort),
            ("adrs", EdgeType::ShapedBy),
            ("protects", EdgeType::Protects),
            ("constrains", EdgeType::Constrains),
            ("mitigates", EdgeType::Mitigates),
            ("invariant", EdgeType::Mitigates),
        ];

        for (field, edge_type) in mappings {
            builtin.insert((*field).to_string(), *edge_type);
        }

        Self {
            builtin,
            enhanced: HashMap::new(),
            dynamic_edges: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Register a plugin's enhancements. Returns any conflicts detected.
    ///
    /// `overrides` maps `"entity.field"` → `"@plugin/name"` for explicit resolution.
    pub fn register_plugin(
        &mut self,
        package: &str,
        enhancements: &[FieldEnhancement],
        dynamic_edge_types: &[DynamicEdgeType],
        policy: &EnhancementPolicy,
        overrides: &HashMap<std::string::String, std::string::String>,
    ) -> Vec<EnhancementConflict> {
        let mut new_conflicts = Vec::new();

        // Register dynamic edge types
        for det in dynamic_edge_types {
            self.dynamic_edges.insert(
                det.label.clone(),
                det.clone(),
            );
        }

        for enh in enhancements {
            let key = (enh.target_entity.clone(), enh.field_name.clone());
            let override_key = format!("{}.{}", enh.target_entity, enh.field_name);

            // Check for conflict with built-in fields
            if self.builtin.contains_key(&enh.field_name) {
                let conflict = EnhancementConflict {
                    entity_kind: enh.target_entity.clone(),
                    field_name: enh.field_name.clone(),
                    first_plugin: "(built-in)".to_string(),
                    second_plugin: package.to_string(),
                    resolution: ConflictResolution::Unresolved, // Always hard error for built-in
                };
                new_conflicts.push(conflict.clone());
                self.conflicts.push(conflict);
                continue;
            }

            // Check for conflict with existing enhancement
            if let Some(existing) = self.enhanced.get(&key) {
                let resolution = if let Some(winner) = overrides.get(&override_key) {
                    ConflictResolution::ExplicitOverride {
                        winner: winner.clone(),
                    }
                } else {
                    match policy {
                        EnhancementPolicy::Error => ConflictResolution::Unresolved,
                        EnhancementPolicy::Priority => ConflictResolution::LoadOrder {
                            winner: existing.source_plugin.clone(),
                        },
                        EnhancementPolicy::Namespace => {
                            let first_short = short_plugin_name(&existing.source_plugin);
                            let second_short = short_plugin_name(package);
                            ConflictResolution::Namespaced {
                                first_qual: format!("{first_short}__{}", enh.field_name),
                                second_qual: format!("{second_short}__{}", enh.field_name),
                            }
                        }
                    }
                };

                let conflict = EnhancementConflict {
                    entity_kind: enh.target_entity.clone(),
                    field_name: enh.field_name.clone(),
                    first_plugin: existing.source_plugin.clone(),
                    second_plugin: package.to_string(),
                    resolution: resolution.clone(),
                };
                new_conflicts.push(conflict.clone());
                self.conflicts.push(conflict);

                // If the conflict is resolved in favor of the new plugin, replace
                match &resolution {
                    ConflictResolution::ExplicitOverride { winner }
                        if winner == package =>
                    {
                        self.enhanced.insert(
                            key,
                            RegisteredEnhancement {
                                enhancement: enh.clone(),
                                source_plugin: package.to_string(),
                            },
                        );
                    }
                    ConflictResolution::Namespaced {
                        first_qual,
                        second_qual,
                    } => {
                        // Re-register existing with qualified name
                        let existing_enh = existing.clone();
                        let first_key =
                            (enh.target_entity.clone(), first_qual.clone());
                        self.enhanced.insert(first_key, existing_enh);
                        // Register new with qualified name
                        let second_key =
                            (enh.target_entity.clone(), second_qual.clone());
                        self.enhanced.insert(
                            second_key,
                            RegisteredEnhancement {
                                enhancement: enh.clone(),
                                source_plugin: package.to_string(),
                            },
                        );
                        // Remove original unqualified key
                        self.enhanced.remove(&key);
                    }
                    _ => {
                        // LoadOrder: existing stays (first plugin wins)
                        // Unresolved or ExplicitOverride for existing: existing stays
                    }
                }
            } else {
                // No conflict — register
                self.enhanced.insert(
                    key,
                    RegisteredEnhancement {
                        enhancement: enh.clone(),
                        source_plugin: package.to_string(),
                    },
                );
            }
        }

        new_conflicts
    }

    /// Look up a field: check built-in first, then enhanced.
    pub fn lookup(&self, entity_kind: &str, field_name: &str) -> Option<FieldLookup<'_>> {
        // Built-in takes precedence (global, not scoped to entity kind)
        if let Some(&edge_type) = self.builtin.get(field_name) {
            return Some(FieldLookup::Builtin(edge_type));
        }

        // Enhanced fields are scoped to (entity_kind, field_name)
        let key = (entity_kind.to_string(), field_name.to_string());
        if let Some(enh) = self.enhanced.get(&key) {
            return Some(FieldLookup::Enhanced(enh));
        }

        None
    }

    /// Returns true if the field is a reference field (built-in edge or enhanced ref/reflist).
    pub fn is_reference_field(&self, entity_kind: &str, field_name: &str) -> bool {
        match self.lookup(entity_kind, field_name) {
            Some(FieldLookup::Builtin(_)) => true,
            Some(FieldLookup::Enhanced(enh)) => enh.enhancement.field_type.is_reference(),
            None => false,
        }
    }

    /// Get all enhanced fields for a given entity kind (for LSP completion).
    pub fn enhanced_fields_for_kind(&self, entity_kind: &str) -> Vec<&RegisteredEnhancement> {
        self.enhanced
            .iter()
            .filter(|((ek, _), _)| ek == entity_kind)
            .map(|(_, v)| v)
            .collect()
    }

    /// Iterate over all registered enhancements.
    pub fn all_enhancements(&self) -> impl Iterator<Item = &RegisteredEnhancement> {
        self.enhanced.values()
    }

    /// Get detected conflicts.
    pub fn conflicts(&self) -> &[EnhancementConflict] {
        &self.conflicts
    }

    /// Number of built-in field mappings.
    pub fn builtin_count(&self) -> usize {
        self.builtin.len()
    }

    /// Number of enhanced fields registered.
    pub fn enhancement_count(&self) -> usize {
        self.enhanced.len()
    }

    /// Get dynamic edge types.
    pub fn dynamic_edges(&self) -> &HashMap<std::string::String, DynamicEdgeType> {
        &self.dynamic_edges
    }
}

/// Extract short plugin name from package name.
/// `"@specforge/hexagonal"` → `"hexagonal"`
fn short_plugin_name(package: &str) -> &str {
    package.rsplit('/').next().unwrap_or(package)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data_enhancement(target: &str, field: &str) -> FieldEnhancement {
        FieldEnhancement {
            target_entity: target.to_string(),
            field_name: field.to_string(),
            field_type: EnhancedFieldType::Enum {
                values: vec!["a".to_string(), "b".to_string()],
            },
            required: false,
            description: String::new(),
        }
    }

    fn make_ref_enhancement(target: &str, field: &str, edge_label: &str) -> FieldEnhancement {
        FieldEnhancement {
            target_entity: target.to_string(),
            field_name: field.to_string(),
            field_type: EnhancedFieldType::Reference {
                edge_label: edge_label.to_string(),
                target_kind: None,
            },
            required: false,
            description: String::new(),
        }
    }

    fn register_test_plugin(
        registry: &mut FieldRegistry,
        package: &str,
        enhancements: Vec<FieldEnhancement>,
    ) -> Vec<EnhancementConflict> {
        registry.register_plugin(package, &enhancements, &[], &EnhancementPolicy::Error, &HashMap::new())
    }

    #[test]
    fn with_builtins_contains_all_mappings() {
        let registry = FieldRegistry::with_builtins();
        assert_eq!(registry.builtin_count(), 21);

        // Spot-check a few
        assert!(matches!(
            registry.lookup("behavior", "invariants"),
            Some(FieldLookup::Builtin(EdgeType::References))
        ));
        assert!(matches!(
            registry.lookup("invariant", "enforced_by"),
            Some(FieldLookup::Builtin(EdgeType::Enforces))
        ));
        assert!(matches!(
            registry.lookup("behavior", "adrs"),
            Some(FieldLookup::Builtin(EdgeType::ShapedBy))
        ));
    }

    #[test]
    fn register_plugin_adds_scoped_fields() {
        let mut registry = FieldRegistry::with_builtins();
        let conflicts = register_test_plugin(
            &mut registry,
            "@specforge/hexagonal",
            vec![make_data_enhancement("behavior", "hex_layer")],
        );
        assert!(conflicts.is_empty());
        assert_eq!(registry.enhancement_count(), 1);

        // Lookup succeeds for the right entity kind
        assert!(matches!(
            registry.lookup("behavior", "hex_layer"),
            Some(FieldLookup::Enhanced(_))
        ));
        // Different entity kind returns None
        assert!(registry.lookup("invariant", "hex_layer").is_none());
    }

    #[test]
    fn conflict_with_builtin_always_error() {
        let mut registry = FieldRegistry::with_builtins();
        let conflicts = registry.register_plugin(
            "@specforge/bad",
            &[make_data_enhancement("behavior", "invariants")],
            &[],
            &EnhancementPolicy::Priority,
            &HashMap::new(),
        );
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].first_plugin, "(built-in)");
        assert_eq!(conflicts[0].resolution, ConflictResolution::Unresolved);
    }

    #[test]
    fn conflict_between_plugins_error_policy() {
        let mut registry = FieldRegistry::with_builtins();
        register_test_plugin(
            &mut registry,
            "@specforge/hexagonal",
            vec![make_data_enhancement("behavior", "layer")],
        );
        let conflicts = register_test_plugin(
            &mut registry,
            "@specforge/clean-arch",
            vec![make_data_enhancement("behavior", "layer")],
        );

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].first_plugin, "@specforge/hexagonal");
        assert_eq!(conflicts[0].second_plugin, "@specforge/clean-arch");
        assert_eq!(conflicts[0].resolution, ConflictResolution::Unresolved);
    }

    #[test]
    fn conflict_priority_policy_first_wins() {
        let mut registry = FieldRegistry::with_builtins();
        let enh1 = vec![make_data_enhancement("behavior", "layer")];
        let enh2 = vec![make_data_enhancement("behavior", "layer")];
        registry.register_plugin("@specforge/hexagonal", &enh1, &[], &EnhancementPolicy::Priority, &HashMap::new());
        let conflicts = registry.register_plugin("@specforge/clean-arch", &enh2, &[], &EnhancementPolicy::Priority, &HashMap::new());

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0].resolution {
            ConflictResolution::LoadOrder { winner } => {
                assert_eq!(winner, "@specforge/hexagonal");
            }
            _ => panic!("expected LoadOrder resolution"),
        }

        // First plugin's value still registered
        if let Some(FieldLookup::Enhanced(enh)) = registry.lookup("behavior", "layer") {
            assert_eq!(enh.source_plugin, "@specforge/hexagonal");
        } else {
            panic!("expected enhanced field");
        }
    }

    #[test]
    fn conflict_namespace_policy_prefixes() {
        let mut registry = FieldRegistry::with_builtins();
        let enh1 = vec![make_data_enhancement("behavior", "layer")];
        let enh2 = vec![make_data_enhancement("behavior", "layer")];
        registry.register_plugin("@specforge/hexagonal", &enh1, &[], &EnhancementPolicy::Namespace, &HashMap::new());
        let conflicts = registry.register_plugin("@specforge/clean-arch", &enh2, &[], &EnhancementPolicy::Namespace, &HashMap::new());

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0].resolution {
            ConflictResolution::Namespaced {
                first_qual,
                second_qual,
            } => {
                assert_eq!(first_qual, "hexagonal__layer");
                assert_eq!(second_qual, "clean-arch__layer");
            }
            _ => panic!("expected Namespaced resolution"),
        }

        // Original unqualified key removed, qualified keys exist
        assert!(registry.lookup("behavior", "layer").is_none());
        assert!(registry.lookup("behavior", "hexagonal__layer").is_some());
        assert!(registry.lookup("behavior", "clean-arch__layer").is_some());
    }

    #[test]
    fn explicit_override_resolves_conflict() {
        let mut registry = FieldRegistry::with_builtins();
        register_test_plugin(
            &mut registry,
            "@specforge/hexagonal",
            vec![make_data_enhancement("behavior", "layer")],
        );

        let mut overrides = HashMap::new();
        overrides.insert(
            "behavior.layer".to_string(),
            "@specforge/clean-arch".to_string(),
        );

        let enh2 = vec![make_data_enhancement("behavior", "layer")];
        let conflicts = registry.register_plugin("@specforge/clean-arch", &enh2, &[], &EnhancementPolicy::Error, &overrides);

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0].resolution {
            ConflictResolution::ExplicitOverride { winner } => {
                assert_eq!(winner, "@specforge/clean-arch");
            }
            _ => panic!("expected ExplicitOverride resolution"),
        }

        // Second plugin won
        if let Some(FieldLookup::Enhanced(enh)) = registry.lookup("behavior", "layer") {
            assert_eq!(enh.source_plugin, "@specforge/clean-arch");
        } else {
            panic!("expected enhanced field from clean-arch");
        }
    }

    #[test]
    fn is_reference_field_enhanced() {
        let mut registry = FieldRegistry::with_builtins();
        let enhancements = vec![
            make_ref_enhancement("port", "adapter_ref", "adapts"),
            make_data_enhancement("port", "hex_side"),
        ];
        registry.register_plugin("@specforge/hexagonal", &enhancements, &[], &EnhancementPolicy::Error, &HashMap::new());

        assert!(registry.is_reference_field("port", "adapter_ref"));
        assert!(!registry.is_reference_field("port", "hex_side"));
        // Built-in
        assert!(registry.is_reference_field("behavior", "invariants"));
        // Unknown
        assert!(!registry.is_reference_field("behavior", "unknown_field"));
    }

    #[test]
    fn no_false_positive_same_field_different_entities() {
        let mut registry = FieldRegistry::with_builtins();
        let conflicts = register_test_plugin(
            &mut registry,
            "@specforge/hexagonal",
            vec![
                make_data_enhancement("behavior", "layer"),
                make_data_enhancement("port", "layer"),
            ],
        );
        assert!(conflicts.is_empty());
        assert_eq!(registry.enhancement_count(), 2);
    }

    #[test]
    fn enhanced_fields_for_kind() {
        let mut registry = FieldRegistry::with_builtins();
        register_test_plugin(
            &mut registry,
            "@specforge/hexagonal",
            vec![
                make_data_enhancement("behavior", "hex_layer"),
                make_data_enhancement("behavior", "bounded_context"),
                make_data_enhancement("port", "hex_side"),
            ],
        );

        let behavior_enhancements = registry.enhanced_fields_for_kind("behavior");
        assert_eq!(behavior_enhancements.len(), 2);

        let port_enhancements = registry.enhanced_fields_for_kind("port");
        assert_eq!(port_enhancements.len(), 1);

        let invariant_enhancements = registry.enhanced_fields_for_kind("invariant");
        assert!(invariant_enhancements.is_empty());
    }

    #[test]
    fn short_plugin_name_extraction() {
        assert_eq!(short_plugin_name("@specforge/hexagonal"), "hexagonal");
        assert_eq!(short_plugin_name("@specforge/clean-arch"), "clean-arch");
        assert_eq!(short_plugin_name("simple"), "simple");
    }

    #[test]
    fn explicit_override_favors_existing_plugin() {
        let mut registry = FieldRegistry::with_builtins();
        register_test_plugin(
            &mut registry,
            "@specforge/hexagonal",
            vec![make_data_enhancement("behavior", "layer")],
        );

        // Override specifies first plugin as winner
        let mut overrides = HashMap::new();
        overrides.insert(
            "behavior.layer".to_string(),
            "@specforge/hexagonal".to_string(),
        );

        let enh2 = vec![make_data_enhancement("behavior", "layer")];
        let conflicts = registry.register_plugin("@specforge/clean-arch", &enh2, &[], &EnhancementPolicy::Error, &overrides);

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0].resolution {
            ConflictResolution::ExplicitOverride { winner } => {
                assert_eq!(winner, "@specforge/hexagonal");
            }
            _ => panic!("expected ExplicitOverride resolution"),
        }

        // First plugin's enhancement remains registered (override favored existing)
        if let Some(FieldLookup::Enhanced(enh)) = registry.lookup("behavior", "layer") {
            assert_eq!(enh.source_plugin, "@specforge/hexagonal");
        } else {
            panic!("expected enhanced field from hexagonal");
        }
    }

    #[test]
    fn dynamic_edge_types_registered() {
        let mut registry = FieldRegistry::with_builtins();
        let det = vec![DynamicEdgeType {
            label: "adapts".to_string(),
            soft: false,
        }];
        registry.register_plugin("@test/hex", &[], &det, &EnhancementPolicy::default(), &HashMap::new());
        assert!(registry.dynamic_edges().contains_key("adapts"));
        assert!(!registry.dynamic_edges()["adapts"].soft);
    }

    #[test]
    fn edge_label_accessor() {
        let ref_type = EnhancedFieldType::Reference {
            edge_label: "adapts".to_string(),
            target_kind: Some("port".to_string()),
        };
        assert_eq!(ref_type.edge_label(), Some("adapts"));
        assert!(ref_type.is_reference());

        let ref_list = EnhancedFieldType::ReferenceList {
            edge_label: "implements".to_string(),
            target_kind: None,
        };
        assert_eq!(ref_list.edge_label(), Some("implements"));
        assert!(ref_list.is_reference());

        // Non-reference types return None
        assert_eq!(EnhancedFieldType::String.edge_label(), None);
        assert!(!EnhancedFieldType::String.is_reference());
        assert_eq!(
            EnhancedFieldType::Enum {
                values: vec![]
            }
            .edge_label(),
            None
        );
    }
}
