//! Strangler bridge (plan 03 T4, deleted with the manifest types): a loaded
//! [`ExtensionDeclaration`] as the `ManifestV2` the registry build still
//! takes. Pure data, no I/O; it carries `ext_short` (the declared short
//! name) through, where the bridge it replaces dropped it.

use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{ManifestV2, SurfaceContributions};

use super::types::*;

/// `declaration` as a `ManifestV2` for the registry build. Synthetic
/// fields: `manifest_version` = 2, `wasm_path` = "builtin".
pub fn declaration_to_manifest(declaration: &ExtensionDeclaration) -> ManifestV2 {
    let handshake = &declaration.handshake;
    // H7: every kind's verify_kinds (deduplicated, order-preserving).
    let verify_kinds = collect_verify_kinds(&declaration.entities);
    let surfaces = (declaration.surfaces != SurfaceDescriptor::default())
        .then(|| convert_surface_descriptor(&declaration.surfaces));

    ManifestV2 {
        name: handshake.name.clone(),
        version: handshake.version.clone(),
        manifest_version: 2,
        wasm_path: "builtin".to_string(),
        contributes: convert_contribution_flags(&declaration.contribution_flags()),
        entity_kinds: declaration
            .entities
            .iter()
            .map(convert_entity_kind)
            .collect(),
        edge_types: declaration.edges.iter().map(convert_edge_type).collect(),
        validation_rules: declaration
            .validation_rules
            .iter()
            .map(convert_validation_rule)
            .collect(),
        verify_kinds,
        fields: declaration
            .shared_fields
            .iter()
            .map(convert_field)
            .collect(),
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: handshake.migration_hook.clone(),
        peer_dependencies: handshake.peer_dependencies.clone(),
        sandbox_policy: handshake
            .sandbox_policy
            .as_ref()
            .map(convert_sandbox_policy),
        host_api_version: None,
        entity_enhancements: declaration
            .enhancements
            .iter()
            .map(convert_enhancement)
            .collect(),
        starter_template: handshake.starter_template.clone(),
        theme_color: handshake.theme_color.clone(),
        ext_short: handshake.ext_short.clone(),
        query_scope: None,
        collector_contributions: declaration
            .collectors
            .iter()
            .map(convert_collector)
            .collect(),
        analyzer_contributions: declaration.analyzers.iter().map(convert_analyzer).collect(),
        surfaces,
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
        analyzers: flags.analyzers,
    }
}

fn convert_sandbox_policy(policy: &SandboxPolicy) -> specforge_registry::SandboxPolicy {
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
        contract_target: desc.contract_target,
        declares_types: desc.declares_types,
        lifecycle_field: desc.lifecycle_field.clone(),
        inference_guide: desc.inference_guide.clone(),
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
        inverse_of: desc.inverse_of.clone(),
        normative: desc.normative,
        exempts_obligations: desc.exempts_obligations,
        headline: desc.headline,
        derived_from: desc.derived_from.clone(),
        proof_role: desc.proof_role.clone(),
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
fn convert_enhancement(desc: &EntityEnhancementDescriptor) -> specforge_registry::FieldEnhancement {
    specforge_registry::FieldEnhancement {
        target_kind: desc.target_kind.clone(),
        source_extension: desc.source_extension.clone(),
        fields: desc.fields.iter().map(convert_field).collect(),
        edge_types: desc.edge_types.iter().map(convert_edge_type).collect(),
        verify_kinds: desc.verify_kinds.clone(),
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

fn convert_collector(desc: &CollectorDescriptor) -> specforge_registry::CollectorContribution {
    specforge_registry::CollectorContribution {
        name: desc.name.clone(),
        input_formats: desc.input_formats.clone(),
        export: desc.export.clone(),
        auto_detect: desc
            .auto_detect
            .as_ref()
            .map(|ad| specforge_registry::CollectorAutoDetect {
                file_patterns: ad.file_patterns.clone(),
                env_vars: ad.env_vars.clone(),
            }),
        run: desc.run.clone(),
        report: desc.report.clone(),
        capture: desc.capture.clone(),
    }
}

fn convert_analyzer(desc: &AnalyzerDescriptor) -> specforge_registry::AnalyzerContribution {
    specforge_registry::AnalyzerContribution {
        language: desc.language.clone(),
        file_extensions: desc.file_extensions.clone(),
        excluded_dirs: desc.excluded_dirs.clone(),
        scan_export: desc.scan_export.clone(),
        classify_export: desc.classify_export.clone(),
        map_export: desc.map_export.clone(),
        description: desc.description.clone(),
    }
}

fn convert_surface_descriptor(desc: &SurfaceDescriptor) -> SurfaceContributions {
    SurfaceContributions {
        commands: desc
            .commands
            .iter()
            .map(|c| specforge_registry::CommandContribution {
                id: c.id.clone(),
                title: c.title.clone(),
                description: c.description.clone(),
                category: c.category.clone(),
                export: c.export.clone(),
                args: c
                    .args
                    .iter()
                    .map(|a| specforge_registry::CommandArg {
                        name: a.name.clone(),
                        arg_type: a.arg_type.clone(),
                        required: a.required,
                        default_value: a.default_value.clone(),
                        description: a.description.clone(),
                    })
                    .collect(),
                sandbox: c.sandbox.as_ref().map(convert_surface_sandbox),
            })
            .collect(),
        mcp_tools: desc
            .mcp_tools
            .iter()
            .map(|t| specforge_registry::McpToolContribution {
                name: t.name.clone(),
                description: t.description.clone(),
                category: t.category.clone(),
                export: t.export.clone(),
                input_schema: t.input_schema.clone(),
                output_schema: t.output_schema.clone(),
                sandbox: t.sandbox.as_ref().map(convert_surface_sandbox),
            })
            .collect(),
        mcp_resources: desc
            .mcp_resources
            .iter()
            .map(|r| specforge_registry::McpResourceContribution {
                uri_template: r.uri_template.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                export: r.export.clone(),
                mime_type: r.mime_type.clone(),
                sandbox: r.sandbox.as_ref().map(convert_surface_sandbox),
            })
            .collect(),
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
