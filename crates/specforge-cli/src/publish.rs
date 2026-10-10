//! `specforge publish`: the publish operation, presented.

use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
use serde_json::json;
use specforge_component::ComponentRuntime;
use specforge_ops::publish::{self, PublishOutcome};
use specforge_ops_registry::ConfiguredRegistry;
use std::path::Path;

/// Publish the extension at `extension` (a `.wasm` component or its crate
/// directory) to the registry that serves its name among those `project`'s
/// `specforge.json` configures (ADR 0045). The package's manifest is the
/// declaration read from the binary (ADR 0012): a binary whose declaration
/// has errors is refused before any network call. The declaration's
/// warnings, then what reading the registry configuration reported, go to
/// stderr whatever the result.
pub(crate) fn run(extension: &Path, project: &Path, format: OutputFormat) -> Exit {
    let registry = ConfiguredRegistry::for_project(project, "publish");
    let runtime = ComponentRuntime::with_user_cache();
    let report = publish::publish(extension, &registry, &runtime);
    format.eprint_diagnostics(&report.warnings);
    format.eprint_diagnostics(&registry.reported());
    match &report.result {
        Ok(outcome) => {
            present(outcome, format);
            Exit::Passed
        }
        Err(error) => Refusal::of(format).report(error),
    }
}

fn present(outcome: &PublishOutcome, format: OutputFormat) {
    let published = &outcome.published;
    match format {
        OutputFormat::Json => {
            let output = json!({
                "action": "publish",
                "name": outcome.name.as_str(),
                "version": outcome.version.to_string(),
                "registry": published.registry,
                "url": published.url,
                "size_bytes": outcome.size_bytes,
                "key_id": published.key_id,
                "signed": true,
                "key_created": published.key_created,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Human => {
            if published.key_created {
                println!("generated publisher signing key {}", published.key_id);
            }
            println!("published {} v{}", outcome.name, outcome.version);
            println!("  registry: {}", published.registry);
            println!("  url: {}", published.url);
            println!("  size: {} bytes", outcome.size_bytes);
            println!("  signed by key: {}", published.key_id);
        }
    }
}
