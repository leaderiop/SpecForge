//! Bridge between protocol descriptor types and manifest registry types.
//!
//! Converts `ProtocolExtension` -> `ManifestV2` so that existing
//! `populate_registries()` logic can be reused without duplication.

use specforge_common::Diagnostic;
use specforge_registry::{
    EdgeRegistry, FieldRegistry, KindRegistry, ManifestV2, SurfaceContributions,
    populate_registries,
};

use super::host::ProtocolExtension;
use super::types::*;

/// Convert a `ProtocolExtension` into a `ManifestV2` for registry population.
///
/// This is a pure data transformation -- no I/O, no Wasm calls.
/// Synthetic fields: `manifest_version` = 2, `wasm_path` = "" (unused by populate_registries).
pub fn protocol_extension_to_manifest(ext: &ProtocolExtension) -> ManifestV2 {
    // H7: Derive verify_kinds from all entity kinds' verify_kinds (deduplicated, order-preserving).
    let verify_kinds = collect_verify_kinds(&ext.descriptions.entity_kinds);

    ManifestV2 {
        name: ext.name.clone(),
        version: ext.version.clone(),
        manifest_version: 2,
        wasm_path: "builtin".to_string(),
        contributes: convert_contribution_flags(&ext.handshake.contribution_flags),
        entity_kinds: ext
            .descriptions
            .entity_kinds
            .iter()
            .map(convert_entity_kind)
            .collect(),
        edge_types: ext
            .descriptions
            .edge_types
            .iter()
            .map(convert_edge_type)
            .collect(),
        validation_rules: ext
            .descriptions
            .validation_rules
            .iter()
            .map(convert_validation_rule)
            .collect(),
        verify_kinds,
        fields: ext
            .descriptions
            .shared_fields
            .iter()
            .map(convert_field)
            .collect(),
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: None,
        peer_dependencies: ext
            .handshake
            .peer_dependencies
            .iter()
            .map(convert_peer_dependency)
            .collect(),
        sandbox_policy: ext.handshake.sandbox_policy.as_ref().map(convert_sandbox_policy),
        host_api_version: None,
        entity_enhancements: ext
            .descriptions
            .enhancements
            .iter()
            .map(convert_enhancement)
            .collect(),
        starter_template: None,
        grammar_contributions: ext
            .descriptions
            .grammars
            .iter()
            .map(convert_grammar)
            .collect(),
        body_parser_contributions: ext
            .descriptions
            .body_parsers
            .iter()
            .map(convert_body_parser)
            .collect(),
        ext_short: None,
        query_scope: None,
        collector_contributions: ext
            .descriptions
            .collectors
            .iter()
            .map(convert_collector)
            .collect(),
        // H7: Map surfaces from protocol data instead of hardcoding None.
        surfaces: ext
            .descriptions
            .surfaces
            .as_ref()
            .map(convert_surface_descriptor),
    }
}

/// Collect all unique verify_kinds from entity kind descriptors, preserving insertion order.
fn collect_verify_kinds(entity_kinds: &[EntityKindDescriptor]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for kind in entity_kinds {
        for vk in &kind.verify_kinds {
            if seen.insert(vk.clone()) {
                result.push(vk.clone());
            }
        }
    }
    result
}

fn convert_contribution_flags(
    flags: &ContributionFlags,
) -> specforge_registry::ExtensionContributions {
    specforge_registry::ExtensionContributions {
        entities: flags.entities,
        validators: flags.validators,
        renderers: flags.renderers,
        providers: flags.providers,
        collectors: flags.collectors,
        prompts: flags.prompts,
        parsers: flags.parsers,
        grammars: flags.grammars,
        body_parsers: flags.body_parsers,
    }
}

fn convert_peer_dependency(
    dep: &PeerDependency,
) -> specforge_registry::PeerDependency {
    specforge_registry::PeerDependency {
        name: dep.name.clone(),
        version: dep.version.clone(),
        optional: dep.optional,
    }
}

fn convert_sandbox_policy(
    policy: &SandboxPolicy,
) -> specforge_registry::SandboxPolicy {
    specforge_registry::SandboxPolicy {
        max_memory_mb: policy.max_memory_mb,
        max_execution_ms: policy.max_execution_ms,
        allowed_domains: policy.allowed_domains.clone(),
        allowed_paths: policy.allowed_paths.clone(),
        allowed_output_extensions: policy.allowed_output_extensions.clone(),
        network_access: policy.network_access,
        file_system_access: policy.file_system_access,
    }
}

fn convert_entity_kind(desc: &EntityKindDescriptor) -> specforge_registry::ManifestEntityKind {
    let keyword = desc.keyword.clone().unwrap_or_else(|| desc.name.clone());
    specforge_registry::ManifestEntityKind {
        name: desc.name.clone(),
        keyword,
        description: desc.description.clone(),
        testable: desc.testable,
        singleton: desc.singleton,
        supports_verify: desc.supports_verify,
        allowed_verify_kinds: desc.verify_kinds.clone(),
        semantic_token: desc.semantic_token.clone(),
        lsp_icon: desc.lsp_icon.clone(),
        dot_shape: desc.dot_shape.clone(),
        dot_color: desc.dot_color.clone(),
        dot_fillcolor: desc.dot_fillcolor.clone(),
        fields: desc.fields.iter().map(convert_field).collect(),
        incremental: desc.incremental,
        has_body_parser: desc.has_body_parser,
        open_fields: desc.open_fields,
    }
}

/// H6: Map all FieldDescriptor fields including default_value and enum_values.
fn convert_field(desc: &FieldDescriptor) -> specforge_registry::ManifestField {
    specforge_registry::ManifestField {
        name: desc.name.clone(),
        field_type: desc.field_type.clone(),
        description: desc.description.clone(),
        edge: desc.edge.clone(),
        target_kind: desc.target_kind.clone(),
        file_reference: desc.file_reference,
        required: desc.required,
        default_value: desc.default_value.clone(),
        enum_values: desc.enum_values.clone(),
    }
}

fn convert_edge_type(desc: &EdgeTypeDescriptor) -> specforge_registry::ManifestEdgeType {
    specforge_registry::ManifestEdgeType {
        label: desc.label.clone(),
        description: desc.description.clone(),
        source_kind: desc.source_kind.clone(),
        target_kind: desc.target_kind.clone(),
        edge_style: desc.edge_style.clone(),
        edge_color: desc.edge_color.clone(),
        edge_arrowhead: desc.edge_arrowhead.clone(),
    }
}

/// H5: Map all EntityEnhancementDescriptor fields including edge_types.
fn convert_enhancement(
    desc: &EntityEnhancementDescriptor,
) -> specforge_registry::FieldEnhancement {
    specforge_registry::FieldEnhancement {
        target_kind: desc.target_kind.clone(),
        source_extension: desc.source_extension.clone(),
        fields: desc.fields.iter().map(convert_field).collect(),
        edge_types: desc.edge_types.iter().map(convert_edge_type).collect(),
    }
}

fn convert_validation_rule(
    desc: &ValidationRuleDescriptor,
) -> specforge_registry::ManifestValidationRule {
    let severity = match desc.severity {
        ValidationSeverity::Error => "error".to_string(),
        ValidationSeverity::Warning => "warning".to_string(),
        ValidationSeverity::Info => "info".to_string(),
    };
    specforge_registry::ManifestValidationRule {
        code: desc.code.clone(),
        severity,
        message_template: desc.message_template.clone(),
        check: desc.check.clone(),
        target_kind: desc.target_kind.clone(),
        edge_type: desc.edge_type.clone(),
        field: desc.field.clone(),
        constraint: desc.constraint.as_ref().map(convert_field_constraint),
        wasm_function: desc.wasm_function.clone(),
    }
}

fn convert_field_constraint(
    desc: &FieldConstraintDescriptor,
) -> specforge_registry::FieldConstraint {
    specforge_registry::FieldConstraint {
        kind: desc.kind.clone(),
        pattern: desc.pattern.clone(),
        values: desc.values.clone(),
    }
}

fn convert_grammar(desc: &GrammarDescriptor) -> specforge_registry::GrammarContribution {
    specforge_registry::GrammarContribution {
        entity_kind: desc.entity_kind.clone(),
        grammar_wasm_path: desc.grammar_wasm_path.clone(),
        export_name: desc.export_name.clone(),
    }
}

fn convert_body_parser(desc: &BodyParserDescriptor) -> specforge_registry::BodyParserContribution {
    specforge_registry::BodyParserContribution {
        entity_kind: desc.entity_kind.clone(),
        export_name: desc.export_name.clone(),
    }
}

fn convert_collector(desc: &CollectorDescriptor) -> specforge_registry::CollectorContribution {
    specforge_registry::CollectorContribution {
        name: desc.name.clone(),
        input_formats: desc.input_formats.clone(),
        export: desc.export.clone(),
        auto_detect: desc.auto_detect.as_ref().map(|ad| {
            specforge_registry::CollectorAutoDetect {
                file_patterns: ad.file_patterns.clone(),
                env_vars: ad.env_vars.clone(),
            }
        }),
    }
}

fn convert_surface_descriptor(desc: &SurfaceDescriptor) -> SurfaceContributions {
    SurfaceContributions {
        commands: desc.commands.iter().map(|c| {
            specforge_registry::CommandContribution {
                id: c.id.clone(),
                title: c.title.clone(),
                description: c.description.clone(),
                category: c.category.clone(),
                export: c.export.clone(),
                args: c.args.iter().map(|a| {
                    specforge_registry::CommandArg {
                        name: a.name.clone(),
                        arg_type: convert_command_arg_type(&a.arg_type),
                        required: a.required,
                        default_value: a.default_value.clone(),
                        description: a.description.clone(),
                    }
                }).collect(),
                sandbox: c.sandbox.as_ref().map(convert_surface_sandbox),
            }
        }).collect(),
        mcp_tools: desc.mcp_tools.iter().map(|t| {
            specforge_registry::McpToolContribution {
                name: t.name.clone(),
                description: t.description.clone(),
                category: t.category.clone(),
                export: t.export.clone(),
                input_schema: t.input_schema.clone(),
                output_schema: t.output_schema.clone(),
                sandbox: t.sandbox.as_ref().map(convert_surface_sandbox),
            }
        }).collect(),
        mcp_resources: desc.mcp_resources.iter().map(|r| {
            specforge_registry::McpResourceContribution {
                uri_template: r.uri_template.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                export: r.export.clone(),
                mime_type: r.mime_type.clone(),
                sandbox: r.sandbox.as_ref().map(convert_surface_sandbox),
            }
        }).collect(),
    }
}

fn convert_command_arg_type(t: &CommandArgType) -> specforge_registry::CommandArgType {
    match t {
        CommandArgType::String => specforge_registry::CommandArgType::StringArg,
        CommandArgType::Path => specforge_registry::CommandArgType::PathArg,
        CommandArgType::Bool => specforge_registry::CommandArgType::BoolArg,
        CommandArgType::Integer => specforge_registry::CommandArgType::IntegerArg,
        CommandArgType::Enum { values } => {
            specforge_registry::CommandArgType::EnumArg { values: values.clone() }
        }
    }
}

fn convert_surface_sandbox(
    s: &SurfaceSandboxOverride,
) -> specforge_registry::SurfaceSandboxOverride {
    specforge_registry::SurfaceSandboxOverride {
        fs_read: s.fs_read,
        fs_write: s.fs_write,
        network: s.network,
    }
}

/// Convert protocol surface descriptors to the format expected by `register_surface_contributions()`.
pub fn protocol_surfaces_to_manifest(
    extensions: &[ProtocolExtension],
) -> Vec<(String, Option<SurfaceContributions>)> {
    extensions
        .iter()
        .map(|ext| {
            let surfaces = ext
                .descriptions
                .surfaces
                .as_ref()
                .map(convert_surface_descriptor);
            (ext.name.clone(), surfaces)
        })
        .collect()
}

/// Populate registries from protocol-loaded extensions.
/// Converts to `ManifestV2` internally and delegates to `populate_registries()`.
pub fn populate_from_protocol(
    extensions: &[ProtocolExtension],
) -> (KindRegistry, FieldRegistry, EdgeRegistry, Vec<Diagnostic>) {
    let manifests: Vec<ManifestV2> = extensions
        .iter()
        .map(protocol_extension_to_manifest)
        .collect();
    populate_registries(&manifests)
}
