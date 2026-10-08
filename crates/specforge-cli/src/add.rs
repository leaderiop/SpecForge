use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
use serde_json::json;
use specforge_ops::extension::{self, AddOutcome, AddRequest, Origin, Trust};
use specforge_ops_registry::ConfiguredRegistry;
use std::path::Path;

/// `specforge add`: the shared add operation, presented.
pub fn run(
    specifier: &str,
    path: &Path,
    format: OutputFormat,
    allow_unsigned: bool,
    trust: Trust,
) -> Exit {
    let source = match extension::parse(specifier) {
        Ok(source) => source,
        Err(error) => {
            return Refusal::of(format).report(&error);
        }
    };
    let registry = ConfiguredRegistry::for_project(path, "add");
    let runtime = specforge_component::ComponentRuntime::with_user_cache();
    let request = AddRequest {
        root: path,
        source,
        allow_unsigned,
        trust,
        dry_run: false,
    };
    let added = extension::add(&request, &registry, &runtime);
    // What reading the registry configuration reported, once the add asked a registry.
    format.eprint_diagnostics(registry.reported());
    match added {
        Ok(added) => {
            present(&added.outcome, &added.writes.names_under(path), format);
            Exit::Passed
        }
        // An install that failed after placing its module names it.
        Err(error) => Refusal::of(format).at(path).report(&error),
    }
}

/// The add's outcome; its JSON lists `files_written` (empty when the
/// extension was already enabled).
fn present(outcome: &AddOutcome, files_written: &[String], format: OutputFormat) {
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
            "files_written": files_written,
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
                "files_written": files_written,
            });
            match origin {
                Origin::Installed { source } if !source.is_registry() => {
                    output["source"] = json!(source.to_string());
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
            Origin::Installed { source } if !source.is_registry() => {
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
            "files_written": files_written,
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
