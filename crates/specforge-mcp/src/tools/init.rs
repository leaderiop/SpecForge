//! `specforge.init`: create a project (`specforge_ops::init`).

use std::path::Path;

use serde_json::json;

use crate::args::Arguments;
use crate::mutation::{Mutated, MutationEvent, Written};
use crate::tool::ToolOutcome;

/// `specforge.init`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Project name (defaults to the directory name)
    name: Option<String>,
    /// Project version
    #[arg(default = specforge_ops::init::DEFAULT_VERSION.to_string())]
    version: String,
    /// Builtin extensions to enable (e.g. @specforge/software) and local .wasm files to install
    extensions: Vec<String>,
}

pub(crate) fn call(path: &Path, runtime: &specforge_project::SharedRuntime, args: Args) -> Mutated {
    use specforge_ops::init;

    // The directory the target names (as given: init creates it).
    let extensions = &args.extensions;

    // The scaffold `specforge init` writes. The call target refused a
    // directory inside the served project before this ran.
    let request = init::Request {
        dir: path,
        name: args.name.as_deref(),
        version: &args.version,
        extensions,
    };
    let outcome =
        match init::plan(&request, runtime.as_ref()).and_then(|plan| init::apply(path, plan)) {
            Ok(outcome) => outcome,
            Err(error) => return Mutated::refused_after(false, error),
        };
    let result = ToolOutcome::ok(json!({
        "project_path": path.display().to_string(),
        "config_file": "specforge.json",
        "starter_file": init::STARTER_FILE,
        "extensions_installed": outcome.extensions,
        "name": outcome.name,
        "version": outcome.version,
    }));
    // With no project served, the server serves the one it created (ADR
    // 0014 D5): `mutation::refresh` does, once it wrote.
    let event = MutationEvent::ProjectInitialized {
        project_name: outcome.name.clone(),
        extension_count: outcome.extensions.len(),
        spec_file_path: init::STARTER_FILE.to_string(),
    };
    Mutated::wrote(result, Written::files(outcome.writes).with_event(event))
}
