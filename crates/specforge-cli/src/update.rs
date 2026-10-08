use crate::OutputFormat;
use crate::outcome::Refusal;
use serde_json::json;
use specforge_ops::extension::{self, Trust, UpdateRequest};
use specforge_ops_registry::ConfiguredRegistry;
use std::path::Path;

/// `specforge update`: the shared update operation, presented. Exit 1
/// when it fails outright or when any extension fails (nothing is then
/// applied).
pub fn run(
    name: Option<&str>,
    path: &Path,
    format: OutputFormat,
    major: bool,
    allow_unsigned: bool,
    assume_yes: bool,
) -> i32 {
    let registry = ConfiguredRegistry::for_project(path, "update");
    let request = UpdateRequest {
        root: path,
        name,
        major,
        allow_unsigned,
        trust: match (assume_yes, format) {
            (true, _) => Trust::AssumeYes,
            (false, OutputFormat::Json) => Trust::Refuse,
            (false, OutputFormat::Human) => Trust::Prompt,
        },
    };
    let updated = extension::update(&request, &registry);
    format.eprint_diagnostics(registry.reported());
    let outcome = match updated {
        Ok(outcome) => outcome,
        Err(error) => {
            return Refusal::of(format).report(&error);
        }
    };
    // The update ran to the end (applied or rolled back): it emits
    // `batch_update_completed`, which JSON output carries.
    let completed = batch_update_completed(&outcome);

    if let Some((_, first)) = outcome.failures().next() {
        let failed: Vec<_> = outcome
            .failures()
            .map(|(name, e)| json!({"name": name, "code": e.code, "error": e.message}))
            .collect();
        match format {
            OutputFormat::Json => {
                let mut output = crate::outcome::error_document(first, None);
                output["failed"] = json!(failed);
                output["updated"] = json!([]);
                output["batch_update_completed"] = completed;
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Human => {
                for (name, e) in outcome.failures() {
                    eprintln!("error[{}]: {name}: {}", e.code, e.message);
                    if let Some(hint) = &e.suggestion {
                        eprintln!("  hint: {hint}");
                    }
                }
                eprintln!("no extension was updated");
            }
        }
        return 1;
    }

    let updated: Vec<_> = outcome
        .updated()
        .map(|(name, from, to)| json!({"name": name, "from": from, "to": to}))
        .collect();
    match format {
        OutputFormat::Json => {
            let output = json!({"updated": updated, "batch_update_completed": completed});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human if outcome.extensions.is_empty() => {
            println!("no extensions to update");
        }
        OutputFormat::Human if updated.is_empty() => println!("all extensions are up to date"),
        OutputFormat::Human => {
            println!("updated {} extension(s):", updated.len());
            for (name, from, to) in outcome.updated() {
                println!("  {name} {from} -> {to}");
            }
        }
    }
    0
}

/// The `batch_update_completed` event's payload
/// (`spec/events/wasm-extensions.spec`), stamped now.
fn batch_update_completed(outcome: &extension::UpdateOutcome) -> serde_json::Value {
    let counts = outcome.batch_update_completed();
    json!({
        "updatedCount": counts.updated_count,
        "failedCount": counts.failed_count,
        "skippedCount": counts.skipped_count,
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    })
}
