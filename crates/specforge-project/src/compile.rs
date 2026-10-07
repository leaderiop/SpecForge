//! The graph build's inputs from a registry build, and loading the
//! extensions they come from. The checks over a built graph's entities are
//! `RegistryBuild::check`'s (ADR 0031).

use specforge_common::{Diagnostic, ExtensionEntry, codes};
use specforge_graph::GraphConfig;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::RegistryBuild;
use specforge_wasm::WasmRuntime;
use std::collections::HashSet;

/// The graph build's inputs, from a registry build. Every surface that
/// builds a graph (`check`, watch, the LSP) takes its `GraphConfig` from
/// here, so none can drift.
pub fn graph_config(build: &RegistryBuild) -> GraphConfig {
    GraphConfig {
        known_provider_schemes: HashSet::new(),
        bidirectional_pairs: build.bidirectional_pairs.clone(),
        body_parser_kinds: build.body_parser_kinds.clone(),
        single_reference_fields: build.single_reference_fields.clone(),
        absent_reference_targets: build.absent_reference_targets.clone(),
        field_coercions: crate::field_types::field_coercions(&build.fields),
        derived_references: crate::field_types::derived_references(&build.fields),
    }
}

/// What one `specforge.json` `extensions` entry enables, as the runtime
/// loaded it: the entry read by [`ExtensionEntry`], the rule the runtime
/// (`specforge_component::project_runtime`) loads it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnabledExtension {
    /// The entry as `specforge.json` writes it (trimmed).
    pub entry: String,
    /// The extension's name: a named entry's; for a `.wasm` file entry the
    /// name its component declares once it loaded, else the name written
    /// before `=`, else the path.
    pub name: String,
    /// The path a `.wasm` file entry names, as written.
    pub file: Option<String>,
}

impl EnabledExtension {
    /// What `entry` enables, as `runtime` (if any) loaded it.
    pub fn of(entry: &str, runtime: Option<&dyn WasmRuntime>) -> Self {
        match ExtensionEntry::parse(entry) {
            ExtensionEntry::Named(name) => EnabledExtension {
                entry: entry.trim().to_string(),
                name: name.to_string(),
                file: None,
            },
            ExtensionEntry::File { name, path } => EnabledExtension {
                entry: entry.trim().to_string(),
                name: runtime
                    .and_then(|runtime| runtime.file_entry_extension(entry.trim()))
                    .or(name.map(str::to_string))
                    .unwrap_or_else(|| path.to_string()),
                file: Some(path.to_string()),
            },
        }
    }
}

/// Load the declarations of `extensions` (as `specforge.json` lists them)
/// through `runtime`, in that order: one [`load_declaration`] per
/// extension, each entry naming the extension [`EnabledExtension::of`]
/// says (an extension two entries enable is read once). An extension that
/// does not load is E028 (or the runtime's own reason, E028/E033, when it
/// knows one) and is left out. `diagnostics` receives those runtime
/// failures in load order, then the load warnings (W153, W138) of the
/// declarations that loaded. What the declarations themselves are worth
/// (E030, W021, E027, W145) is the registry build's to say.
///
/// [`load_declaration`]: specforge_wasm::protocol::load_declaration
pub fn load_extensions(
    extensions: &[String],
    runtime: &dyn WasmRuntime,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ExtensionDeclaration> {
    use specforge_wasm::protocol::load_declaration;

    let mut declarations = Vec::new();
    let mut warnings = Vec::new();
    let mut read = HashSet::new();
    for entry in extensions {
        // A `.wasm` file's failure is known by the entry (what it would
        // have declared is not), and is reported whatever the entry names.
        let (failure_key, file) = match ExtensionEntry::parse(entry) {
            ExtensionEntry::Named(name) => (name, false),
            ExtensionEntry::File { .. } => (entry.trim(), true),
        };
        if file && let Some(failure) = runtime.load_failure(failure_key) {
            diagnostics.push(failure);
            continue;
        }
        let ext_name = EnabledExtension::of(entry, Some(runtime)).name;
        if !read.insert(ext_name.clone()) {
            continue;
        }
        match load_declaration(runtime, &ext_name) {
            Ok(loaded) => {
                warnings.extend(loaded.warnings);
                declarations.push(loaded.declaration);
            }
            // Why the runtime could not load it (a missing or tampered
            // installed binary), when it knows.
            Err(_) if let Some(failure) = runtime.load_failure(failure_key) => {
                diagnostics.push(failure);
            }
            Err(e) => {
                diagnostics.push(Diagnostic::new(
                    codes::E028,
                    format!("extension '{}': protocol loading failed: {}", ext_name, e),
                ));
            }
        }
    }
    diagnostics.extend(warnings);
    declarations
}
