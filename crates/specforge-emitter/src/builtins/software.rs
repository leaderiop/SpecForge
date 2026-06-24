use specforge_wasm::builtin::BuiltinExtension;
use specforge_wasm::protocol::{
    CompilerPassDescriptor, ContributionFlags, DescribeResponse, EdgeTypeDescriptor,
    EntityEnhancementDescriptor, EntityKindDescriptor, FeatureFlagDescriptor,
    FieldConstraintDescriptor, FieldDescriptor, HandshakeResponse, PeerDependency, SandboxPolicy,
    SharedFieldDescriptor, SurfaceDescriptor, ValidationRuleDescriptor, ValidationSeverity,
};

pub struct SoftwareExtension;

impl SoftwareExtension {
    fn entity_kinds(&self) -> Vec<EntityKindDescriptor> {
        vec![
            EntityKindDescriptor {
                name: "Behavior".into(),
                keyword: Some("behavior".into()),
                description: Some(
                    "A testable unit of system functionality with a defined contract".into(),
                ),
                inference_guide: Some("Look for public functions in service, handler, or controller layers that represent user-visible operations or business logic. Route handlers (HTTP, gRPC, CLI commands), use-case functions, and domain service methods are strong signals. Map function name to entity ID (snake_case). Extract doc comments or the function's purpose as the contract. If the function validates rules or enforces constraints, those are invariants. If it emits events or messages, those are produces. If it has dedicated test files or test functions, add verify statements. Skip: private helpers, utility functions, test fixtures, generated code, framework boilerplate.".into()),
                semantic_token: Some("function".into()),
                lsp_icon: Some("Method".into()),
                dot_shape: Some("box".into()),
                dot_color: Some("#1565C0".into()),
                dot_fillcolor: Some("#E3F2FD".into()),
                fields: vec![
                    fd("contract", "string", true, Some("The behavioral contract this behavior guarantees"), None, None),
                    fd_ref_inv("invariants", "reference_list", "BehaviorEnforcesInvariant", "invariant", "enforced_by", Some("Invariants this behavior enforces")),
                    fd_ref("types", "reference_list", "BehaviorReferencesType", "type", Some("Type definitions used by this behavior")),
                    fd_ref("ports", "reference_list", "BehaviorUsesPort", "port", Some("Port interfaces this behavior interacts with")),
                    fd_ref("produces", "reference_list", "BehaviorProducesEvent", "event", Some("Events produced as a result of this behavior")),
                    fd_ref("consumes", "reference_list", "BehaviorConsumesEvent", "event", Some("Events this behavior reacts to")),
                    fd("category", "string", false, Some("Classification tag for agent task routing"), None, None),
                    fd_ref_inv("features", "reference_list", "BehaviorImplementsFeature", "feature", "behaviors", Some("Product features this behavior implements")),
                    fd("description", "string", false, Some("Human-readable summary of this behavior"), None, None),
                    fd("status", "string", false, Some("Current lifecycle status of this behavior"), None, None),
                    fd("refs", "string_list", false, Some("External references such as issue or document URIs"), None, None),
                    fd("severity", "string", false, Some("Impact level if this behavior fails"), None, None),
                    fd("diagnostic", "string", false, Some("Diagnostic message for validation tooling"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Invariant".into(),
                keyword: Some("invariant".into()),
                description: Some(
                    "A system-wide constraint that must always hold true".into(),
                ),
                inference_guide: Some("Look for assertions, validation logic, and defensive checks that enforce system-wide rules. Signals: assert!() / assert_eq!() statements, guard clauses that panic or return errors, database constraints (UNIQUE, CHECK, NOT NULL), middleware that rejects invalid state, config validation at startup, and comments like 'must always', 'never allow', 'invariant'. The guarantee field should state what must hold (e.g., 'User email must be unique across all accounts'). The risk field describes consequences of violation. Invariants are architectural rules about state; for quantified limits (latency <200ms, max connections), use constraint instead. Skip: local variable checks, input validation that's behavior-specific, temporary debug assertions.".into()),
                semantic_token: Some("property".into()),
                lsp_icon: Some("Property".into()),
                dot_shape: Some("diamond".into()),
                dot_color: Some("#C62828".into()),
                dot_fillcolor: Some("#FFEBEE".into()),
                fields: vec![
                    fd("guarantee", "string", true, Some("The constraint this invariant guarantees holds at all times"), None, None),
                    fd("risk", "string", false, Some("Consequence or impact if this invariant is violated"), None, None),
                    fd("description", "string", false, Some("Human-readable summary of this invariant"), None, None),
                    fd("refs", "string_list", false, Some("External references such as issue or document URIs"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Event".into(),
                keyword: Some("event".into()),
                description: Some(
                    "A significant occurrence in the system that triggers reactions".into(),
                ),
                inference_guide: Some("Look for message types, event classes, pub/sub topics, webhook payloads, and signal/notification patterns. Signals: structs/classes named *Event, *Message, *Notification; message queue topic/channel definitions; emit()/publish()/dispatch()/notify() calls; event handler registrations (on_*, handle_*); webhook payload schemas. Map the event name to entity ID. If it carries structured data, reference the payload type. Identify which behaviors produce and consume each event. Skip: internal method calls, logging statements, framework lifecycle callbacks (unless domain-meaningful).".into()),
                semantic_token: Some("event".into()),
                lsp_icon: Some("Event".into()),
                dot_shape: Some("ellipse".into()),
                dot_color: Some("#E65100".into()),
                dot_fillcolor: Some("#FFF3E0".into()),
                fields: vec![
                    fd("channel", "string", false, Some("Communication channel this event is published on"), None, None),
                    fd("channel_type", "string", false, Some("Transport mechanism for the channel (e.g. queue, topic, stream)"), None, None),
                    fd("category", "string", false, Some("Classification of this event (e.g. domain, integration, system)"), None, None),
                    fd_ref("payload", "reference", "EventCarriesPayloadType", "type", Some("Type definition describing this event's data shape")),
                    fd("description", "string", false, Some("Human-readable summary of this event"), None, None),
                    fd("refs", "string_list", false, Some("External references such as issue or document URIs"), None, None),
                    fd("contract", "string", false, Some("Delivery or ordering guarantees for this event"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Type".into(),
                keyword: Some("type".into()),
                description: Some("A data structure or domain model definition".into()),
                inference_guide: Some("Look for domain model structs, data transfer objects, API request/response shapes, database entities, and enum definitions that carry business meaning. Signals: struct/class definitions in models/ or domain/ directories; TypeScript interfaces/types for API contracts; protobuf/GraphQL/JSON Schema type definitions; ORM model classes; enum types with business variants. Use open_fields to list the type's fields. Set kind field to 'struct', 'enum', 'alias', or 'opaque'. Reference composed_types for nested or referenced types. Skip: internal implementation structs, builder patterns, framework-generated types, test fixtures.".into()),
                semantic_token: Some("type".into()),
                lsp_icon: Some("Struct".into()),
                dot_shape: Some("rectangle".into()),
                dot_color: Some("#2E7D32".into()),
                dot_fillcolor: Some("#E8F5E9".into()),
                open_fields: true,
                fields: vec![
                    fd("kind", "string", false, Some("The type category (e.g. struct, enum, alias, opaque)"), None, None),
                    fd("fields", "block", false, Some("Structured field definitions for this type"), None, None),
                    fd_ref("composed_types", "reference_list", "TypeComposesType", "type", Some("Types composed or referenced by this type")),
                    fd_ref("extends", "reference", "TypeExtendsType", "type", Some("Parent type this type extends or inherits from")),
                    fd("description", "string", false, Some("Human-readable summary of this type"), None, None),
                    fd("refs", "string_list", false, Some("External references such as issue or document URIs"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Port".into(),
                keyword: Some("port".into()),
                description: Some(
                    "An interface boundary between system components".into(),
                ),
                inference_guide: Some("Look for trait definitions, abstract classes, interface declarations, and adapter patterns that define boundaries between system layers. Signals: Rust traits in ports/ or interfaces/ directories; TypeScript/Java interfaces for repositories, gateways, or external services; abstract base classes; dependency injection interfaces; API client contracts. Set direction to 'inbound' (receives requests), 'outbound' (calls external systems), or 'bidirectional'. Use open_fields/methods to list the port's operations. Set category to 'http', 'grpc', 'database', 'queue', 'filesystem', etc. Skip: internal module boundaries, utility traits (Display, Debug), marker traits, framework-imposed interfaces.".into()),
                semantic_token: Some("interface".into()),
                lsp_icon: Some("Interface".into()),
                dot_shape: Some("trapezium".into()),
                dot_color: Some("#00695C".into()),
                dot_fillcolor: Some("#E0F2F1".into()),
                open_fields: true,
                fields: vec![
                    fd("direction", "string", true, Some("Whether this port is inbound, outbound, or bidirectional"), None, None),
                    fd("category", "string", false, Some("Classification of this port (e.g. http, grpc, database, queue)"), None, None),
                    fd("methods", "block", false, Some("Method signatures exposed by this port interface"), None, None),
                    fd("description", "string", false, Some("Human-readable summary of this port"), None, None),
                    fd("refs", "string_list", false, Some("External references such as issue or document URIs"), None, None),
                ],
                ..ekd_defaults()
            },
        ]
    }

    fn edge_types(&self) -> Vec<EdgeTypeDescriptor> {
        vec![
            // References — no source/target kind constraints
            EdgeTypeDescriptor {
                label: "References".into(),
                description: Some("General cross-reference between entities".into()),
                source_kind: None,
                target_kind: None,
                edge_style: None,
                edge_color: None,
                edge_arrowhead: None,
            },
            edge_desc("BehaviorImplementsFeature", "behavior", "feature", "solid", "#1565C0", "Behavior implements a feature (cross-extension via peer_dependency @specforge/product)"),
            edge("BehaviorProducesEvent", "behavior", "event", "solid", "#E65100", None),
            edge("BehaviorConsumesEvent", "behavior", "event", "dashed", "#E65100", None),
            edge("BehaviorReferencesType", "behavior", "type", "solid", "#2E7D32", None),
            edge("EventCarriesPayloadType", "event", "type", "solid", "#2E7D32", None),
            edge("TypeComposesType", "type", "type", "solid", "#2E7D32", None),
            edge("BehaviorUsesPort", "behavior", "port", "solid", "#00695C", None),
            edge("BehaviorEnforcesInvariant", "behavior", "invariant", "dashed", "#C62828", None),
            // TypeExtendsType — has edgeArrowhead "empty"
            edge("TypeExtendsType", "type", "type", "solid", "#2E7D32", Some("empty")),
            // ExternalRef — no source/target kind constraints
            EdgeTypeDescriptor {
                label: "ExternalRef".into(),
                description: Some("Entity references an external URI or resource".into()),
                source_kind: None,
                target_kind: None,
                edge_style: Some("dotted".into()),
                edge_color: Some("#9E9E9E".into()),
                edge_arrowhead: None,
            },
            edge_desc("MilestoneIncludesBehavior", "milestone", "behavior", "solid", "#9C27B0", "Milestone delivers a behavior (cross-extension enhancement edge)"),
            edge("ModuleConsumesPort", "module", "port", "dashed", "#00695C", None),
            edge("ModuleDefinesPort", "module", "port", "solid", "#00695C", None),
        ]
    }

    fn enhancements(&self) -> Vec<EntityEnhancementDescriptor> {
        vec![
            EntityEnhancementDescriptor {
                target_kind: "module".into(),
                source_extension: "@specforge/product".into(),
                fields: vec![
                    fd_ref("ports", "reference_list", "ModuleConsumesPort", "port", Some("Port interfaces this module consumes")),
                    fd_ref("ports_defined", "reference_list", "ModuleDefinesPort", "port", Some("Port interfaces this module defines")),
                ],
                edge_types: vec![],
            },
            EntityEnhancementDescriptor {
                target_kind: "milestone".into(),
                source_extension: "@specforge/product".into(),
                fields: vec![
                    fd_ref_inv("behaviors", "reference_list", "MilestoneIncludesBehavior", "behavior", "features", Some("Behaviors this milestone includes in its delivery scope")),
                ],
                edge_types: vec![],
            },
        ]
    }

    fn validation_rules(&self) -> Vec<ValidationRuleDescriptor> {
        vec![
            // W001: behavior has no outgoing BehaviorImplementsFeature edges
            ValidationRuleDescriptor {
                code: "W001".into(),
                severity: ValidationSeverity::Warning,
                message_template: "behavior '{id}' does not implement any feature".into(),
                check: "no_outgoing_edges".into(),
                target_kind: Some("behavior".into()),
                edge_type: Some("BehaviorImplementsFeature".into()),
                ..vrd_defaults()
            },
            // W002: type has no incoming edges
            no_incoming("W002", "type", "type '{id}' is not referenced by any behavior, port, or type"),
            // W003: invariant has no incoming edges
            no_incoming("W003", "invariant", "invariant '{id}' is not enforced by any behavior"),
            // W005: port has no incoming edges
            no_incoming("W005", "port", "port '{id}' is not referenced by any behavior"),
            // W006: behavior missing category field
            ValidationRuleDescriptor {
                code: "W006".into(),
                severity: ValidationSeverity::Warning,
                message_template: "behavior '{id}' has no category \u{2014} agents use category for task routing".into(),
                check: "missing_field_when_flag_set".into(),
                target_kind: Some("behavior".into()),
                field: Some("category".into()),
                ..vrd_defaults()
            },
            // W007: event has no incoming edges
            no_incoming("W007", "event", "event '{id}' is not produced by any behavior"),
            // W008: feature has no incoming BehaviorImplementsFeature edges
            ValidationRuleDescriptor {
                code: "W008".into(),
                severity: ValidationSeverity::Warning,
                message_template: "feature '{id}' is not implemented by any behavior".into(),
                check: "no_incoming_edges".into(),
                target_kind: Some("feature".into()),
                edge_type: Some("BehaviorImplementsFeature".into()),
                ..vrd_defaults()
            },
            // W010: custom — type field annotations
            ValidationRuleDescriptor {
                code: "W010".into(),
                severity: ValidationSeverity::Warning,
                message_template: "type '{id}' field '{field}' has unknown annotation '{value}'".into(),
                check: "custom".into(),
                target_kind: Some("type".into()),
                wasm_function: Some("validate__type_field_annotations".into()),
                ..vrd_defaults()
            },
            // E004: custom — port method type references
            ValidationRuleDescriptor {
                code: "E004".into(),
                severity: ValidationSeverity::Error,
                message_template: "port '{id}' method '{field}' references unknown type '{value}'".into(),
                check: "custom".into(),
                target_kind: Some("port".into()),
                wasm_function: Some("validate__port_methods".into()),
                ..vrd_defaults()
            },
            // E006: custom — event triggers
            ValidationRuleDescriptor {
                code: "E006".into(),
                severity: ValidationSeverity::Error,
                message_template: "event '{id}' trigger must reference a behavior, found {kind} '{value}'".into(),
                check: "custom".into(),
                target_kind: Some("event".into()),
                wasm_function: Some("validate__event_triggers".into()),
                ..vrd_defaults()
            },
            // E010: custom — milestone behavior ranges
            ValidationRuleDescriptor {
                code: "E010".into(),
                severity: ValidationSeverity::Error,
                message_template: "milestone '{id}' behaviors range is invalid: {reason}".into(),
                check: "custom".into(),
                target_kind: Some("milestone".into()),
                wasm_function: Some("validate__milestone_behavior_ranges".into()),
                ..vrd_defaults()
            },
            // W011: type kind field value constraint
            fvc(
                "W011",
                "type",
                "kind",
                &["struct", "enum", "alias", "opaque"],
                "type '{id}' has invalid kind '{value}' \u{2014} expected one of: struct, enum, alias, opaque",
            ),
        ]
    }
}

impl BuiltinExtension for SoftwareExtension {
    fn handshake(&self) -> HandshakeResponse {
        HandshakeResponse {
            protocol_version: "1.0.0".into(),
            name: "@specforge/software".into(),
            version: "1.0.0".into(),
            contribution_flags: ContributionFlags {
                entities: true,
                validators: true,
                ..Default::default()
            },
            peer_dependencies: vec![PeerDependency {
                name: "@specforge/product".into(),
                version: "^1.0".into(),
                optional: false,
            }],
            sandbox_policy: Some(SandboxPolicy {
                network_access: Some(false),
                file_system_access: Some(false),
                max_memory_mb: Some(256),
                max_execution_ms: Some(5000),
                ..Default::default()
            }),
        }
    }

    fn describe(&self, category: &str) -> Option<DescribeResponse> {
        let items = match category {
            "entities" => serde_json::to_value(self.entity_kinds()).unwrap(),
            "edges" => serde_json::to_value(self.edge_types()).unwrap(),
            "fields" => serde_json::to_value(Vec::<FieldDescriptor>::new()).unwrap(),
            "shared_fields" => serde_json::to_value(Vec::<SharedFieldDescriptor>::new()).unwrap(),
            "enhancements" => serde_json::to_value(self.enhancements()).unwrap(),
            "validation_rules" => serde_json::to_value(self.validation_rules()).unwrap(),
            "surfaces" => serde_json::to_value(Vec::<SurfaceDescriptor>::new()).unwrap(),
            "passes" => serde_json::to_value(Vec::<CompilerPassDescriptor>::new()).unwrap(),
            "feature_flags" => serde_json::to_value(Vec::<FeatureFlagDescriptor>::new()).unwrap(),
            _ => return None,
        };
        Some(DescribeResponse {
            category: category.into(),
            items,
        })
    }
}

// ── Helper functions ──

fn fd_defaults() -> FieldDescriptor {
    FieldDescriptor {
        name: String::new(),
        field_type: String::new(),
        required: false,
        description: None,
        edge: None,
        target_kind: None,
        file_reference: false,
        default_value: None,
        enum_values: vec![],
        inverse_of: None,
    }
}

fn fd(
    name: &str,
    ft: &str,
    required: bool,
    desc: Option<&str>,
    edge: Option<&str>,
    target: Option<&str>,
) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        field_type: ft.into(),
        required,
        description: desc.map(|s| s.into()),
        edge: edge.map(|s| s.into()),
        target_kind: target.map(|s| s.into()),
        ..fd_defaults()
    }
}

fn fd_ref(
    name: &str,
    ft: &str,
    edge: &str,
    target: &str,
    desc: Option<&str>,
) -> FieldDescriptor {
    fd(name, ft, false, desc, Some(edge), Some(target))
}

fn fd_ref_inv(
    name: &str,
    ft: &str,
    edge: &str,
    target: &str,
    inverse: &str,
    desc: Option<&str>,
) -> FieldDescriptor {
    let mut f = fd(name, ft, false, desc, Some(edge), Some(target));
    f.inverse_of = Some(inverse.into());
    f
}

fn ekd_defaults() -> EntityKindDescriptor {
    EntityKindDescriptor {
        name: String::new(),
        keyword: None,
        description: None,
        fields: vec![],
        testable: true,
        singleton: false,
        supports_verify: true,
        incremental: None,
        has_body_parser: false,
        open_fields: false,
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        verify_kinds: vec![],
        inference_guide: None,
    }
}

fn edge(
    label: &str,
    src: &str,
    tgt: &str,
    style: &str,
    color: &str,
    arrowhead: Option<&str>,
) -> EdgeTypeDescriptor {
    EdgeTypeDescriptor {
        label: label.into(),
        description: None,
        source_kind: Some(src.into()),
        target_kind: Some(tgt.into()),
        edge_style: Some(style.into()),
        edge_color: Some(color.into()),
        edge_arrowhead: arrowhead.map(|s| s.into()),
    }
}

fn edge_desc(
    label: &str,
    src: &str,
    tgt: &str,
    style: &str,
    color: &str,
    desc: &str,
) -> EdgeTypeDescriptor {
    EdgeTypeDescriptor {
        description: Some(desc.into()),
        ..edge(label, src, tgt, style, color, None)
    }
}

fn vrd_defaults() -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: String::new(),
        severity: ValidationSeverity::Warning,
        message_template: String::new(),
        check: String::new(),
        target_kind: None,
        edge_type: None,
        field: None,
        constraint: None,
        wasm_function: None,
    }
}

fn fvc(
    code: &str,
    target: &str,
    field: &str,
    values: &[&str],
    msg: &str,
) -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: code.into(),
        severity: ValidationSeverity::Warning,
        message_template: msg.into(),
        check: "field_value_constraint".into(),
        target_kind: Some(target.into()),
        field: Some(field.into()),
        constraint: Some(FieldConstraintDescriptor {
            kind: "one_of".into(),
            pattern: None,
            values: values.iter().map(|s| (*s).into()).collect(),
        }),
        ..vrd_defaults()
    }
}

fn no_incoming(code: &str, target: &str, msg: &str) -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: code.into(),
        severity: ValidationSeverity::Warning,
        message_template: msg.into(),
        check: "no_incoming_edges".into(),
        target_kind: Some(target.into()),
        ..vrd_defaults()
    }
}
