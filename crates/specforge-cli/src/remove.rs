use crate::OutputFormat;
use crate::outcome::Refusal;
use serde_json::json;
use specforge_ops::extension::{self, Origin, RemoveRequest};
use specforge_ops::view::ProjectView;
use std::path::Path;

/// `specforge remove`: the shared remove operation over the view of a fresh
/// compile of the project, whose loaded declarations say which extensions
/// depend on the one removed.
pub fn run(name: &str, path: &Path, force: bool, format: OutputFormat) -> i32 {
    let (project, _runtime) = crate::pipeline::compile_project(path);
    let request = RemoveRequest {
        name,
        force,
        dry_run: false,
    };
    let outcome = match extension::remove(&ProjectView::of(&project), &request) {
        Ok(outcome) => outcome,
        // A removal that failed after editing specforge.json names it.
        Err(error) => {
            return Refusal::of(format).at(path).report(&error);
        }
    };

    match format {
        OutputFormat::Json => {
            let mut output = json!({
                "removed": outcome.name,
                "orphan_warnings": outcome.orphan_warnings,
                "files_written": outcome.writes.names_under(path),
            });
            match &outcome.origin {
                Origin::Builtin => output["source"] = json!("builtin"),
                // `remove` refuses an unlocked non-builtin, so it never
                // reports `Unknown`.
                Origin::Installed { .. } | Origin::Unknown => {
                    output["version"] = json!(outcome.version)
                }
                Origin::File { .. } => {
                    output["version"] = json!(outcome.version);
                    output["source"] = json!(outcome.origin.source());
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            match &outcome.origin {
                Origin::Builtin => println!("Disabled builtin extension '{}'", outcome.name),
                Origin::File { path } => println!(
                    "Disabled extension '{}' loaded from {path} (the file is left in place)",
                    outcome.name
                ),
                Origin::Installed { .. } | Origin::Unknown => println!(
                    "Removed extension '{}' (v{})",
                    outcome.name,
                    outcome.version.as_deref().unwrap_or("?")
                ),
            }
            for warning in &outcome.orphan_warnings {
                eprintln!("warning: {warning}");
            }
        }
    }
    0
}
