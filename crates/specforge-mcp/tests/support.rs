//! Environments the tests share.

use specforge_mcp::McpServer;
use specforge_registry::{FieldRegistryEntry, ManifestFieldType};

/// Declare `contract` and `status` headline fields of `kind` in the served
/// environment, as `@specforge/software` declares them on `behavior`, so
/// the context export lifts them.
pub fn declare_headline_fields(server: &mut McpServer, kind: &str) {
    server.state_mut().edit_environment(|env| {
        for field in ["contract", "status"] {
            env.registries.fields.register(FieldRegistryEntry {
                kind_name: kind.to_string(),
                field_name: field.to_string(),
                description: None,
                field_type: ManifestFieldType::String,
                source_extension: "@test/ext".to_string(),
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                inverse_of: None,
                normative: field == "contract",
                exempts_obligations: false,
                headline: true,
                derived_from: None,
                proof_role: None,
            });
        }
    });
}
