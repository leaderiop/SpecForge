use crate::OutputFormat;
use serde_json::json;
use specforge_ops::extension::{self, ExtensionEntry, Origin};
use std::path::Path;

/// `specforge extensions`: every extension the project enables, has
/// installed or loaded, alphabetically, with the entity kinds each
/// registered and how many of the project's entities use them.
pub fn run(path: &Path, format: OutputFormat) -> i32 {
    let ctx = crate::pipeline::compile(path);
    let entries = extension::list(path, &ctx.manifests, &ctx.kind_registry, &ctx.graph);

    match format {
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = entries.iter().map(entry_json).collect();
            let output = json!({
                "extensions": items,
                "count": items.len(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            if entries.is_empty() {
                println!("No extensions installed.");
                println!();
                println!("Install one with: specforge add <extension>");
                return 0;
            }
            println!("Installed extensions:");
            println!();
            for entry in &entries {
                let version = entry
                    .version
                    .as_deref()
                    .map(|v| format!(" v{v}"))
                    .unwrap_or_default();
                let provides = if entry.entity_kinds.is_empty() {
                    String::new()
                } else {
                    format!(
                        ": {} entities ({})",
                        entry.entity_count,
                        entry.entity_kinds.join(", ")
                    )
                };
                let status = match entry.status {
                    extension::Status::Loaded => String::new(),
                    other => format!(" [{}]", other.as_str()),
                };
                println!(
                    "  {}{version} ({}){status}{provides}",
                    entry.name,
                    source(&entry.origin)
                );
            }
            println!();
            println!("{} extension(s) installed.", entries.len());
        }
    }

    0
}

fn source(origin: &Origin) -> &str {
    match origin {
        Origin::Builtin => "builtin",
        Origin::Installed { source } => source,
    }
}

fn entry_json(entry: &ExtensionEntry) -> serde_json::Value {
    json!({
        "name": entry.name,
        "version": entry.version,
        "source": source(&entry.origin),
        "status": entry.status.as_str(),
        "entity_kinds": entry.entity_kinds,
        "entity_count": entry.entity_count,
        "validation_rules": entry.validation_rules,
    })
}
