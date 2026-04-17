use specforge_registry::ManifestFieldType;
use specforge_wasm::protocol::*;

// ── Helper: build a minimal ProtocolExtension ──

fn minimal_protocol_extension(name: &str, entity_kinds: Vec<EntityKindDescriptor>) -> ProtocolExtension {
    ProtocolExtension {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        handshake: HandshakeResponse {
            protocol_version: "1.0".to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            contribution_flags: ContributionFlags {
                entities: !entity_kinds.is_empty(),
                ..Default::default()
            },
            peer_dependencies: vec![],
            sandbox_policy: None,
        },
        descriptions: ExtensionDescriptions {
            entity_kinds,
            ..Default::default()
        },
    }
}

// ── Step 1: Tracer Bullet — minimal entity kind conversion ──

#[test]
fn convert_minimal_entity_kind() {
    let ext = minimal_protocol_extension(
        "@specforge/software",
        vec![EntityKindDescriptor {
            name: "behavior".to_string(),
            keyword: None,
            description: None,
            fields: vec![],
            testable: true,
            singleton: false,
            supports_verify: true,
            incremental: None,
            has_body_parser: false,
            open_fields: false,
            semantic_token: Some("function".to_string()),
            lsp_icon: Some("Method".to_string()),
            dot_shape: Some("ellipse".to_string()),
            dot_color: None,
            dot_fillcolor: None,
            verify_kinds: vec!["smoke".to_string()],
        }],
    );

    let manifest = protocol_extension_to_manifest(&ext);

    assert_eq!(manifest.name, "@specforge/software");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.manifest_version, 2);
    assert_eq!(manifest.entity_kinds.len(), 1);

    let kind = &manifest.entity_kinds[0];
    assert_eq!(kind.keyword, "behavior");
    assert_eq!(kind.name, "behavior"); // name defaults to keyword when no separate display name
    assert!(kind.testable);
    assert!(kind.supports_verify);
    assert_eq!(kind.semantic_token.as_deref(), Some("function"));
    assert_eq!(kind.lsp_icon.as_deref(), Some("Method"));
    assert_eq!(kind.dot_shape.as_deref(), Some("ellipse"));
    assert_eq!(kind.allowed_verify_kinds, vec!["smoke"]);
}

#[test]
fn convert_entity_kind_with_keyword_override() {
    let ext = minimal_protocol_extension(
        "@test/ext",
        vec![EntityKindDescriptor {
            name: "Behavior".to_string(),
            keyword: Some("behavior".to_string()),
            description: Some("A testable unit".to_string()),
            fields: vec![],
            testable: false,
            singleton: true,
            supports_verify: false,
            incremental: Some(true),
            has_body_parser: true,
            open_fields: true,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: Some("blue".to_string()),
            dot_fillcolor: Some("lightblue".to_string()),
            verify_kinds: vec![],
        }],
    );

    let manifest = protocol_extension_to_manifest(&ext);
    let kind = &manifest.entity_kinds[0];
    assert_eq!(kind.name, "Behavior");
    assert_eq!(kind.keyword, "behavior");
    assert_eq!(kind.description.as_deref(), Some("A testable unit"));
    assert!(kind.singleton);
    assert!(kind.has_body_parser);
    assert!(kind.open_fields);
    assert_eq!(kind.incremental, Some(true));
    assert_eq!(kind.dot_color.as_deref(), Some("blue"));
    assert_eq!(kind.dot_fillcolor.as_deref(), Some("lightblue"));
}

// ── Step 2: Entity kind with fields + populate_from_protocol ──

#[test]
fn convert_entity_kind_fields_to_manifest_fields() {
    let ext = minimal_protocol_extension(
        "@specforge/software",
        vec![EntityKindDescriptor {
            name: "behavior".to_string(),
            keyword: None,
            description: None,
            fields: vec![
                FieldDescriptor {
                    name: "contract".to_string(),
                    field_type: "block".to_string(),
                    required: false,
                    description: Some("The behavioral contract".to_string()),
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    default_value: None,
                    enum_values: vec![],
                    inverse_of: None,
                },
                FieldDescriptor {
                    name: "invariants".to_string(),
                    field_type: "reference_list".to_string(),
                    required: false,
                    description: None,
                    edge: Some("enforces".to_string()),
                    target_kind: Some("invariant".to_string()),
                    file_reference: false,
                    default_value: None,
                    enum_values: vec![],
                    inverse_of: None,
                },
            ],
            testable: true,
            singleton: false,
            supports_verify: false,
            incremental: None,
            has_body_parser: false,
            open_fields: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            verify_kinds: vec![],
        }],
    );

    let manifest = protocol_extension_to_manifest(&ext);
    let kind = &manifest.entity_kinds[0];
    assert_eq!(kind.fields.len(), 2);

    let contract = &kind.fields[0];
    assert_eq!(contract.name, "contract");
    assert_eq!(contract.field_type, "block");
    assert_eq!(contract.description.as_deref(), Some("The behavioral contract"));
    assert!(contract.edge.is_none());

    let invariants = &kind.fields[1];
    assert_eq!(invariants.name, "invariants");
    assert_eq!(invariants.field_type, "reference_list");
    assert_eq!(invariants.edge.as_deref(), Some("enforces"));
    assert_eq!(invariants.target_kind.as_deref(), Some("invariant"));
}

#[test]
fn populate_from_protocol_registers_kind_and_fields() {
    let ext = minimal_protocol_extension(
        "@specforge/software",
        vec![EntityKindDescriptor {
            name: "behavior".to_string(),
            keyword: None,
            description: None,
            fields: vec![
                FieldDescriptor {
                    name: "contract".to_string(),
                    field_type: "block".to_string(),
                    required: false,
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    default_value: None,
                    enum_values: vec![],
                    inverse_of: None,
                },
            ],
            testable: true,
            singleton: false,
            supports_verify: false,
            incremental: None,
            has_body_parser: false,
            open_fields: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            verify_kinds: vec![],
        }],
    );

    let (kind_reg, field_reg, _, diags) = populate_from_protocol(&[ext]);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    assert!(kind_reg.contains("behavior"));
    let entry = kind_reg.get("behavior").unwrap();
    assert!(entry.testable);
    assert_eq!(entry.source_extension, "@specforge/software");

    assert!(field_reg.contains("behavior", "contract"));
    let field = field_reg.get("behavior", "contract").unwrap();
    assert_eq!(field.field_type, ManifestFieldType::Block);
}

// ── Step 3: Edge types ──

#[test]
fn convert_edge_types_to_manifest() {
    let mut ext = minimal_protocol_extension("@specforge/software", vec![]);
    ext.descriptions.edge_types = vec![
        EdgeTypeDescriptor {
            label: "enforces".to_string(),
            description: Some("Behavior enforces an invariant".to_string()),
            source_kind: Some("behavior".to_string()),
            target_kind: Some("invariant".to_string()),
            edge_style: Some("dashed".to_string()),
            edge_color: Some("red".to_string()),
            edge_arrowhead: Some("vee".to_string()),
        },
        EdgeTypeDescriptor {
            label: "composes".to_string(),
            description: None,
            source_kind: None,
            target_kind: None,
            edge_style: None,
            edge_color: None,
            edge_arrowhead: None,
        },
    ];

    let manifest = protocol_extension_to_manifest(&ext);
    assert_eq!(manifest.edge_types.len(), 2);

    let enforces = &manifest.edge_types[0];
    assert_eq!(enforces.label, "enforces");
    assert_eq!(enforces.description.as_deref(), Some("Behavior enforces an invariant"));
    assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
    assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
    assert_eq!(enforces.edge_style.as_deref(), Some("dashed"));
    assert_eq!(enforces.edge_color.as_deref(), Some("red"));
    assert_eq!(enforces.edge_arrowhead.as_deref(), Some("vee"));

    let composes = &manifest.edge_types[1];
    assert_eq!(composes.label, "composes");
    assert!(composes.source_kind.is_none());
}

#[test]
fn populate_from_protocol_registers_edges() {
    let mut ext = minimal_protocol_extension(
        "@specforge/software",
        vec![EntityKindDescriptor {
            name: "behavior".to_string(),
            keyword: None,
            description: None,
            fields: vec![],
            testable: true,
            singleton: false,
            supports_verify: false,
            incremental: None,
            has_body_parser: false,
            open_fields: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            verify_kinds: vec![],
        }],
    );
    ext.descriptions.edge_types = vec![EdgeTypeDescriptor {
        label: "enforces".to_string(),
        description: None,
        source_kind: Some("behavior".to_string()),
        target_kind: Some("invariant".to_string()),
        edge_style: Some("dashed".to_string()),
        edge_color: None,
        edge_arrowhead: None,
    }];

    let (_, _, edge_reg, _) = populate_from_protocol(&[ext]);
    assert!(edge_reg.contains("enforces"));
    let edge = edge_reg.get("enforces").unwrap();
    assert_eq!(edge.source_kind.as_deref(), Some("behavior"));
    assert_eq!(edge.target_kind.as_deref(), Some("invariant"));
    assert_eq!(edge.edge_style.as_deref(), Some("dashed"));
}

// ── Step 4: Validation rules ──

#[test]
fn convert_validation_rules_to_manifest() {
    let mut ext = minimal_protocol_extension("@test/ext", vec![]);
    ext.descriptions.validation_rules = vec![
        ValidationRuleDescriptor {
            code: "W001".to_string(),
            severity: ValidationSeverity::Warning,
            message_template: "entity has no incoming edges".to_string(),
            check: "no_incoming_edges".to_string(),
            target_kind: Some("behavior".to_string()),
            edge_type: None,
            field: None,
            constraint: None,
            wasm_function: None,
        },
        ValidationRuleDescriptor {
            code: "E010".to_string(),
            severity: ValidationSeverity::Error,
            message_template: "invalid field value".to_string(),
            check: "field_value_constraint".to_string(),
            target_kind: None,
            edge_type: None,
            field: Some("status".to_string()),
            constraint: Some(FieldConstraintDescriptor {
                kind: "enum".to_string(),
                pattern: None,
                values: vec!["draft".to_string(), "active".to_string()],
            }),
            wasm_function: Some("validate__e010".to_string()),
        },
        ValidationRuleDescriptor {
            code: "I001".to_string(),
            severity: ValidationSeverity::Info,
            message_template: "informational".to_string(),
            check: "custom".to_string(),
            target_kind: None,
            edge_type: Some("enforces".to_string()),
            field: None,
            constraint: None,
            wasm_function: None,
        },
    ];

    let manifest = protocol_extension_to_manifest(&ext);
    assert_eq!(manifest.validation_rules.len(), 3);

    let w001 = &manifest.validation_rules[0];
    assert_eq!(w001.code, "W001");
    assert_eq!(w001.severity, "warning");
    assert_eq!(w001.check, "no_incoming_edges");
    assert_eq!(w001.target_kind.as_deref(), Some("behavior"));

    let e010 = &manifest.validation_rules[1];
    assert_eq!(e010.code, "E010");
    assert_eq!(e010.severity, "error");
    assert_eq!(e010.field.as_deref(), Some("status"));
    let constraint = e010.constraint.as_ref().unwrap();
    assert_eq!(constraint.kind, "enum");
    assert_eq!(constraint.values, vec!["draft", "active"]);
    assert_eq!(e010.wasm_function.as_deref(), Some("validate__e010"));

    let i001 = &manifest.validation_rules[2];
    assert_eq!(i001.severity, "info");
    assert_eq!(i001.edge_type.as_deref(), Some("enforces"));
}

// ── Step 5: Entity enhancements across extensions ──

#[test]
fn convert_entity_enhancements_to_manifest() {
    let mut ext = minimal_protocol_extension("@specforge/formal", vec![]);
    ext.descriptions.enhancements = vec![EntityEnhancementDescriptor {
        target_kind: "behavior".to_string(),
        source_extension: "@specforge/formal".to_string(),
        fields: vec![FieldDescriptor {
            name: "requires".to_string(),
            field_type: "block".to_string(),
            required: false,
            description: Some("Precondition block".to_string()),
            edge: None,
            target_kind: None,
            file_reference: false,
            default_value: None,
            enum_values: vec![],
            inverse_of: None,
        }],
        edge_types: vec![],
    }];

    let manifest = protocol_extension_to_manifest(&ext);
    assert_eq!(manifest.entity_enhancements.len(), 1);
    let enh = &manifest.entity_enhancements[0];
    assert_eq!(enh.target_kind, "behavior");
    assert_eq!(enh.source_extension, "@specforge/formal");
    assert_eq!(enh.fields.len(), 1);
    assert_eq!(enh.fields[0].name, "requires");
    assert_eq!(enh.fields[0].field_type, "block");
}

#[test]
fn populate_from_protocol_applies_enhancements_across_extensions() {
    // Extension 1: declares "behavior" entity kind
    let software = minimal_protocol_extension(
        "@specforge/software",
        vec![EntityKindDescriptor {
            name: "behavior".to_string(),
            keyword: None,
            description: None,
            fields: vec![FieldDescriptor {
                name: "contract".to_string(),
                field_type: "block".to_string(),
                required: false,
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                default_value: None,
                enum_values: vec![],
                inverse_of: None,
            }],
            testable: true,
            singleton: false,
            supports_verify: false,
            incremental: None,
            has_body_parser: false,
            open_fields: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            verify_kinds: vec![],
        }],
    );

    // Extension 2: enhances "behavior" with "requires" field
    let mut formal = minimal_protocol_extension("@specforge/formal", vec![]);
    formal.descriptions.enhancements = vec![EntityEnhancementDescriptor {
        target_kind: "behavior".to_string(),
        source_extension: "@specforge/formal".to_string(),
        fields: vec![FieldDescriptor {
            name: "requires".to_string(),
            field_type: "block".to_string(),
            required: false,
            description: None,
            edge: None,
            target_kind: None,
            file_reference: false,
            default_value: None,
            enum_values: vec![],
            inverse_of: None,
        }],
        edge_types: vec![],
    }];

    let (kind_reg, field_reg, _, diags) = populate_from_protocol(&[software, formal]);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);

    // behavior kind registered
    assert!(kind_reg.contains("behavior"));

    // Original field preserved
    assert!(field_reg.contains("behavior", "contract"));

    // Enhancement field merged
    assert!(field_reg.contains("behavior", "requires"));
    let requires = field_reg.get("behavior", "requires").unwrap();
    assert_eq!(requires.field_type, ManifestFieldType::Block);
}

// ── Step 6: Shared fields + metadata ──

#[test]
fn convert_shared_fields_to_manifest() {
    let mut ext = minimal_protocol_extension("@test/ext", vec![]);
    ext.descriptions.shared_fields = vec![FieldDescriptor {
        name: "status".to_string(),
        field_type: "string".to_string(),
        required: false,
        description: Some("Entity status".to_string()),
        edge: None,
        target_kind: None,
        file_reference: false,
        default_value: Some("draft".to_string()),
        enum_values: vec!["draft".to_string(), "active".to_string()],
        inverse_of: None,
    }];

    let manifest = protocol_extension_to_manifest(&ext);
    assert_eq!(manifest.fields.len(), 1);
    let field = &manifest.fields[0];
    assert_eq!(field.name, "status");
    assert_eq!(field.field_type, "string");
    assert_eq!(field.description.as_deref(), Some("Entity status"));
}

#[test]
fn convert_metadata_peer_deps_sandbox_flags() {
    let ext = ProtocolExtension {
        name: "@specforge/formal".to_string(),
        version: "2.0.0".to_string(),
        handshake: HandshakeResponse {
            protocol_version: "1.0".to_string(),
            name: "@specforge/formal".to_string(),
            version: "2.0.0".to_string(),
            contribution_flags: ContributionFlags {
                entities: true,
                validators: true,
                grammars: true,
                body_parsers: false,
                collectors: false,
                renderers: false,
                providers: false,
                prompts: false,
                parsers: false,
            },
            peer_dependencies: vec![
                PeerDependency {
                    name: "@specforge/software".to_string(),
                    version: ">=1.0.0".to_string(),
                    optional: false,
                },
                PeerDependency {
                    name: "@specforge/governance".to_string(),
                    version: ">=1.0.0".to_string(),
                    optional: true,
                },
            ],
            sandbox_policy: Some(SandboxPolicy {
                max_memory_mb: Some(256),
                max_execution_ms: Some(5000),
                allowed_domains: vec!["example.com".to_string()],
                allowed_paths: vec!["/tmp".to_string()],
                allowed_output_extensions: vec!["json".to_string()],
                network_access: Some(false),
                file_system_access: Some(true),
            }),
        },
        descriptions: ExtensionDescriptions::default(),
    };

    let manifest = protocol_extension_to_manifest(&ext);

    // Contribution flags
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);
    assert!(manifest.contributes.grammars);
    assert!(!manifest.contributes.collectors);

    // Peer dependencies
    assert_eq!(manifest.peer_dependencies.len(), 2);
    assert_eq!(manifest.peer_dependencies[0].name, "@specforge/software");
    assert_eq!(manifest.peer_dependencies[0].version, ">=1.0.0");
    assert!(!manifest.peer_dependencies[0].optional);
    assert!(manifest.peer_dependencies[1].optional);

    // Sandbox policy
    let sandbox = manifest.sandbox_policy.as_ref().unwrap();
    assert_eq!(sandbox.max_memory_mb, Some(256));
    assert_eq!(sandbox.max_execution_ms, Some(5000));
    assert_eq!(sandbox.allowed_domains, vec!["example.com"]);
    assert_eq!(sandbox.network_access, Some(false));
    assert_eq!(sandbox.file_system_access, Some(true));
}

// ── Step 7: Grammar, body parser, collector ──

#[test]
fn convert_grammar_body_parser_collector() {
    let mut ext = minimal_protocol_extension("@test/ext", vec![]);
    ext.descriptions.grammars = vec![GrammarDescriptor {
        entity_kind: "behavior".to_string(),
        grammar_wasm_path: "grammar.wasm".to_string(),
        export_name: Some("parse_behavior".to_string()),
    }];
    ext.descriptions.body_parsers = vec![BodyParserDescriptor {
        entity_kind: "behavior".to_string(),
        export_name: "parse__behavior".to_string(),
    }];
    ext.descriptions.collectors = vec![CollectorDescriptor {
        name: "rust".to_string(),
        input_formats: vec!["junit-xml".to_string()],
        export: "collect__rust".to_string(),
        auto_detect: Some(AutoDetectConfig {
            file_patterns: vec!["**/target/**/junit.xml".to_string()],
            env_vars: vec!["CARGO_TARGET_DIR".to_string()],
        }),
    }];

    let manifest = protocol_extension_to_manifest(&ext);

    // Grammars
    assert_eq!(manifest.grammar_contributions.len(), 1);
    let gc = &manifest.grammar_contributions[0];
    assert_eq!(gc.entity_kind, "behavior");
    assert_eq!(gc.grammar_wasm_path, "grammar.wasm");
    assert_eq!(gc.export_name.as_deref(), Some("parse_behavior"));

    // Body parsers
    assert_eq!(manifest.body_parser_contributions.len(), 1);
    let bp = &manifest.body_parser_contributions[0];
    assert_eq!(bp.entity_kind, "behavior");
    assert_eq!(bp.export_name, "parse__behavior");

    // Collectors
    assert_eq!(manifest.collector_contributions.len(), 1);
    let cc = &manifest.collector_contributions[0];
    assert_eq!(cc.name, "rust");
    assert_eq!(cc.input_formats, vec!["junit-xml"]);
    assert_eq!(cc.export, "collect__rust");
    let ad = cc.auto_detect.as_ref().unwrap();
    assert_eq!(ad.file_patterns, vec!["**/target/**/junit.xml"]);
    assert_eq!(ad.env_vars, vec!["CARGO_TARGET_DIR"]);
}

#[test]
fn convert_collector_without_auto_detect() {
    let mut ext = minimal_protocol_extension("@test/ext", vec![]);
    ext.descriptions.collectors = vec![CollectorDescriptor {
        name: "jest".to_string(),
        input_formats: vec!["jest-json".to_string()],
        export: "collect__jest".to_string(),
        auto_detect: None,
    }];

    let manifest = protocol_extension_to_manifest(&ext);
    assert!(manifest.collector_contributions[0].auto_detect.is_none());
}

// ── Step 8: Surfaces + parity test ──

#[test]
fn convert_surfaces_to_manifest() {
    let mut ext = minimal_protocol_extension("@test/ext", vec![]);
    ext.descriptions.surfaces = Some(SurfaceDescriptor {
        commands: vec![CommandDescriptor {
            id: "check".to_string(),
            title: "Check".to_string(),
            description: "Run validation".to_string(),
            category: Some("validation".to_string()),
            export: "cmd__check".to_string(),
            args: vec![CommandArgDescriptor {
                name: "strict".to_string(),
                arg_type: CommandArgType::Bool,
                required: false,
                default_value: Some("false".to_string()),
                description: Some("Enable strict mode".to_string()),
            }],
            sandbox: None,
        }],
        mcp_tools: vec![McpToolDescriptor {
            name: "specforge_query".to_string(),
            description: "Query the graph".to_string(),
            category: None,
            export: "mcp__query".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
            sandbox: None,
        }],
        mcp_resources: vec![McpResourceDescriptor {
            uri_template: "specforge://entities/{kind}".to_string(),
            name: "entities".to_string(),
            description: Some("List entities".to_string()),
            export: "mcp__entities".to_string(),
            mime_type: "application/json".to_string(),
            sandbox: None,
        }],
    });

    let surfaces_input = protocol_surfaces_to_manifest(&[ext]);
    assert_eq!(surfaces_input.len(), 1);
    let (name, surfaces) = &surfaces_input[0];
    assert_eq!(name, "@test/ext");
    let surfaces = surfaces.as_ref().unwrap();

    assert_eq!(surfaces.commands.len(), 1);
    assert_eq!(surfaces.commands[0].id, "check");
    assert_eq!(surfaces.commands[0].export, "cmd__check");
    assert_eq!(surfaces.commands[0].args.len(), 1);
    assert_eq!(surfaces.commands[0].args[0].name, "strict");

    assert_eq!(surfaces.mcp_tools.len(), 1);
    assert_eq!(surfaces.mcp_tools[0].name, "specforge_query");

    assert_eq!(surfaces.mcp_resources.len(), 1);
    assert_eq!(surfaces.mcp_resources[0].name, "entities");
    assert_eq!(surfaces.mcp_resources[0].mime_type, "application/json");
}

#[test]
fn protocol_surfaces_to_manifest_none_when_no_surfaces() {
    let ext = minimal_protocol_extension("@test/ext", vec![]);
    let surfaces_input = protocol_surfaces_to_manifest(&[ext]);
    assert_eq!(surfaces_input.len(), 1);
    assert!(surfaces_input[0].1.is_none());
}

#[test]
fn parity_protocol_vs_manifest_registries() {
    // Build identical data through BOTH paths and verify registries match.

    // Path 1: Direct ManifestV2
    let manifest = specforge_registry::ManifestV2 {
        name: "@specforge/software".to_string(),
        version: "1.0.0".to_string(),
        manifest_version: 2,
        wasm_path: "software.wasm".to_string(),
        contributes: specforge_registry::ExtensionContributions {
            entities: true,
            validators: true,
            ..Default::default()
        },
        entity_kinds: vec![specforge_registry::ManifestEntityKind {
            name: "behavior".to_string(),
            keyword: "behavior".to_string(),
            description: Some("A testable behavior".to_string()),
            testable: true,
            singleton: false,
            supports_verify: true,
            allowed_verify_kinds: vec!["smoke".to_string()],
            semantic_token: Some("function".to_string()),
            lsp_icon: Some("Method".to_string()),
            dot_shape: Some("ellipse".to_string()),
            dot_color: None,
            dot_fillcolor: None,
            fields: vec![
                specforge_registry::ManifestField {
                    name: "contract".to_string(),
                    field_type: "block".to_string(),
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                    default_value: None,
                    enum_values: vec![],
                    inverse_of: None,
                },
                specforge_registry::ManifestField {
                    name: "invariants".to_string(),
                    field_type: "reference_list".to_string(),
                    description: None,
                    edge: Some("enforces".to_string()),
                    target_kind: Some("invariant".to_string()),
                    file_reference: false,
                    required: false,
                    default_value: None,
                    enum_values: vec![],
                    inverse_of: None,
                },
            ],
            incremental: None,
            has_body_parser: false,
            open_fields: false,
        }],
        edge_types: vec![specforge_registry::ManifestEdgeType {
            label: "enforces".to_string(),
            description: None,
            source_kind: Some("behavior".to_string()),
            target_kind: Some("invariant".to_string()),
            edge_style: Some("dashed".to_string()),
            edge_color: None,
            edge_arrowhead: None,
        }],
        validation_rules: vec![],
        verify_kinds: vec![],
        fields: vec![],
        incremental: None,
        reserved_keywords: vec![],
        migration_hook: None,
        peer_dependencies: vec![],
        sandbox_policy: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        surfaces: None,
    };
    let (m_kind_reg, m_field_reg, m_edge_reg, m_diags) =
        specforge_registry::populate_registries(&[manifest]);

    // Path 2: ProtocolExtension → bridge → populate_registries
    let protocol_ext = ProtocolExtension {
        name: "@specforge/software".to_string(),
        version: "1.0.0".to_string(),
        handshake: HandshakeResponse {
            protocol_version: "1.0".to_string(),
            name: "@specforge/software".to_string(),
            version: "1.0.0".to_string(),
            contribution_flags: ContributionFlags {
                entities: true,
                validators: true,
                ..Default::default()
            },
            peer_dependencies: vec![],
            sandbox_policy: None,
        },
        descriptions: ExtensionDescriptions {
            entity_kinds: vec![EntityKindDescriptor {
                name: "behavior".to_string(),
                keyword: None,
                description: Some("A testable behavior".to_string()),
                fields: vec![
                    FieldDescriptor {
                        name: "contract".to_string(),
                        field_type: "block".to_string(),
                        required: false,
                        description: None,
                        edge: None,
                        target_kind: None,
                        file_reference: false,
                        default_value: None,
                        enum_values: vec![],
                        inverse_of: None,
                    },
                    FieldDescriptor {
                        name: "invariants".to_string(),
                        field_type: "reference_list".to_string(),
                        required: false,
                        description: None,
                        edge: Some("enforces".to_string()),
                        target_kind: Some("invariant".to_string()),
                        file_reference: false,
                        default_value: None,
                        enum_values: vec![],
                        inverse_of: None,
                    },
                ],
                testable: true,
                singleton: false,
                supports_verify: true,
                incremental: None,
                has_body_parser: false,
                open_fields: false,
                semantic_token: Some("function".to_string()),
                lsp_icon: Some("Method".to_string()),
                dot_shape: Some("ellipse".to_string()),
                dot_color: None,
                dot_fillcolor: None,
                verify_kinds: vec!["smoke".to_string()],
            }],
            edge_types: vec![EdgeTypeDescriptor {
                label: "enforces".to_string(),
                description: None,
                source_kind: Some("behavior".to_string()),
                target_kind: Some("invariant".to_string()),
                edge_style: Some("dashed".to_string()),
                edge_color: None,
                edge_arrowhead: None,
            }],
            ..Default::default()
        },
    };
    let (p_kind_reg, p_field_reg, p_edge_reg, p_diags) =
        populate_from_protocol(&[protocol_ext]);

    // Verify parity: both paths produce same diagnostics count
    assert_eq!(m_diags.len(), p_diags.len(),
        "diagnostic count mismatch: manifest={:?}, protocol={:?}", m_diags, p_diags);

    // Same kind registry
    assert_eq!(m_kind_reg.len(), p_kind_reg.len());
    assert!(p_kind_reg.contains("behavior"));
    let m_beh = m_kind_reg.get("behavior").unwrap();
    let p_beh = p_kind_reg.get("behavior").unwrap();
    assert_eq!(m_beh.testable, p_beh.testable);
    assert_eq!(m_beh.source_extension, p_beh.source_extension);
    assert_eq!(m_beh.supports_verify, p_beh.supports_verify);
    assert_eq!(m_beh.semantic_token, p_beh.semantic_token);
    assert_eq!(m_beh.dot_shape, p_beh.dot_shape);

    // Same field registry
    assert!(p_field_reg.contains("behavior", "contract"));
    assert!(p_field_reg.contains("behavior", "invariants"));
    let m_contract = m_field_reg.get("behavior", "contract").unwrap();
    let p_contract = p_field_reg.get("behavior", "contract").unwrap();
    assert_eq!(m_contract.field_type, p_contract.field_type);

    let m_inv = m_field_reg.get("behavior", "invariants").unwrap();
    let p_inv = p_field_reg.get("behavior", "invariants").unwrap();
    assert_eq!(m_inv.field_type, p_inv.field_type);
    assert_eq!(m_inv.edge, p_inv.edge);
    assert_eq!(m_inv.target_kind, p_inv.target_kind);

    // Same edge registry
    assert!(p_edge_reg.contains("enforces"));
    let m_edge = m_edge_reg.get("enforces").unwrap();
    let p_edge = p_edge_reg.get("enforces").unwrap();
    assert_eq!(m_edge.source_kind, p_edge.source_kind);
    assert_eq!(m_edge.target_kind, p_edge.target_kind);
    assert_eq!(m_edge.edge_style, p_edge.edge_style);
    assert_eq!(m_edge.source_extension, p_edge.source_extension);
}
