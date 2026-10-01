use crate::OutputFormat;
use serde_json::json;
use specforge_ops::extension::{self, AddOutcome, AddRequest, Origin, Source, Trust};
use specforge_ops::registry::HttpRegistry;
use std::path::Path;

/// `specforge add`: the shared add operation, presented.
pub fn run(
    specifier: &str,
    path: &Path,
    format: OutputFormat,
    allow_unsigned: bool,
    assume_yes: bool,
) -> i32 {
    let source = match extension::parse(specifier) {
        Ok(source) => source,
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };
    let registry = HttpRegistry::for_project(path, "add");
    // Only a registry package reads the registry configuration.
    if matches!(source, Source::Registry { .. }) {
        format.eprint_diagnostics(registry.diagnostics());
    }
    let request = AddRequest {
        root: path,
        source,
        allow_unsigned,
        trust: match (assume_yes, format) {
            (true, _) => Trust::AssumeYes,
            (false, OutputFormat::Json) => Trust::Refuse,
            (false, OutputFormat::Human) => Trust::Prompt,
        },
        dry_run: false,
    };
    match extension::add(&request, &registry) {
        Ok(outcome) => {
            present(&outcome, format);
            0
        }
        Err(error) => {
            format.print_op_error(&error);
            1
        }
    }
}

fn present(outcome: &AddOutcome, format: OutputFormat) {
    match (outcome, format) {
        (
            AddOutcome::Builtin {
                name,
                changed,
                peers_enabled,
            },
            OutputFormat::Json,
        ) => print_json(json!({
            "action": "add",
            "name": name,
            "source": "builtin",
            "changed": changed,
            "peers_enabled": peers_enabled,
        })),
        (
            AddOutcome::Builtin {
                name,
                changed,
                peers_enabled,
            },
            OutputFormat::Human,
        ) => {
            for peer in peers_enabled {
                println!("enabled builtin {peer} (required by {name})");
            }
            if *changed {
                println!("enabled builtin {}", name);
            } else {
                println!("{} is already enabled", name);
            }
        }
        (
            AddOutcome::Installed {
                name,
                version,
                sha256,
                key_id,
                origin,
            },
            OutputFormat::Json,
        ) => {
            let mut output = json!({
                "action": "add",
                "name": name,
                "version": version,
                "sha256": sha256,
            });
            match origin {
                Origin::Installed { source } if source != "registry" => {
                    output["source"] = json!(source);
                }
                _ => output["key_id"] = json!(key_id),
            }
            print_json(output);
        }
        (
            AddOutcome::Installed {
                name,
                version,
                key_id,
                origin,
                ..
            },
            OutputFormat::Human,
        ) => match origin {
            Origin::Installed { source } if source != "registry" => {
                println!("installed {} from local path", name);
            }
            _ => {
                println!("installed {} v{}", name, version);
                match key_id {
                    Some(key_id) => println!("  signed by key: {}", key_id),
                    None => println!("  unsigned"),
                }
            }
        },
        (AddOutcome::AlreadyPresent { name, version }, OutputFormat::Json) => print_json(json!({
            "action": "none",
            "name": name,
            "version": version,
            "already_present": true,
        })),
        (AddOutcome::AlreadyPresent { name, version }, OutputFormat::Human) => {
            println!("{name} {version} is already installed");
        }
        // `specforge add` has no dry run.
        (AddOutcome::Planned { .. }, _) => {}
    }
}

fn print_json(value: serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}
