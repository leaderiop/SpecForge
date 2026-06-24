use specforge_wasm::builtin::BuiltinExtension;
use specforge_wasm::protocol::{
    CompilerPassDescriptor, ContributionFlags, DescribeResponse, EdgeTypeDescriptor,
    EntityEnhancementDescriptor, EntityKindDescriptor, FeatureFlagDescriptor,
    FieldConstraintDescriptor, FieldDescriptor, HandshakeResponse, PeerDependency,
    SharedFieldDescriptor, SurfaceDescriptor, ValidationRuleDescriptor, ValidationSeverity,
};

pub struct GovernanceExtension;

impl GovernanceExtension {
    fn entity_kinds(&self) -> Vec<EntityKindDescriptor> {
        vec![
            EntityKindDescriptor {
                name: "Decision".into(),
                keyword: Some("decision".into()),
                description: Some("An architectural or design decision with rationale and status".into()),
                inference_guide: Some("Look for Architecture Decision Records (ADRs), design documents, RFC files, and significant comments explaining WHY something was built a certain way. Signals: docs/adr/ or docs/decisions/ directories; files named ADR-*, DECISION-*, RFC-*; PR descriptions with 'decided to', 'chosen approach', 'trade-off'; comments starting with 'Decision:', 'Rationale:', 'Why:'. Extract status (proposed/accepted/deprecated), context (what prompted it), decision (what was chosen), and consequences (trade-offs). Link to invariants the decision protects and constraints it imposes. Skip: trivial implementation choices, style preferences, auto-generated config.".into()),
                semantic_token: Some("string".into()),
                lsp_icon: Some("Text".into()),
                dot_shape: Some("note".into()),
                dot_color: Some("#6A1B9A".into()),
                dot_fillcolor: Some("#F3E5F5".into()),
                fields: vec![
                    fd("status", "string", true, Some("Current lifecycle status of the decision (e.g. proposed, accepted, deprecated)"), None, None),
                    fd("context", "string", true, Some("Background and circumstances that motivated this decision"), None, None),
                    fd("decision", "string", true, Some("The decision statement itself"), None, None),
                    fd("consequences", "string_list", false, Some("Expected outcomes and trade-offs resulting from this decision"), None, None),
                    fd_ref("superseded_by", "reference", "DecisionSupersedesDecision", "decision", Some("Reference to a newer decision that replaces this one")),
                    fd_ref("invariants", "reference_list", "DecisionProtectsInvariant", "invariant", Some("Invariants that this decision protects")),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                    fd("description", "string", false, Some("Free-form description of the decision"), None, None),
                    fd("date", "string", false, Some("Date the decision was made or last updated"), None, None),
                    fd("authors", "string_list", false, Some("People who authored this decision record"), None, None),
                    fd("tags", "string_list", false, Some("Categorization tags for filtering and grouping"), None, None),
                    fd("alternatives", "string_list", false, Some("Alternative options that were considered"), None, None),
                    fd("reason", "string", false, Some("Justification for why this option was chosen over alternatives"), None, None),
                    fd("deciders", "string_list", false, Some("People who approved or signed off on this decision"), None, None),
                    fd_ref("affects_features", "reference_list", "DecisionAffectsFeature", "feature", Some("Product features affected by this decision (requires @specforge/product)")),
                    fd_ref("constraints", "reference_list", "DecisionImposesConstraint", "constraint", Some("Constraints imposed by this decision")),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Constraint".into(),
                keyword: Some("constraint".into()),
                description: Some("A technical or business constraint that limits design choices".into()),
                inference_guide: Some("Look for non-functional requirements, SLAs, performance budgets, regulatory mandates, and technical limitations. Signals: performance benchmarks or thresholds in code/config; rate limiting configuration; compliance annotations (@HIPAA, @PCI-DSS); resource limits (max connections, memory caps, timeout values); security policies; dependency version constraints; platform limitations documented in README or config. Set category (performance, security, regulatory, compatibility). Include metric and threshold when quantifiable (e.g., metric='p99 latency', threshold='<200ms'). Link enforced_by to behaviors that enforce this constraint at runtime. Constraints are quantified limits with measurable thresholds; for architectural rules about state validity, use invariant instead. Skip: soft preferences, guidelines that aren't enforced.".into()),
                semantic_token: Some("property".into()),
                lsp_icon: Some("Property".into()),
                dot_shape: Some("octagon".into()),
                dot_color: Some("#BF360C".into()),
                dot_fillcolor: Some("#FBE9E7".into()),
                testable: true,
                supports_verify: true,
                fields: vec![
                    fd("category", "string", false, Some("Classification of the constraint (e.g. performance, security, regulatory)"), None, None),
                    fd("priority", "string", false, Some("Relative importance of this constraint"), None, None),
                    fd("metric", "string", false, Some("Measurable quantity used to evaluate compliance"), None, None),
                    fd("threshold", "string", false, Some("Acceptable limit or target value for the metric"), None, None),
                    fd("description", "string", true, Some("Free-form description of the constraint"), None, None),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                    fd_ref_inv("enforced_by", "reference_list", "ConstraintEnforcedByBehavior", "behavior", "invariants", Some("Behaviors that enforce this constraint at runtime")),
                    fd("scope", "string", false, Some("Applicability scope (e.g. system-wide, per-module, per-request)"), None, None),
                    fd_ref("constrains", "reference_list", "ConstraintConstrainsBehavior", "behavior", Some("Behaviors that are limited or governed by this constraint")),
                    fd_ref("protects", "reference_list", "ConstraintProtectsInvariant", "invariant", Some("Invariants that this constraint helps protect")),
                    fd_ref("governs_features", "reference_list", "ConstraintGovernsFeature", "feature", Some("Product features this constraint governs (requires @specforge/product)")),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "FailureMode".into(),
                keyword: Some("failure_mode".into()),
                description: Some("A potential failure scenario with severity and mitigation".into()),
                inference_guide: Some("Look for error handling paths, retry logic, circuit breakers, fallback mechanisms, and comments describing what can go wrong. Signals: catch/rescue blocks handling specific failure scenarios; circuit breaker configurations; retry policies with backoff; fallback implementations; chaos engineering tests; error types/classes representing failure categories; timeout handling; dead letter queues. Extract cause (what triggers it), effect (what breaks), and mitigation (how the system recovers). Rate severity/occurrence/detection. Link to the invariant that gets violated and behaviors affected. Skip: generic error handling, validation errors for user input, expected business rule rejections.".into()),
                semantic_token: Some("variable".into()),
                lsp_icon: Some("Variable".into()),
                dot_shape: Some("triangle".into()),
                dot_color: Some("#D32F2F".into()),
                dot_fillcolor: Some("#FFCDD2".into()),
                fields: vec![
                    fd("severity", "string", true, Some("Impact severity rating before mitigation (e.g. critical, high, medium, low)"), None, None),
                    fd("occurrence", "string", false, Some("Likelihood of this failure occurring before mitigation"), None, None),
                    fd("detection", "string", false, Some("Likelihood of detecting this failure before it causes harm"), None, None),
                    fd("cause", "string", true, Some("Root cause or trigger of the failure"), None, None),
                    fd("effect", "string", true, Some("Consequence or impact when the failure occurs"), None, None),
                    fd("mitigation", "string", false, Some("Strategy or action to reduce risk of the failure"), None, None),
                    fd("post_severity", "string", false, Some("Impact severity rating after mitigation is applied"), None, None),
                    fd("post_occurrence", "string", false, Some("Likelihood of occurrence after mitigation is applied"), None, None),
                    fd("post_detection", "string", false, Some("Likelihood of detection after mitigation is applied"), None, None),
                    fd_ref("invariant", "reference", "FailureModeTargetsInvariant", "invariant", Some("The invariant that this failure mode threatens")),
                    fd("refs", "string_list", false, Some("External references (URLs, documents, issue links)"), None, None),
                    fd("description", "string", false, Some("Free-form description of the failure mode"), None, None),
                    fd("risk", "string", false, Some("Overall risk assessment combining severity, occurrence, and detection"), None, None),
                    fd("rpn", "integer", false, Some("Risk Priority Number (severity x occurrence x detection)"), None, None),
                    fd("post_mitigation", "string", false, Some("Summary of the residual risk state after mitigation"), None, None),
                    fd_ref("threatens_features", "reference_list", "FailureModeThreatensFeature", "feature", Some("Product features threatened by this failure mode (requires @specforge/product)")),
                    fd_ref("affected_behaviors", "reference_list", "FailureModeAffectsBehavior", "behavior", Some("Behaviors affected by this failure mode as failure vectors")),
                ],
                ..ekd_defaults()
            },
        ]
    }

    fn edge_types(&self) -> Vec<EdgeTypeDescriptor> {
        vec![
            edge_desc("DecisionProtectsInvariant", "decision", "invariant", "dashed", "#6A1B9A", "Decision protects an invariant"),
            edge_desc("ConstraintEnforcedByBehavior", "constraint", "behavior", "solid", "#BF360C", "Constraint is enforced by a behavior"),
            edge_desc("DecisionSupersedesDecision", "decision", "decision", "dotted", "#6A1B9A", "Decision supersedes a previous decision"),
            edge_desc("ConstraintConstrainsBehavior", "constraint", "behavior", "dashed", "#BF360C", "Constraint constrains a behavior"),
            edge_desc("ConstraintProtectsInvariant", "constraint", "invariant", "dashed", "#BF360C", "Constraint protects an invariant"),
            edge_desc("FailureModeTargetsInvariant", "failure_mode", "invariant", "dashed", "#D32F2F", "Failure mode targets an invariant"),
            edge_desc("ConstraintGovernsFeature", "constraint", "feature", "dashed", "#CC6600", "Constraint governs a product feature"),
            edge_desc("DecisionAffectsFeature", "decision", "feature", "dashed", "#CC6600", "Decision affects a product feature"),
            edge_desc("FailureModeThreatensFeature", "failure_mode", "feature", "dashed", "#CC0000", "Failure mode threatens a product feature"),
            edge_desc("DecisionImposesConstraint", "decision", "constraint", "solid", "#6A1B9A", "Decision imposes a constraint on the system"),
            edge_desc("FailureModeAffectsBehavior", "failure_mode", "behavior", "dashed", "#D32F2F", "Failure mode identifies a behavior as a failure vector"),
        ]
    }

    fn validation_rules(&self) -> Vec<ValidationRuleDescriptor> {
        vec![
            fvc("W050", "decision", "status", &["proposed", "accepted", "deprecated", "superseded"],
                "decision '{id}' has invalid status '{value}' — expected one of: proposed, accepted, deprecated, superseded"),
            fvc("W051", "failure_mode", "severity", &["critical", "high", "medium", "low"],
                "failure_mode '{id}' has invalid severity '{value}' — expected one of: critical, high, medium, low"),
            fvc("W051", "failure_mode", "post_severity", &["critical", "high", "medium", "low"],
                "failure_mode '{id}' has invalid post_severity '{value}' — expected one of: critical, high, medium, low"),
            fvc("W052", "failure_mode", "occurrence", &["certain", "likely", "occasional", "unlikely", "rare"],
                "failure_mode '{id}' has invalid occurrence '{value}' — expected one of: certain, likely, occasional, unlikely, rare"),
            fvc("W052", "failure_mode", "post_occurrence", &["certain", "likely", "occasional", "unlikely", "rare"],
                "failure_mode '{id}' has invalid post_occurrence '{value}' — expected one of: certain, likely, occasional, unlikely, rare"),
            fvc("W053", "failure_mode", "detection", &["certain", "likely", "moderate", "unlikely", "undetectable"],
                "failure_mode '{id}' has invalid detection '{value}' — expected one of: certain, likely, moderate, unlikely, undetectable"),
            fvc("W053", "failure_mode", "post_detection", &["certain", "likely", "moderate", "unlikely", "undetectable"],
                "failure_mode '{id}' has invalid post_detection '{value}' — expected one of: certain, likely, moderate, unlikely, undetectable"),
        ]
    }
}

impl BuiltinExtension for GovernanceExtension {
    fn handshake(&self) -> HandshakeResponse {
        HandshakeResponse {
            protocol_version: "1.0.0".into(),
            name: "@specforge/governance".into(),
            version: "1.0.0".into(),
            contribution_flags: ContributionFlags {
                entities: true,
                validators: true,
                ..Default::default()
            },
            peer_dependencies: vec![
                PeerDependency {
                    name: "@specforge/software".into(),
                    version: "^1.0".into(),
                    optional: false,
                },
                PeerDependency {
                    name: "@specforge/product".into(),
                    version: "^1.0".into(),
                    optional: true,
                },
            ],
            sandbox_policy: None,
        }
    }

    fn describe(&self, category: &str) -> Option<DescribeResponse> {
        let items = match category {
            "entities" => serde_json::to_value(self.entity_kinds()).unwrap(),
            "edges" => serde_json::to_value(self.edge_types()).unwrap(),
            "fields" => serde_json::to_value(Vec::<FieldDescriptor>::new()).unwrap(),
            "shared_fields" => serde_json::to_value(Vec::<SharedFieldDescriptor>::new()).unwrap(),
            "enhancements" => serde_json::to_value(Vec::<EntityEnhancementDescriptor>::new()).unwrap(),
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

fn fd_ref_inv(name: &str, ft: &str, edge: &str, target: &str, inverse: &str, desc: Option<&str>) -> FieldDescriptor {
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
        inference_guide: None,
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
