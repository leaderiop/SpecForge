use crate::OutputFormat;
use serde_json::json;
use specforge_common::find_project_root;
use specforge_ops::init;
use std::path::Path;

/// `specforge init`: the shared scaffold (`specforge_ops::init`), planned
/// and validated before anything is written, then presented.
pub fn run(
    path: &Path,
    name: Option<&str>,
    version: Option<&str>,
    extensions: &[String],
    format: OutputFormat,
) -> i32 {
    let request = init::Request {
        dir: path,
        name,
        version,
        extensions,
        // A project further up doesn't block init: the new one is separate,
        // and commands run inside it resolve to it (the nearest wins).
        forbid_inside: None,
    };
    let plan = match init::plan(&request) {
        Ok(plan) => plan,
        Err(error) => return fail(&error),
    };
    if let Some(enclosing) = path.parent().and_then(find_project_root)
        && format != OutputFormat::Json
    {
        eprintln!(
            "note: {} is inside the project at {}; the new project is separate",
            path.display(),
            enclosing.display()
        );
    }
    let outcome = match init::apply(path, &plan) {
        Ok(outcome) => outcome,
        Err(error) => return fail(&error),
    };

    match format {
        OutputFormat::Json => {
            let output = json!({
                "project_root": outcome.root,
                "config_path": outcome.config_path,
                "spec_file_path": outcome.starter_path,
                "extensions_installed": outcome.extensions,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize JSON output")
            );
        }
        OutputFormat::Human => {
            println!(
                "Initialized project '{}' at {}",
                outcome.name,
                path.display()
            );
            println!("  specforge.json");
            println!("  {}", init::STARTER_FILE);
            if outcome.extensions.is_empty() {
                println!("\nNo extensions installed. Add one with: specforge add <extension>");
            }
            println!("\nNext steps:");
            println!("  specforge check    # validate your spec files");
            println!("  specforge export   # export the graph");
            if outcome
                .extensions
                .iter()
                .any(|e| e == "@specforge/cargo-test" || e == "@specforge/vitest")
            {
                println!("  specforge collect  # run the tests and record what they prove");
            }
        }
    }
    0
}

/// An init error on stderr, as `error: message` with its hint; exit 1.
fn fail(error: &specforge_ops::OpError) -> i32 {
    eprintln!("error: {}", error.message);
    if let Some(suggestion) = &error.suggestion {
        eprintln!("  hint: {suggestion}");
    }
    1
}
