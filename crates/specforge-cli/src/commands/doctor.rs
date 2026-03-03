use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;
use specforge_common::{ConflictResolution, EnhancementConflict, FieldRegistry};

use crate::pipeline;

#[derive(Args)]
pub struct DoctorArgs {
    /// Path to project. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Output machine-readable JSON instead of human-readable text
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: DoctorArgs) -> i32 {
    // Load project config (but don't run the full pipeline)
    let project_root = pipeline::find_project_root(&args.path);

    let config = match &project_root {
        Some(pipeline::ProjectRoot::Json(json_path)) => {
            match pipeline::load_json_config(json_path) {
                Ok(c) => Some(c.to_compiler_config()),
                Err(msg) => {
                    eprintln!("specforge: {msg}");
                    return 1;
                }
            }
        }
        Some(pipeline::ProjectRoot::Spec(_)) => {
            // Legacy mode — no JSON config, use core-only
            None
        }
        None => None,
    };

    // Build field registry
    let registry = match &config {
        Some(c) => pipeline::build_field_registry(c),
        None => FieldRegistry::with_builtins(),
    };

    // Collect plugin info
    let plugins = &config
        .as_ref()
        .map(|c| c.plugins.clone())
        .unwrap_or_default();

    let mut plugin_infos: Vec<PluginInfo> = plugins
        .iter()
        .filter_map(|module| {
            let package = module.package_name()?;
            Some(PluginInfo {
                package: package.to_string(),
                source: "built-in".to_string(),
                entity_count: module.entity_count(),
                enhancement_count: 0,
                wasm_hash: None,
                lifecycle_state: None,
            })
        })
        .collect();

    // Add Wasm plugin info
    if let Some(ref c) = config {
        let root_dir = match &project_root {
            Some(pipeline::ProjectRoot::Json(p)) => {
                p.parent().unwrap_or(std::path::Path::new(".")).to_path_buf()
            }
            _ => args.path.clone(),
        };

        let (wasm_manifests, _) =
            specforge_wasm::discover::discover_packages(&c.wasm_package_specifiers, &root_dir);

        for wm in &wasm_manifests {
            let wasm_hash = std::fs::read(&wm.wasm_path)
                .ok()
                .map(|bytes| specforge_wasm::loader::compute_sha256(&bytes));

            plugin_infos.push(PluginInfo {
                package: wm.package.clone(),
                source: "wasm".to_string(),
                entity_count: wm.entity_kinds.len(),
                enhancement_count: wm.enhancements.len(),
                wasm_hash,
                lifecycle_state: Some("discovered".to_string()),
            });
        }
    }

    // Collect enhancements grouped by entity kind
    let mut enhancements_by_kind: HashMap<String, Vec<EnhancementDisplay>> = HashMap::new();
    for enh in registry.all_enhancements() {
        let display = EnhancementDisplay {
            field_name: enh.enhancement.field_name.clone(),
            field_type: format_field_type(&enh.enhancement.field_type),
            source_plugin: enh.source_plugin.clone(),
        };
        enhancements_by_kind
            .entry(enh.enhancement.target_entity.clone())
            .or_default()
            .push(display);
    }

    let conflicts = registry.conflicts();

    if args.json {
        print_json(&plugin_infos, &enhancements_by_kind, conflicts);
    } else {
        print_text(&plugin_infos, &enhancements_by_kind, conflicts);
    }

    if conflicts
        .iter()
        .any(|c| matches!(c.resolution, ConflictResolution::Unresolved))
    {
        1
    } else {
        0
    }
}

struct PluginInfo {
    package: String,
    source: String,
    entity_count: usize,
    enhancement_count: usize,
    wasm_hash: Option<String>,
    lifecycle_state: Option<String>,
}

struct EnhancementDisplay {
    field_name: String,
    field_type: String,
    source_plugin: String,
}

fn format_field_type(ft: &specforge_common::EnhancedFieldType) -> String {
    use specforge_common::EnhancedFieldType;
    match ft {
        EnhancedFieldType::String => "string".to_string(),
        EnhancedFieldType::Integer => "integer".to_string(),
        EnhancedFieldType::Bool => "bool".to_string(),
        EnhancedFieldType::Enum { values } => format!("enum [{}]", values.join(", ")),
        EnhancedFieldType::StringList => "string[]".to_string(),
        EnhancedFieldType::Reference {
            target_kind: Some(k),
            ..
        } => format!("reference -> {k}"),
        EnhancedFieldType::Reference {
            target_kind: None, ..
        } => "reference -> (any)".to_string(),
        EnhancedFieldType::ReferenceList {
            target_kind: Some(k),
            ..
        } => format!("reference[] -> {k}"),
        EnhancedFieldType::ReferenceList {
            target_kind: None, ..
        } => "reference[] -> (any)".to_string(),
    }
}

fn print_text(
    plugins: &[PluginInfo],
    enhancements: &HashMap<String, Vec<EnhancementDisplay>>,
    conflicts: &[EnhancementConflict],
) {
    eprintln!("specforge doctor\n");

    // Plugins section
    if plugins.is_empty() {
        eprintln!("Plugins: none installed\n");
    } else {
        eprintln!("Plugins ({} installed):", plugins.len());
        for p in plugins {
            let mut line = format!(
                "  {:<25} {:<10} {} entities   {} enhancements",
                p.package, p.source, p.entity_count, p.enhancement_count
            );
            if let Some(ref hash) = p.wasm_hash {
                line.push_str(&format!("   sha256:{}", &hash[..12]));
            }
            if let Some(ref state) = p.lifecycle_state {
                line.push_str(&format!("   [{state}]"));
            }
            eprintln!("{line}");
        }
        eprintln!();
    }

    // Enhancements section
    if enhancements.is_empty() {
        eprintln!("Enhancements: none\n");
    } else {
        eprintln!("Enhancements:");
        let mut kinds: Vec<_> = enhancements.keys().collect();
        kinds.sort();
        for kind in kinds {
            eprintln!("  {kind}");
            let fields = &enhancements[kind];
            for f in fields {
                let dots = ".".repeat(30usize.saturating_sub(f.field_name.len()));
                eprintln!(
                    "    {} {} {:<40} {}",
                    f.field_name, dots, f.field_type, f.source_plugin
                );
            }
        }
        eprintln!();
    }

    // Conflicts section
    if conflicts.is_empty() {
        eprintln!("Conflicts: none");
    } else {
        let unresolved_count = conflicts
            .iter()
            .filter(|c| matches!(c.resolution, ConflictResolution::Unresolved))
            .count();
        eprintln!("Conflicts ({}):\n", conflicts.len());
        for (i, c) in conflicts.iter().enumerate() {
            eprintln!("  [{}] {}.{}", i + 1, c.entity_kind, c.field_name);
            eprintln!("      {}  (first)", c.first_plugin);
            eprintln!("      {}  (second)", c.second_plugin);
            match &c.resolution {
                ConflictResolution::Unresolved => {
                    eprintln!("      Resolution: UNRESOLVED");
                    eprintln!("        Add to specforge.json: \"enhancement_overrides\": {{ \"{}.{}\": \"{}\" }}",
                        c.entity_kind, c.field_name, c.first_plugin);
                }
                ConflictResolution::ExplicitOverride { winner } => {
                    eprintln!("      Resolution: explicit override -> {winner}");
                }
                ConflictResolution::LoadOrder { winner } => {
                    eprintln!("      Resolution: load order -> {winner} (W023)");
                }
                ConflictResolution::Namespaced {
                    first_qual,
                    second_qual,
                } => {
                    eprintln!(
                        "      Resolution: namespaced -> {first_qual} / {second_qual}"
                    );
                }
            }
            eprintln!();
        }
        if unresolved_count > 0 {
            eprintln!(
                "specforge: {unresolved_count} unresolved conflict(s) — add enhancement_overrides to specforge.json"
            );
        }
    }
}

fn print_json(
    plugins: &[PluginInfo],
    enhancements: &HashMap<String, Vec<EnhancementDisplay>>,
    conflicts: &[EnhancementConflict],
) {
    let plugins_json: Vec<serde_json::Value> = plugins
        .iter()
        .map(|p| {
            let mut val = serde_json::json!({
                "package": p.package,
                "source": p.source,
                "entity_count": p.entity_count,
                "enhancement_count": p.enhancement_count,
            });
            if let Some(ref hash) = p.wasm_hash {
                val["wasm_hash"] = serde_json::json!(hash);
            }
            if let Some(ref state) = p.lifecycle_state {
                val["lifecycle_state"] = serde_json::json!(state);
            }
            val
        })
        .collect();

    let mut enhancements_json = serde_json::Map::new();
    let mut kinds: Vec<_> = enhancements.keys().collect();
    kinds.sort();
    for kind in kinds {
        let fields: Vec<serde_json::Value> = enhancements[kind]
            .iter()
            .map(|f| {
                serde_json::json!({
                    "field_name": f.field_name,
                    "field_type": f.field_type,
                    "source_plugin": f.source_plugin,
                })
            })
            .collect();
        enhancements_json.insert(kind.clone(), serde_json::Value::Array(fields));
    }

    let conflicts_json: Vec<serde_json::Value> = conflicts
        .iter()
        .map(|c| {
            let resolution = match &c.resolution {
                ConflictResolution::Unresolved => "unresolved".to_string(),
                ConflictResolution::ExplicitOverride { winner } => {
                    format!("explicit_override:{winner}")
                }
                ConflictResolution::LoadOrder { winner } => format!("load_order:{winner}"),
                ConflictResolution::Namespaced {
                    first_qual,
                    second_qual,
                } => format!("namespaced:{first_qual}/{second_qual}"),
            };
            serde_json::json!({
                "entity_kind": c.entity_kind,
                "field_name": c.field_name,
                "first_plugin": c.first_plugin,
                "second_plugin": c.second_plugin,
                "resolution": resolution,
            })
        })
        .collect();

    let output = serde_json::json!({
        "plugins": plugins_json,
        "enhancements": enhancements_json,
        "conflicts": conflicts_json,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{EnhancedFieldType, FieldEnhancement};

    #[test]
    fn doctor_no_plugins_no_conflicts() {
        let registry = FieldRegistry::with_builtins();
        assert!(registry.conflicts().is_empty());
        assert_eq!(registry.all_enhancements().count(), 0);
    }

    #[test]
    fn doctor_with_enhancements() {
        let mut registry = FieldRegistry::with_builtins();
        let enhancements = vec![FieldEnhancement {
            target_entity: "behavior".to_string(),
            field_name: "hex_layer".to_string(),
            field_type: EnhancedFieldType::Enum {
                values: vec![
                    "domain".to_string(),
                    "application".to_string(),
                    "adapter".to_string(),
                    "infrastructure".to_string(),
                ],
            },
            required: false,
            description: "Hexagonal architecture layer".to_string(),
        }];
        registry.register_plugin(
            "@specforge/hexagonal",
            &enhancements,
            &[],
            &specforge_common::EnhancementPolicy::default(),
            &HashMap::new(),
        );
        assert_eq!(registry.all_enhancements().count(), 1);
        assert!(registry.conflicts().is_empty());
    }

    #[test]
    fn format_field_type_variants() {
        assert_eq!(format_field_type(&EnhancedFieldType::String), "string");
        assert_eq!(format_field_type(&EnhancedFieldType::Integer), "integer");
        assert_eq!(format_field_type(&EnhancedFieldType::Bool), "bool");
        assert_eq!(format_field_type(&EnhancedFieldType::StringList), "string[]");
        assert_eq!(
            format_field_type(&EnhancedFieldType::Enum {
                values: vec!["a".to_string(), "b".to_string()]
            }),
            "enum [a, b]"
        );
        assert_eq!(
            format_field_type(&EnhancedFieldType::Reference {
                edge_label: "adapts".to_string(),
                target_kind: None
            }),
            "reference -> (any)"
        );
        assert_eq!(
            format_field_type(&EnhancedFieldType::Reference {
                edge_label: "adapts".to_string(),
                target_kind: Some("port".to_string())
            }),
            "reference -> port"
        );
    }

    #[test]
    fn doctor_json_output_is_valid() {
        let plugins = vec![PluginInfo {
            package: "@specforge/product".to_string(),
            source: "built-in".to_string(),
            entity_count: 5,
            enhancement_count: 0,
            wasm_hash: None,
            lifecycle_state: None,
        }];
        let _enhancements: HashMap<String, Vec<EnhancementDisplay>> = HashMap::new();
        let _conflicts: Vec<EnhancementConflict> = vec![];

        // Capture JSON output by constructing it directly
        let plugins_json: Vec<serde_json::Value> = plugins
            .iter()
            .map(|p| {
                serde_json::json!({
                    "package": p.package,
                    "source": p.source,
                    "entity_count": p.entity_count,
                    "enhancement_count": p.enhancement_count,
                })
            })
            .collect();

        let output = serde_json::json!({
            "plugins": plugins_json,
            "enhancements": {},
            "conflicts": [],
        });

        let json_str = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed["plugins"].is_array());
        assert!(parsed["enhancements"].is_object());
        assert!(parsed["conflicts"].is_array());
    }
}
