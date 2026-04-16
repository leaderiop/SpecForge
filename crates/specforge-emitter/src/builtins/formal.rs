use specforge_wasm::builtin::BuiltinExtension;
use specforge_wasm::protocol::{
    CompilerPassDescriptor, ContributionFlags, DescribeResponse, EdgeTypeDescriptor,
    EntityEnhancementDescriptor, EntityKindDescriptor, FeatureFlagDescriptor,
    FieldConstraintDescriptor, FieldDescriptor, HandshakeResponse, PeerDependency,
    SharedFieldDescriptor, SurfaceDescriptor, ValidationRuleDescriptor, ValidationSeverity,
};

pub struct FormalExtension;

impl FormalExtension {
    fn entity_kinds(&self) -> Vec<EntityKindDescriptor> {
        vec![
            EntityKindDescriptor {
                name: "Property".into(),
                keyword: Some("property".into()),
                description: Some("A temporal property assertion (safety, liveness, fairness)".into()),
                semantic_token: Some("property".into()),
                lsp_icon: Some("Property".into()),
                dot_shape: Some("hexagon".into()),
                dot_color: Some("#0D47A1".into()),
                dot_fillcolor: Some("#E3F2FD".into()),
                fields: vec![
                    fd("expression", "string", true, Some("Formal expression defining the temporal property"), None, None),
                    fd("property_type", "string", false, Some("Category of temporal property (safety, liveness, or fairness)"), None, None),
                    fd("scope", "string", false, Some("Applicability scope of this property"), None, None),
                    fd("description", "string", false, Some("Free-form description of the property"), None, None),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Axiom".into(),
                keyword: Some("axiom".into()),
                description: Some("An assumed-true foundational assertion".into()),
                semantic_token: Some("constant".into()),
                lsp_icon: Some("Constant".into()),
                dot_shape: Some("ellipse".into()),
                dot_color: Some("#311B92".into()),
                dot_fillcolor: Some("#EDE7F6".into()),
                fields: vec![
                    fd("expression", "string", true, Some("Formal expression stating the axiom"), None, None),
                    fd_ref("assumes", "reference_list", "AxiomAssumesInvariant", "invariant", Some("Invariants this axiom assumes as foundational truths")),
                    fd("justification", "string", false, Some("Rationale for why this axiom is assumed true"), None, None),
                    fd("description", "string", false, Some("Free-form description of the axiom"), None, None),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Protocol".into(),
                keyword: Some("protocol".into()),
                description: Some("A shared synchronization contract between processes".into()),
                semantic_token: Some("interface".into()),
                lsp_icon: Some("Interface".into()),
                dot_shape: Some("component".into()),
                dot_color: Some("#004D40".into()),
                dot_fillcolor: Some("#E0F2F1".into()),
                fields: vec![
                    fd("alphabet", "string_list", false, Some("Set of events that this protocol can communicate"), None, None),
                    fd("states", "string_list", false, Some("Possible states in the protocol state machine"), None, None),
                    fd("transitions", "string_list", false, Some("State transitions triggered by events (from -> event -> to)"), None, None),
                    fd("initial_state", "string", false, Some("Starting state of the protocol"), None, None),
                    fd("description", "string", false, Some("Free-form description of the protocol"), None, None),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Refinement".into(),
                keyword: Some("refinement".into()),
                description: Some("A mapping from abstract to concrete specification".into()),
                semantic_token: Some("function".into()),
                lsp_icon: Some("Method".into()),
                dot_shape: Some("parallelogram".into()),
                dot_color: Some("#1B5E20".into()),
                dot_fillcolor: Some("#E8F5E9".into()),
                fields: vec![
                    fd_ref("abstract_entity", "reference", "RefinementRefinesAbstract", "behavior", Some("Reference to the abstract behavior being refined")),
                    fd_ref("concrete_entity", "reference", "RefinementRefinesConcrete", "behavior", Some("Reference to the concrete behavior that implements the refinement")),
                    fd("invariant_deltas", "string_list", false, Some("Changes to invariants introduced or relaxed by this refinement"), None, None),
                    fd_ref("chains_to", "reference", "RefinementChainsToRefinement", "refinement", Some("Next refinement in the refinement chain")),
                    fd("description", "string", false, Some("Free-form description of the refinement mapping"), None, None),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Process".into(),
                keyword: Some("process".into()),
                description: Some("A CSP-style communicating process with states and transitions".into()),
                semantic_token: Some("function".into()),
                lsp_icon: Some("Method".into()),
                dot_shape: Some("ellipse".into()),
                dot_color: Some("#006064".into()),
                dot_fillcolor: Some("#E0F7FA".into()),
                fields: vec![
                    fd("alphabet", "string_list", false, Some("Set of events that this process can communicate"), None, None),
                    fd("states", "string_list", false, Some("Possible states in the process state machine"), None, None),
                    fd("initial_state", "string", false, Some("Starting state of the process"), None, None),
                    fd("composition", "string", false, Some("Composition operator and sub-processes (e.g. parallel, sequential, choice)"), None, None),
                    fd_ref("sub_processes", "reference_list", "ProcessComposesProcess", "process", Some("Sub-processes composed by this process")),
                    fd("description", "string", false, Some("Free-form description of the process"), None, None),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                ],
                ..ekd_defaults()
            },
        ]
    }

    fn edge_types(&self) -> Vec<EdgeTypeDescriptor> {
        vec![
            edge_desc("BehaviorRequiresInvariant", "behavior", "invariant", "solid", "#1A237E", "Behavior requires an invariant as precondition"),
            edge_desc("BehaviorEnsuresInvariant", "behavior", "invariant", "solid", "#1A237E", "Behavior ensures an invariant as postcondition"),
            edge_desc("BehaviorMaintainsInvariant", "behavior", "invariant", "dashed", "#1A237E", "Behavior maintains an invariant as frame invariant throughout execution"),
            edge_desc("AxiomAssumesInvariant", "axiom", "invariant", "dotted", "#311B92", "Axiom assumes an invariant as foundational truth"),
            edge_desc("BehaviorSatisfiesProperty", "behavior", "property", "solid", "#0D47A1", "Behavior satisfies a temporal property"),
            edge_desc("EventFollowsProtocol", "event", "protocol", "solid", "#004D40", "Event follows a synchronization protocol"),
            edge_desc("PropertyDependsOnInvariant", "property", "invariant", "dashed", "#0D47A1", "Property depends on an invariant"),
            edge_desc("RefinementRefinesAbstract", "refinement", "behavior", "dashed", "#1B5E20", "Refinement maps from this abstract behavior"),
            edge_desc("RefinementRefinesConcrete", "refinement", "behavior", "solid", "#1B5E20", "Refinement maps to this concrete behavior"),
            edge_desc("RefinementChainsToRefinement", "refinement", "refinement", "dashed", "#1B5E20", "Chain of refinements"),
            edge_desc("EventParticipatesInProcess", "event", "process", "solid", "#006064", "Event participates in a process"),
            edge_desc("ProcessComposesProcess", "process", "process", "solid", "#006064", "Process composes sub-processes"),
        ]
    }

    fn enhancements(&self) -> Vec<EntityEnhancementDescriptor> {
        vec![
            EntityEnhancementDescriptor {
                target_kind: "behavior".into(),
                source_extension: "@specforge/formal".into(),
                fields: vec![
                    fd_ref("requires", "reference_list", "BehaviorRequiresInvariant", "invariant", Some("Invariants that must hold as preconditions before execution")),
                    fd_ref("ensures", "reference_list", "BehaviorEnsuresInvariant", "invariant", Some("Invariants guaranteed as postconditions after successful execution")),
                    fd_ref("maintains", "reference_list", "BehaviorMaintainsInvariant", "invariant", Some("Invariants preserved throughout execution")),
                    fd_ref("satisfies", "reference_list", "BehaviorSatisfiesProperty", "property", Some("Temporal properties this behavior satisfies")),
                    fd("sync", "string_list", false, Some("Synchronization constraints for execution"), None, None),
                ],
                edge_types: vec![
                    EdgeTypeDescriptor {
                        label: "BehaviorRequiresInvariant".into(),
                        description: Some("Behavior requires an invariant as precondition".into()),
                        source_kind: Some("behavior".into()),
                        target_kind: Some("invariant".into()),
                        edge_style: Some("solid".into()),
                        edge_color: Some("#1A237E".into()),
                        edge_arrowhead: None,
                    },
                    EdgeTypeDescriptor {
                        label: "BehaviorEnsuresInvariant".into(),
                        description: Some("Behavior ensures an invariant as postcondition".into()),
                        source_kind: Some("behavior".into()),
                        target_kind: Some("invariant".into()),
                        edge_style: Some("solid".into()),
                        edge_color: Some("#1A237E".into()),
                        edge_arrowhead: None,
                    },
                    EdgeTypeDescriptor {
                        label: "BehaviorMaintainsInvariant".into(),
                        description: Some("Behavior maintains an invariant as frame invariant throughout execution".into()),
                        source_kind: Some("behavior".into()),
                        target_kind: Some("invariant".into()),
                        edge_style: Some("dashed".into()),
                        edge_color: Some("#1A237E".into()),
                        edge_arrowhead: None,
                    },
                    EdgeTypeDescriptor {
                        label: "BehaviorSatisfiesProperty".into(),
                        description: Some("Behavior satisfies a temporal property".into()),
                        source_kind: Some("behavior".into()),
                        target_kind: Some("property".into()),
                        edge_style: Some("solid".into()),
                        edge_color: Some("#0D47A1".into()),
                        edge_arrowhead: None,
                    },
                ],
            },
            EntityEnhancementDescriptor {
                target_kind: "event".into(),
                source_extension: "@specforge/formal".into(),
                fields: vec![
                    fd_ref("follows_protocol", "reference_list", "EventFollowsProtocol", "protocol", Some("Synchronization protocol this event follows")),
                    fd_ref("participates_in", "reference_list", "EventParticipatesInProcess", "process", Some("Processes this event participates in")),
                    fd("sync", "string_list", false, Some("Synchronization constraints for event processing"), None, None),
                ],
                edge_types: vec![
                    EdgeTypeDescriptor {
                        label: "EventFollowsProtocol".into(),
                        description: Some("Event follows a synchronization protocol".into()),
                        source_kind: Some("event".into()),
                        target_kind: Some("protocol".into()),
                        edge_style: Some("solid".into()),
                        edge_color: Some("#004D40".into()),
                        edge_arrowhead: None,
                    },
                    EdgeTypeDescriptor {
                        label: "EventParticipatesInProcess".into(),
                        description: Some("Event participates in a process".into()),
                        source_kind: Some("event".into()),
                        target_kind: Some("process".into()),
                        edge_style: Some("solid".into()),
                        edge_color: Some("#006064".into()),
                        edge_arrowhead: None,
                    },
                ],
            },
        ]
    }

    fn validation_rules(&self) -> Vec<ValidationRuleDescriptor> {
        vec![
            fvc("W059", "property", "property_type", &["safety", "liveness", "fairness"],
                "property '{id}' has invalid property_type '{value}' — expected one of: safety, liveness, fairness"),
        ]
    }
}

impl BuiltinExtension for FormalExtension {
    fn handshake(&self) -> HandshakeResponse {
        HandshakeResponse {
            protocol_version: "1.0".into(),
            name: "@specforge/formal".into(),
            version: "1.0.0".into(),
            contribution_flags: ContributionFlags {
                entities: true,
                validators: true,
                ..Default::default()
            },
            peer_dependencies: vec![PeerDependency {
                name: "@specforge/software".into(),
                version: "^1.0".into(),
                optional: false,
            }],
            sandbox_policy: None,
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

// -- Helper functions --

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
    }
}

fn fd(name: &str, ft: &str, required: bool, desc: Option<&str>, edge: Option<&str>, target: Option<&str>) -> FieldDescriptor {
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

fn fd_ref(name: &str, ft: &str, edge: &str, target: &str, desc: Option<&str>) -> FieldDescriptor {
    fd(name, ft, false, desc, Some(edge), Some(target))
}

fn ekd_defaults() -> EntityKindDescriptor {
    EntityKindDescriptor {
        name: String::new(),
        keyword: None,
        description: None,
        fields: vec![],
        testable: false,
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
    }
}

fn edge_desc(label: &str, src: &str, tgt: &str, style: &str, color: &str, desc: &str) -> EdgeTypeDescriptor {
    EdgeTypeDescriptor {
        label: label.into(),
        description: Some(desc.into()),
        source_kind: Some(src.into()),
        target_kind: Some(tgt.into()),
        edge_style: Some(style.into()),
        edge_color: Some(color.into()),
        edge_arrowhead: None,
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

fn fvc(code: &str, target: &str, field: &str, values: &[&str], msg: &str) -> ValidationRuleDescriptor {
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
