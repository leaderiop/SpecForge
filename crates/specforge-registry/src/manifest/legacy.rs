//! A `ManifestV2` as the declaration the registry build reads, for the
//! manifest-based public functions that outlive the build's move to
//! declarations (plan 03 T5) until the manifest types are deleted (T10).
//! Their one implementation is the declaration-based one.

use specforge_protocol_types::{
    AnalyzerDescriptor, AutoDetectConfig, CollectorDescriptor, CommandArgDescriptor,
    CommandDescriptor, ContributionFlags, EdgeTypeDescriptor, EntityEnhancementDescriptor,
    EntityKindDescriptor, ExtensionDeclaration, FieldConstraintDescriptor, FieldDescriptor,
    HandshakeResponse, McpResourceDescriptor, McpToolDescriptor, SandboxPolicy, SurfaceDescriptor,
    SurfaceSandboxOverride, ValidationRuleDescriptor, ValidationSeverity,
};

use super::surface::{SurfaceContributions, SurfaceSandboxOverride as ManifestSandboxOverride};
use super::types::{
    ManifestEdgeType, ManifestEntityKind, ManifestField, ManifestV2, ManifestValidationRule,
};

/// `manifest` as the declaration it describes.
pub(crate) fn to_declaration(manifest: &ManifestV2) -> ExtensionDeclaration {
    let c = &manifest.contributes;
    ExtensionDeclaration {
        handshake: HandshakeResponse {
            protocol_version: specforge_protocol_types::PROTOCOL_VERSION.to_string(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            contribution_flags: ContributionFlags {
                entities: c.entities,
                validators: c.validators,
                renderers: c.renderers,
                providers: c.providers,
                collectors: c.collectors,
                prompts: c.prompts,
                parsers: c.parsers,
                grammars: c.grammars,
                body_parsers: c.body_parsers,
                analyzers: c.analyzers,
            },
            peer_dependencies: manifest.peer_dependencies.clone(),
            sandbox_policy: manifest.sandbox_policy.as_ref().map(|p| SandboxPolicy {
                max_memory_mb: p.max_memory_mb,
                max_execution_ms: p.max_execution_ms,
                allowed_domains: p.allowed_domains.clone(),
                allowed_paths: p.allowed_paths.clone(),
                allowed_output_extensions: p.allowed_output_extensions.clone(),
                network_access: p.network_access,
                file_system_access: p.file_system_access,
            }),
            starter_template: manifest.starter_template.clone(),
            migration_hook: manifest.migration_hook.clone(),
            theme_color: manifest.theme_color.clone(),
            ext_short: manifest.ext_short.clone(),
            description: None,
            keywords: Vec::new(),
        },
        entities: manifest.entity_kinds.iter().map(kind).collect(),
        edges: manifest.edge_types.iter().map(edge).collect(),
        shared_fields: manifest.fields.iter().map(field).collect(),
        enhancements: manifest
            .entity_enhancements
            .iter()
            .map(|e| EntityEnhancementDescriptor {
                target_kind: e.target_kind.clone(),
                source_extension: e.source_extension.clone(),
                fields: e.fields.iter().map(field).collect(),
                edge_types: e.edge_types.iter().map(edge).collect(),
                verify_kinds: e.verify_kinds.clone(),
            })
            .collect(),
        validation_rules: manifest.validation_rules.iter().map(rule).collect(),
        surfaces: manifest.surfaces.as_ref().map(surfaces).unwrap_or_default(),
        collectors: manifest
            .collector_contributions
            .iter()
            .map(|c| CollectorDescriptor {
                name: c.name.clone(),
                input_formats: c.input_formats.clone(),
                export: c.export.clone(),
                auto_detect: c.auto_detect.as_ref().map(|a| AutoDetectConfig {
                    file_patterns: a.file_patterns.clone(),
                    env_vars: a.env_vars.clone(),
                }),
                run: c.run.clone(),
                report: c.report.clone(),
                capture: c.capture.clone(),
            })
            .collect(),
        analyzers: manifest
            .analyzer_contributions
            .iter()
            .map(|a| AnalyzerDescriptor {
                language: a.language.clone(),
                file_extensions: a.file_extensions.clone(),
                excluded_dirs: a.excluded_dirs.clone(),
                scan_export: a.scan_export.clone(),
                classify_export: a.classify_export.clone(),
                map_export: a.map_export.clone(),
                description: a.description.clone(),
            })
            .collect(),
        passes: Vec::new(),
        feature_flags: Vec::new(),
    }
}

fn kind(k: &ManifestEntityKind) -> EntityKindDescriptor {
    EntityKindDescriptor {
        name: k.name.clone(),
        keyword: Some(k.keyword.clone()),
        description: k.description.clone(),
        fields: k.fields.iter().map(field).collect(),
        testable: k.testable,
        singleton: k.singleton,
        supports_verify: k.supports_verify,
        incremental: k.incremental,
        has_body_parser: k.has_body_parser,
        open_fields: k.open_fields,
        semantic_token: k.semantic_token.clone(),
        lsp_icon: k.lsp_icon.clone(),
        dot_shape: k.dot_shape.clone(),
        dot_color: k.dot_color.clone(),
        dot_fillcolor: k.dot_fillcolor.clone(),
        verify_kinds: k.allowed_verify_kinds.clone(),
        inference_guide: k.inference_guide.clone(),
        contract_target: k.contract_target,
        declares_types: k.declares_types,
        lifecycle_field: k.lifecycle_field.clone(),
    }
}

fn field(f: &ManifestField) -> FieldDescriptor {
    FieldDescriptor {
        name: f.name.clone(),
        field_type: f.field_type.clone(),
        required: f.required,
        description: f.description.clone(),
        edge: f.edge.clone(),
        target_kind: f.target_kind.clone(),
        file_reference: f.file_reference,
        default_value: f.default_value.clone(),
        enum_values: f.enum_values.clone(),
        inverse_of: f.inverse_of.clone(),
        normative: f.normative,
        exempts_obligations: f.exempts_obligations,
        headline: f.headline,
        derived_from: f.derived_from.clone(),
        proof_role: f.proof_role.clone(),
    }
}

fn edge(e: &ManifestEdgeType) -> EdgeTypeDescriptor {
    EdgeTypeDescriptor {
        label: e.label.clone(),
        description: e.description.clone(),
        source_kind: e.source_kind.clone(),
        target_kind: e.target_kind.clone(),
        edge_style: e.edge_style.clone(),
        edge_color: e.edge_color.clone(),
        edge_arrowhead: e.edge_arrowhead.clone(),
    }
}

fn rule(r: &ManifestValidationRule) -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: r.code.clone(),
        // A manifest's unknown severity always read as a warning.
        severity: match r.severity.as_str() {
            "error" => ValidationSeverity::Error,
            "info" => ValidationSeverity::Info,
            _ => ValidationSeverity::Warning,
        },
        message_template: r.message_template.clone(),
        check: r.check.clone(),
        target_kind: r.target_kind.clone(),
        edge_type: r.edge_type.clone(),
        field: r.field.clone(),
        constraint: r.constraint.as_ref().map(|c| FieldConstraintDescriptor {
            kind: c.kind.clone(),
            pattern: c.pattern.clone(),
            values: c.values.clone(),
        }),
        wasm_function: r.wasm_function.clone(),
    }
}

fn sandbox(s: &ManifestSandboxOverride) -> SurfaceSandboxOverride {
    SurfaceSandboxOverride {
        fs_read: s.fs_read,
        fs_write: s.fs_write,
        network: s.network,
    }
}

/// Manifest surfaces as the declaration's.
pub(crate) fn surfaces(s: &SurfaceContributions) -> SurfaceDescriptor {
    SurfaceDescriptor {
        commands: s
            .commands
            .iter()
            .map(|c| CommandDescriptor {
                id: c.id.clone(),
                title: c.title.clone(),
                description: c.description.clone(),
                category: c.category.clone(),
                export: c.export.clone(),
                args: c
                    .args
                    .iter()
                    .map(|a| CommandArgDescriptor {
                        name: a.name.clone(),
                        arg_type: a.arg_type.clone(),
                        required: a.required,
                        default_value: a.default_value.clone(),
                        description: a.description.clone(),
                    })
                    .collect(),
                sandbox: c.sandbox.as_ref().map(sandbox),
            })
            .collect(),
        mcp_tools: s
            .mcp_tools
            .iter()
            .map(|t| McpToolDescriptor {
                name: t.name.clone(),
                description: t.description.clone(),
                category: t.category.clone(),
                export: t.export.clone(),
                input_schema: t.input_schema.clone(),
                output_schema: t.output_schema.clone(),
                sandbox: t.sandbox.as_ref().map(sandbox),
            })
            .collect(),
        mcp_resources: s
            .mcp_resources
            .iter()
            .map(|r| McpResourceDescriptor {
                uri_template: r.uri_template.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                export: r.export.clone(),
                mime_type: r.mime_type.clone(),
                sandbox: r.sandbox.as_ref().map(sandbox),
            })
            .collect(),
    }
}
