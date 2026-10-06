use crate::OutputFormat;
use serde_json::json;
use specforge_ops::extension::{self, Origin, RemoveRequest};
use std::path::Path;

/// `specforge remove`: the shared remove operation over a fresh compile of
/// the project, whose loaded declarations say which extensions depend on the
/// one removed.
pub fn run(name: &str, path: &Path, force: bool, format: OutputFormat) -> i32 {
    let ctx = crate::pipeline::compile(path);
    let request = RemoveRequest {
        root: path,
        name,
        force,
        dry_run: false,
        loaded: &ctx.declarations,
        kinds: &ctx.kind_registry,
        graph: &ctx.graph,
    };
    let outcome = match extension::remove(&request) {
        Ok(outcome) => outcome,
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };

    match format {
        OutputFormat::Json => {
            let mut output = json!({
                "removed": outcome.name,
                "orphan_warnings": outcome.orphan_warnings,
            });
            match &outcome.origin {
                Origin::Builtin => output["source"] = json!("builtin"),
                Origin::Installed { .. } | Origin::File { .. } => {
                    output["version"] = json!(outcome.version)
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
                Origin::Installed { .. } | Origin::File { .. } => println!(
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
