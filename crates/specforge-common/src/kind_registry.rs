use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::EntityKind;

/// Conflict between two sources registering the same entity kind name.
#[derive(Debug, Clone)]
pub struct KindConflict {
    pub kind_name: String,
    pub first_source: String,
    pub second_source: String,
    pub resolution: KindConflictResolution,
}

/// How a kind conflict was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KindConflictResolution {
    /// Not resolved — will produce E022 or E023 error.
    Unresolved,
    /// User explicitly chose a winner in specforge.json `entity_kinds`.
    ExplicitOverride { winner: String },
    /// First plugin in `plugins` array wins (policy = "priority").
    LoadOrder { winner: String },
    /// Kinds auto-prefixed with plugin short name (policy = "namespace").
    Namespaced {
        first_qual: String,
        second_qual: String,
    },
}

/// Entity kind conflict resolution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKindPolicy {
    /// Conflicts are hard errors; user must add overrides.
    #[default]
    Error,
    /// First plugin in `plugins` array wins; emits W027 warning.
    Priority,
    /// Conflicting kinds auto-prefixed with plugin short name.
    Namespace,
}

/// A registered entity kind with its source plugin identity.
#[derive(Debug, Clone)]
pub struct RegisteredKind {
    pub name: String,
    pub source_plugin: String,
    pub testable: bool,
}

/// Central registry for entity kind names: built-in + plugin-registered + define-block.
///
/// Mirrors `FieldRegistry` but for entity kind name uniqueness.
pub struct KindRegistry {
    /// Built-in entity kind keywords (the 16 hardcoded ones).
    builtin: HashMap<String, EntityKind>,
    /// Plugin-registered entity kinds: kind_name → RegisteredKind.
    registered: HashMap<String, RegisteredKind>,
    /// Define-block entity kinds (from `define` blocks in .spec files).
    define_kinds: HashMap<String, String>,
    /// Detected conflicts during registration.
    conflicts: Vec<KindConflict>,
}

impl KindRegistry {
    /// Create a registry pre-loaded with all 16 built-in entity kind keywords.
    pub fn with_builtins() -> Self {
        let mut builtin = HashMap::new();
        for kind in EntityKind::ALL {
            builtin.insert(kind.keyword().to_string(), kind);
        }

        Self {
            builtin,
            registered: HashMap::new(),
            define_kinds: HashMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Register entity kinds from `define` blocks in the spec files.
    pub fn add_define_kinds(&mut self, kinds: &[String]) {
        for name in kinds {
            self.define_kinds.insert(name.clone(), "(define)".to_string());
        }
    }

    /// Register entity kinds from a plugin. Returns any conflicts detected.
    ///
    /// `overrides` maps kind_name → plugin package name for explicit resolution.
    pub fn register_plugin(
        &mut self,
        plugin_package: &str,
        kinds: &[(String, bool)],
        policy: &EntityKindPolicy,
        overrides: &HashMap<String, String>,
    ) -> Vec<KindConflict> {
        let mut new_conflicts = Vec::new();

        for (kind_name, testable) in kinds {
            // Check vs builtin → always Unresolved
            if self.builtin.contains_key(kind_name) {
                let conflict = KindConflict {
                    kind_name: kind_name.clone(),
                    first_source: "(built-in)".to_string(),
                    second_source: plugin_package.to_string(),
                    resolution: KindConflictResolution::Unresolved,
                };
                new_conflicts.push(conflict.clone());
                self.conflicts.push(conflict);
                continue;
            }

            // Check vs define → always Unresolved
            if self.define_kinds.contains_key(kind_name) {
                let conflict = KindConflict {
                    kind_name: kind_name.clone(),
                    first_source: "(define)".to_string(),
                    second_source: plugin_package.to_string(),
                    resolution: KindConflictResolution::Unresolved,
                };
                new_conflicts.push(conflict.clone());
                self.conflicts.push(conflict);
                continue;
            }

            // Check vs other plugin → apply policy
            if let Some(existing) = self.registered.get(kind_name) {
                let resolution = if let Some(winner) = overrides.get(kind_name) {
                    KindConflictResolution::ExplicitOverride {
                        winner: winner.clone(),
                    }
                } else {
                    match policy {
                        EntityKindPolicy::Error => KindConflictResolution::Unresolved,
                        EntityKindPolicy::Priority => KindConflictResolution::LoadOrder {
                            winner: existing.source_plugin.clone(),
                        },
                        EntityKindPolicy::Namespace => {
                            let first_short = short_plugin_name(&existing.source_plugin);
                            let second_short = short_plugin_name(plugin_package);
                            KindConflictResolution::Namespaced {
                                first_qual: format!("{first_short}__{kind_name}"),
                                second_qual: format!("{second_short}__{kind_name}"),
                            }
                        }
                    }
                };

                let conflict = KindConflict {
                    kind_name: kind_name.clone(),
                    first_source: existing.source_plugin.clone(),
                    second_source: plugin_package.to_string(),
                    resolution: resolution.clone(),
                };
                new_conflicts.push(conflict.clone());
                self.conflicts.push(conflict);

                // If the conflict is resolved in favor of the new plugin, replace
                match &resolution {
                    KindConflictResolution::ExplicitOverride { winner }
                        if winner == plugin_package =>
                    {
                        self.registered.insert(
                            kind_name.clone(),
                            RegisteredKind {
                                name: kind_name.clone(),
                                source_plugin: plugin_package.to_string(),
                                testable: *testable,
                            },
                        );
                    }
                    KindConflictResolution::Namespaced {
                        first_qual,
                        second_qual,
                    } => {
                        // Re-register existing with qualified name
                        let existing_kind = existing.clone();
                        self.registered.insert(first_qual.clone(), existing_kind);
                        // Register new with qualified name
                        self.registered.insert(
                            second_qual.clone(),
                            RegisteredKind {
                                name: kind_name.clone(),
                                source_plugin: plugin_package.to_string(),
                                testable: *testable,
                            },
                        );
                        // Remove original unqualified key
                        self.registered.remove(kind_name);
                    }
                    _ => {
                        // LoadOrder: existing stays (first plugin wins)
                        // Unresolved or ExplicitOverride for existing: existing stays
                    }
                }
            } else {
                // No conflict — register
                self.registered.insert(
                    kind_name.clone(),
                    RegisteredKind {
                        name: kind_name.clone(),
                        source_plugin: plugin_package.to_string(),
                        testable: *testable,
                    },
                );
            }
        }

        new_conflicts
    }

    /// Look up a kind by name: check built-in first, then registered.
    pub fn lookup(&self, name: &str) -> Option<KindLookup<'_>> {
        if let Some(kind) = self.builtin.get(name) {
            return Some(KindLookup::Builtin(kind.clone()));
        }
        if let Some(reg) = self.registered.get(name) {
            return Some(KindLookup::Plugin(reg));
        }
        if self.define_kinds.contains_key(name) {
            return Some(KindLookup::Define);
        }
        None
    }

    /// Resolve a qualified kind name `@plugin/kind`.
    pub fn resolve_qualified(&self, plugin: &str, kind: &str) -> Option<&RegisteredKind> {
        self.registered
            .values()
            .find(|rk| rk.name == kind && rk.source_plugin == plugin)
    }

    /// Get detected conflicts.
    pub fn conflicts(&self) -> &[KindConflict] {
        &self.conflicts
    }

    /// Number of built-in kind mappings.
    pub fn builtin_count(&self) -> usize {
        self.builtin.len()
    }

    /// Number of plugin-registered kinds.
    pub fn registered_count(&self) -> usize {
        self.registered.len()
    }

    /// All registered plugin kinds.
    pub fn all_registered_kinds(&self) -> impl Iterator<Item = &RegisteredKind> {
        self.registered.values()
    }
}

/// Lookup result for entity kind.
pub enum KindLookup<'a> {
    Builtin(EntityKind),
    Plugin(&'a RegisteredKind),
    Define,
}

/// Extract short plugin name from package name.
/// `"@specforge/hexagonal"` → `"hexagonal"`
fn short_plugin_name(package: &str) -> &str {
    package.rsplit('/').next().unwrap_or(package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_builtins_contains_all_16() {
        let registry = KindRegistry::with_builtins();
        assert_eq!(registry.builtin_count(), 16);

        // Spot-check
        assert!(matches!(
            registry.lookup("behavior"),
            Some(KindLookup::Builtin(EntityKind::Behavior))
        ));
        assert!(matches!(
            registry.lookup("failure_mode"),
            Some(KindLookup::Builtin(EntityKind::FailureMode))
        ));
    }

    #[test]
    fn register_plugin_adds_kinds() {
        let mut registry = KindRegistry::with_builtins();
        let kinds = vec![("epic".to_string(), true), ("saga".to_string(), false)];

        let conflicts = registry.register_plugin(
            "@myorg/tracker",
            &kinds,
            &EntityKindPolicy::Error,
            &HashMap::new(),
        );
        assert!(conflicts.is_empty());
        assert_eq!(registry.registered_count(), 2);

        assert!(matches!(
            registry.lookup("epic"),
            Some(KindLookup::Plugin(_))
        ));
    }

    #[test]
    fn collision_with_builtin_always_unresolved() {
        let mut registry = KindRegistry::with_builtins();
        let kinds = vec![("behavior".to_string(), false)];

        let conflicts = registry.register_plugin(
            "@bad/plugin",
            &kinds,
            &EntityKindPolicy::Priority,
            &HashMap::new(),
        );
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].first_source, "(built-in)");
        assert_eq!(
            conflicts[0].resolution,
            KindConflictResolution::Unresolved
        );
    }

    #[test]
    fn collision_with_define_always_unresolved() {
        let mut registry = KindRegistry::with_builtins();
        registry.add_define_kinds(&["microservice".to_string()]);

        let kinds = vec![("microservice".to_string(), false)];
        let conflicts = registry.register_plugin(
            "@myorg/plugin",
            &kinds,
            &EntityKindPolicy::Priority,
            &HashMap::new(),
        );
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].first_source, "(define)");
        assert_eq!(
            conflicts[0].resolution,
            KindConflictResolution::Unresolved
        );
    }

    #[test]
    fn conflict_between_plugins_error_policy() {
        let mut registry = KindRegistry::with_builtins();
        let k1 = vec![("epic".to_string(), true)];
        let k2 = vec![("epic".to_string(), false)];

        registry.register_plugin("@org/a", &k1, &EntityKindPolicy::Error, &HashMap::new());
        let conflicts =
            registry.register_plugin("@org/b", &k2, &EntityKindPolicy::Error, &HashMap::new());

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].first_source, "@org/a");
        assert_eq!(conflicts[0].second_source, "@org/b");
        assert_eq!(
            conflicts[0].resolution,
            KindConflictResolution::Unresolved
        );
    }

    #[test]
    fn conflict_priority_policy_first_wins() {
        let mut registry = KindRegistry::with_builtins();
        let k1 = vec![("epic".to_string(), true)];
        let k2 = vec![("epic".to_string(), false)];

        registry.register_plugin(
            "@org/a",
            &k1,
            &EntityKindPolicy::Priority,
            &HashMap::new(),
        );
        let conflicts = registry.register_plugin(
            "@org/b",
            &k2,
            &EntityKindPolicy::Priority,
            &HashMap::new(),
        );

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0].resolution {
            KindConflictResolution::LoadOrder { winner } => {
                assert_eq!(winner, "@org/a");
            }
            _ => panic!("expected LoadOrder resolution"),
        }

        // First plugin's kind still registered
        if let Some(KindLookup::Plugin(rk)) = registry.lookup("epic") {
            assert_eq!(rk.source_plugin, "@org/a");
        } else {
            panic!("expected plugin kind");
        }
    }

    #[test]
    fn conflict_namespace_policy_prefixes() {
        let mut registry = KindRegistry::with_builtins();
        let k1 = vec![("epic".to_string(), true)];
        let k2 = vec![("epic".to_string(), false)];

        registry.register_plugin(
            "@org/tracker",
            &k1,
            &EntityKindPolicy::Namespace,
            &HashMap::new(),
        );
        let conflicts = registry.register_plugin(
            "@org/planner",
            &k2,
            &EntityKindPolicy::Namespace,
            &HashMap::new(),
        );

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0].resolution {
            KindConflictResolution::Namespaced {
                first_qual,
                second_qual,
            } => {
                assert_eq!(first_qual, "tracker__epic");
                assert_eq!(second_qual, "planner__epic");
            }
            _ => panic!("expected Namespaced resolution"),
        }

        // Original unqualified key removed, qualified keys exist
        assert!(registry.lookup("epic").is_none());
        assert!(registry.lookup("tracker__epic").is_some());
        assert!(registry.lookup("planner__epic").is_some());
    }

    #[test]
    fn explicit_override_resolves_conflict() {
        let mut registry = KindRegistry::with_builtins();
        let k1 = vec![("epic".to_string(), true)];
        let k2 = vec![("epic".to_string(), false)];

        let mut overrides = HashMap::new();
        overrides.insert("epic".to_string(), "@org/b".to_string());

        registry.register_plugin("@org/a", &k1, &EntityKindPolicy::Error, &HashMap::new());
        let conflicts =
            registry.register_plugin("@org/b", &k2, &EntityKindPolicy::Error, &overrides);

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0].resolution {
            KindConflictResolution::ExplicitOverride { winner } => {
                assert_eq!(winner, "@org/b");
            }
            _ => panic!("expected ExplicitOverride resolution"),
        }

        // Second plugin won
        if let Some(KindLookup::Plugin(rk)) = registry.lookup("epic") {
            assert_eq!(rk.source_plugin, "@org/b");
        } else {
            panic!("expected plugin kind from @org/b");
        }
    }

    #[test]
    fn no_false_positive_different_names() {
        let mut registry = KindRegistry::with_builtins();
        let k1 = vec![("epic".to_string(), true)];
        let k2 = vec![("saga".to_string(), false)];

        registry.register_plugin("@org/a", &k1, &EntityKindPolicy::Error, &HashMap::new());
        let conflicts =
            registry.register_plugin("@org/b", &k2, &EntityKindPolicy::Error, &HashMap::new());

        assert!(conflicts.is_empty());
        assert_eq!(registry.registered_count(), 2);
    }

    #[test]
    fn resolve_qualified() {
        let mut registry = KindRegistry::with_builtins();
        let k1 = vec![("epic".to_string(), true)];
        registry.register_plugin("@org/tracker", &k1, &EntityKindPolicy::Error, &HashMap::new());

        let result = registry.resolve_qualified("@org/tracker", "epic");
        assert!(result.is_some());
        assert_eq!(result.unwrap().source_plugin, "@org/tracker");

        // Wrong plugin
        assert!(registry.resolve_qualified("@org/other", "epic").is_none());
        // Wrong kind
        assert!(registry.resolve_qualified("@org/tracker", "saga").is_none());
    }

    #[test]
    fn define_kinds_registered() {
        let mut registry = KindRegistry::with_builtins();
        registry.add_define_kinds(&["microservice".to_string(), "api_gateway".to_string()]);

        assert!(matches!(
            registry.lookup("microservice"),
            Some(KindLookup::Define)
        ));
        assert!(matches!(
            registry.lookup("api_gateway"),
            Some(KindLookup::Define)
        ));
        assert!(registry.lookup("unknown").is_none());
    }

    #[test]
    fn short_plugin_name_extraction() {
        assert_eq!(short_plugin_name("@specforge/hexagonal"), "hexagonal");
        assert_eq!(short_plugin_name("@org/tracker"), "tracker");
        assert_eq!(short_plugin_name("simple"), "simple");
    }
}
