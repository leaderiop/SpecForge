use specforge_wasm::builtin::BuiltinExtension;
use specforge_wasm::protocol::{
    CompilerPassDescriptor, ContributionFlags, DescribeResponse, EdgeTypeDescriptor,
    EntityEnhancementDescriptor, EntityKindDescriptor, FeatureFlagDescriptor,
    FieldConstraintDescriptor, FieldDescriptor, HandshakeResponse, SharedFieldDescriptor,
    SurfaceDescriptor, ValidationRuleDescriptor, ValidationSeverity,
};

pub struct ProductExtension;

impl ProductExtension {
    fn entity_kinds(&self) -> Vec<EntityKindDescriptor> {
        vec![
            EntityKindDescriptor {
                name: "Feature".into(),
                keyword: Some("feature".into()),
                description: Some("A user-facing capability that solves a specific problem".into()),
                semantic_token: Some("class".into()),
                lsp_icon: Some("Class".into()),
                dot_shape: Some("box".into()),
                dot_color: Some("#2196F3".into()),
                dot_fillcolor: Some("#E3F2FD".into()),
                fields: vec![
                    fd("description", "string", false, Some("Human-readable summary of the feature"), None, None),
                    fd("problem", "string", false, Some("The user problem this feature addresses"), None, None),
                    fd("solution", "string", false, Some("How this feature solves the stated problem"), None, None),
                    fd("priority", "string", false, Some("Importance level: critical, high, medium, or low"), None, None),
                    fd("status", "string", false, Some("Lifecycle state: proposed, accepted, in_progress, done, deferred, or deprecated"), None, None),
                    fd("acceptance", "string_list", false, Some("Criteria that must be met for the feature to be considered complete"), None, None),
                    fd_ref("depends_on", "reference_list", "FeatureDependsOn", "feature", Some("Other features that must be completed before this one")),
                    fd_ref("features", "reference_list", "FeatureRelatesTo", "feature", Some("Related features referenced by this feature")),
                    fd("refs", "string_list", false, Some("External references such as issues, URLs, or documents"), None, None),
                    fd("reason", "string", false, Some("Justification for the current status or a status change"), None, None),
                    fd("owner", "string", false, Some("Person or team responsible for this feature"), None, None),
                    fd("contributors", "string_list", false, Some("Additional people or teams contributing to this feature"), None, None),
                    fd("effort", "string", false, Some("T-shirt size estimate: xs, s, m, l, or xl"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Journey".into(),
                keyword: Some("journey".into()),
                description: Some("A user journey through the product experience".into()),
                semantic_token: Some("event".into()),
                lsp_icon: Some("Event".into()),
                dot_shape: Some("ellipse".into()),
                dot_color: Some("#FF9800".into()),
                dot_fillcolor: Some("#FFF3E0".into()),
                fields: vec![
                    fd_ref("persona", "reference", "JourneyTargetsPersona", "persona", Some("The user archetype who undertakes this journey")),
                    fd("description", "string", false, Some("Human-readable summary of the journey"), None, None),
                    fd_ref("channels", "reference_list", "JourneyUsesChannel", "channel", Some("Communication or distribution channels used in this journey")),
                    fd_ref("features", "reference_list", "JourneyExercisesFeature", "feature", Some("Features exercised during this journey")),
                    fd("flow", "string_list", false, Some("Ordered steps the user takes through the journey"), None, None),
                    fd("priority", "string", false, Some("Importance level: critical, high, medium, or low"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Deliverable".into(),
                keyword: Some("deliverable".into()),
                description: Some("A shippable unit of work with dependencies and milestones".into()),
                semantic_token: Some("struct".into()),
                lsp_icon: Some("Package".into()),
                dot_shape: Some("box3d".into()),
                dot_color: Some("#4CAF50".into()),
                dot_fillcolor: Some("#E8F5E9".into()),
                fields: vec![
                    fd("description", "string", false, Some("Human-readable summary of the deliverable"), None, None),
                    fd("artifact_type", "string", false, Some("Kind of artifact produced: cli, service, library, web_app, mobile_app, api, extension, documentation, or package"), None, None),
                    fd("status", "string", false, Some("Lifecycle state: draft, in_progress, shipped, or deprecated"), None, None),
                    fd_ref("journeys", "reference_list", "DeliverableSupportsJourney", "journey", Some("User journeys this deliverable supports")),
                    fd_ref("modules", "reference_list", "DeliverableContainsModule", "module", Some("Modules included in this deliverable")),
                    fd("version", "string", false, Some("Semantic version of this deliverable"), None, None),
                    fd_ref("milestones", "reference_list", "DeliverableTrackedByMilestone", "milestone", Some("Milestones this deliverable is tracked under")),
                    fd_ref("depends_on", "reference_list", "DeliverableDependsOn", "deliverable", Some("Other deliverables that must ship before this one")),
                    fd("reason", "string", false, Some("Justification for the current status or a status change"), None, None),
                    fd("owner", "string", false, Some("Person or team responsible for this deliverable"), None, None),
                    fd("contributors", "string_list", false, Some("Additional people or teams contributing to this deliverable"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Milestone".into(),
                keyword: Some("milestone".into()),
                description: Some("A significant project checkpoint with tracked progress".into()),
                semantic_token: Some("namespace".into()),
                lsp_icon: Some("Folder".into()),
                dot_shape: Some("hexagon".into()),
                dot_color: Some("#9C27B0".into()),
                dot_fillcolor: Some("#F3E5F5".into()),
                fields: vec![
                    fd("description", "string", false, Some("Human-readable summary of the milestone"), None, None),
                    fd("status", "string", false, Some("Lifecycle state: planned, in_progress, completed, or blocked"), None, None),
                    fd_ref("features", "reference_list", "MilestoneDeliversFeature", "feature", Some("Features that must be delivered to complete this milestone")),
                    fd("exit_criteria", "string_list", false, Some("Conditions that must be met to consider the milestone complete"), None, None),
                    fd("target_date", "string", false, Some("Planned completion date for this milestone"), None, None),
                    fd("start_date", "string", false, Some("Date when work on this milestone began or is planned to begin"), None, None),
                    fd_ref("modules", "reference_list", "MilestoneScopesModule", "module", Some("Modules scoped to this milestone")),
                    fd_ref("depends_on", "reference_list", "MilestoneDependsOn", "milestone", Some("Other milestones that must be completed before this one")),
                    fd("blockers", "string_list", false, Some("Outstanding issues preventing progress on this milestone"), None, None),
                    fd("priority", "string", false, Some("Importance level: critical, high, medium, or low"), None, None),
                    fd("reason", "string", false, Some("Justification for the current status or a status change"), None, None),
                    fd("owner", "string", false, Some("Person or team responsible for this milestone"), None, None),
                    fd("contributors", "string_list", false, Some("Additional people or teams contributing to this milestone"), None, None),
                    fd("refs", "string_list", false, Some("External references such as issues, URLs, or documents"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Module".into(),
                keyword: Some("module".into()),
                description: Some("A logical grouping of related features and behaviors".into()),
                semantic_token: Some("namespace".into()),
                lsp_icon: Some("Module".into()),
                dot_shape: Some("component".into()),
                dot_color: Some("#607D8B".into()),
                dot_fillcolor: Some("#ECEFF1".into()),
                fields: vec![
                    fd("family", "string", false, Some("Category or family this module belongs to"), None, None),
                    fd("description", "string", false, Some("Human-readable summary of the module"), None, None),
                    fd_ref("features", "reference_list", "ModuleContainsFeature", "feature", Some("Features contained within this module")),
                    fd_ref("depends_on", "reference_list", "ModuleDependsOn", "module", Some("Other modules this module depends on")),
                    fd("reason", "string", false, Some("Justification for this module's existence or grouping"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Term".into(),
                keyword: Some("term".into()),
                description: Some("A glossary entry defining domain-specific vocabulary".into()),
                singleton: false,
                semantic_token: Some("string".into()),
                lsp_icon: Some("Text".into()),
                dot_shape: Some("note".into()),
                dot_color: Some("#795548".into()),
                dot_fillcolor: Some("#EFEBE9".into()),
                fields: vec![
                    fd("definition", "string", true, Some("The precise meaning of this term in the project's domain"), None, None),
                    fd("context", "string", false, Some("Where or how this term is typically used"), None, None),
                    fd("aliases", "string_list", false, Some("Alternative names or abbreviations for this term"), None, None),
                    fd_ref("see_also", "reference_list", "TermReferencesRelatedTerm", "term", Some("Related terms for cross-referencing")),
                    fd_ref("module", "reference", "TermBelongsToModule", "module", Some("The module (bounded context) that owns this term's definition")),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Persona".into(),
                keyword: Some("persona".into()),
                description: Some("A user archetype representing a target audience segment".into()),
                semantic_token: Some("variable".into()),
                lsp_icon: Some("Variable".into()),
                dot_shape: Some("ellipse".into()),
                dot_color: Some("#E91E63".into()),
                dot_fillcolor: Some("#FCE4EC".into()),
                fields: vec![
                    fd("description", "string", false, Some("Human-readable summary of this persona"), None, None),
                    fd("technical_level", "string", false, Some("Technical proficiency of this persona (e.g., beginner, intermediate, expert)"), None, None),
                    fd("goals", "string_list", false, Some("What this persona wants to achieve with the product"), None, None),
                    fd("pain_points", "string_list", false, Some("Frustrations or problems this persona currently faces"), None, None),
                    fd("status", "string", false, Some("Whether this persona is active or deprecated"), None, None),
                    fd("reason", "string", false, Some("Justification for the current status or a status change"), None, None),
                    fd_ref("key_features", "reference_list", "PersonaPrioritizesFeature", "feature", Some("Features this persona cares about most")),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Channel".into(),
                keyword: Some("channel".into()),
                description: Some("A communication or distribution channel for the product".into()),
                semantic_token: Some("interface".into()),
                lsp_icon: Some("Interface".into()),
                dot_shape: Some("rectangle".into()),
                dot_color: Some("#00BCD4".into()),
                dot_fillcolor: Some("#E0F7FA".into()),
                fields: vec![
                    fd("description", "string", false, Some("Human-readable summary of this channel"), None, None),
                    fd("interaction_model", "string", false, Some("How users interact through this channel (e.g., sync, async, push, pull)"), None, None),
                    fd("url", "string", false, Some("URL or endpoint for this channel"), None, None),
                    fd("status", "string", false, Some("Whether this channel is active or deprecated"), None, None),
                    fd("reason", "string", false, Some("Justification for the current status or a status change"), None, None),
                ],
                ..ekd_defaults()
            },
            EntityKindDescriptor {
                name: "Release".into(),
                keyword: Some("release".into()),
                description: Some("A versioned product release bundling deliverables".into()),
                semantic_token: Some("constant".into()),
                lsp_icon: Some("Constant".into()),
                dot_shape: Some("doubleoctagon".into()),
                dot_color: Some("#FF5722".into()),
                dot_fillcolor: Some("#FBE9E7".into()),
                fields: vec![
                    fd("description", "string", false, Some("Human-readable summary of this release"), None, None),
                    fd("version", "string", true, Some("Semantic version identifier for this release"), None, None),
                    fd("status", "string", false, Some("Lifecycle state of the release"), None, None),
                    fd_ref("deliverables", "reference_list", "ReleaseIncludesDeliverable", "deliverable", Some("Deliverables included in this release")),
                    fd_ref("milestones", "reference_list", "ReleaseCompletesMilestone", "milestone", Some("Milestones completed by this release")),
                    fd("target_date", "string", false, Some("Planned release date"), None, None),
                    fd("release_date", "string", false, Some("Actual date the release shipped"), None, None),
                    fd("changelog", "string", false, Some("Summary of changes included in this release"), None, None),
                    fd_ref("depends_on", "reference_list", "ReleaseDependsOn", "release", Some("Prior releases that must ship before this one")),
                    fd("owner", "string", false, Some("Person or team responsible for this release"), None, None),
                    fd("contributors", "string_list", false, Some("Additional people or teams contributing to this release"), None, None),
                    fd("reason", "string", false, Some("Justification for the current status or a status change"), None, None),
                    fd("refs", "string_list", false, Some("External references such as issues, URLs, or documents"), None, None),
                ],
                ..ekd_defaults()
            },
        ]
    }

    fn edge_types(&self) -> Vec<EdgeTypeDescriptor> {
        vec![
            edge("FeatureDependsOn", "feature", "feature", "dashed", "#2196F3", None),
            edge_desc("FeatureRelatesTo", "feature", "feature", "dotted", "#2196F3", "Feature has a non-dependency relationship to another feature"),
            edge("JourneyExercisesFeature", "journey", "feature", "solid", "#FF9800", None),
            edge("JourneyTargetsPersona", "journey", "persona", "solid", "#E91E63", None),
            edge("JourneyUsesChannel", "journey", "channel", "solid", "#00BCD4", None),
            edge("DeliverableSupportsJourney", "deliverable", "journey", "solid", "#4CAF50", None),
            edge("DeliverableContainsModule", "deliverable", "module", "solid", "#607D8B", None),
            edge("DeliverableTrackedByMilestone", "deliverable", "milestone", "solid", "#9C27B0", None),
            edge("DeliverableDependsOn", "deliverable", "deliverable", "dashed", "#4CAF50", None),
            edge("MilestoneDeliversFeature", "milestone", "feature", "solid", "#9C27B0", None),
            edge("MilestoneScopesModule", "milestone", "module", "solid", "#607D8B", None),
            edge("MilestoneDependsOn", "milestone", "milestone", "dashed", "#9C27B0", None),
            edge("ModuleContainsFeature", "module", "feature", "solid", "#607D8B", None),
            edge("ModuleDependsOn", "module", "module", "dashed", "#607D8B", None),
            edge("TermReferencesRelatedTerm", "term", "term", "dotted", "#795548", None),
            edge_desc("TermBelongsToModule", "term", "module", "dotted", "#795548", "Term is defined within a module's bounded context"),
            edge("ReleaseIncludesDeliverable", "release", "deliverable", "solid", "#FF5722", None),
            edge("ReleaseCompletesMilestone", "release", "milestone", "solid", "#FF5722", None),
            edge("ReleaseDependsOn", "release", "release", "dashed", "#FF5722", None),
            edge_desc("PersonaPrioritizesFeature", "persona", "feature", "solid", "#E91E63", "Persona prioritizes a feature"),
        ]
    }

    fn shared_fields(&self) -> Vec<SharedFieldDescriptor> {
        vec![FieldDescriptor {
            name: "tags".into(),
            field_type: "string_list".into(),
            description: Some("Freeform labels for filtering and categorization".into()),
            ..fd_defaults()
        }]
    }

    fn validation_rules(&self) -> Vec<ValidationRuleDescriptor> {
        vec![
            // Field value constraints
            fvc("W077", "feature", "status", &["proposed", "accepted", "in_progress", "done", "deferred", "deprecated"],
                "feature '{id}' has invalid status '{value}' — expected one of: proposed, accepted, in_progress, done, deferred, deprecated"),
            fvc("W078", "feature", "priority", &["critical", "high", "medium", "low"],
                "{kind} '{id}' has invalid priority '{value}' — expected one of: critical, high, medium, low"),
            fvc("W078", "journey", "priority", &["critical", "high", "medium", "low"],
                "{kind} '{id}' has invalid priority '{value}' — expected one of: critical, high, medium, low"),
            fvc("W078", "milestone", "priority", &["critical", "high", "medium", "low"],
                "{kind} '{id}' has invalid priority '{value}' — expected one of: critical, high, medium, low"),
            fvc("W078", "constraint", "priority", &["critical", "high", "medium", "low"],
                "{kind} '{id}' has invalid priority '{value}' — expected one of: critical, high, medium, low"),
            fvc("W079", "milestone", "status", &["planned", "in_progress", "completed", "blocked"],
                "milestone '{id}' has invalid status '{value}' — expected one of: planned, in_progress, completed, blocked"),
            fvc("W080", "deliverable", "artifact_type", &["cli", "service", "library", "web_app", "mobile_app", "api", "extension", "documentation", "package"],
                "deliverable '{id}' has invalid artifact_type '{value}' — expected one of: cli, service, library, web_app, mobile_app, api, extension, documentation, package"),
            fvc("W083", "persona", "status", &["active", "deprecated"],
                "persona '{id}' has invalid status '{value}' — expected one of: active, deprecated"),
            fvc("W084", "channel", "status", &["active", "deprecated"],
                "channel '{id}' has invalid status '{value}' — expected one of: active, deprecated"),
            fvc("W085", "deliverable", "status", &["draft", "in_progress", "shipped", "deprecated"],
                "deliverable '{id}' has invalid status '{value}' — expected one of: draft, in_progress, shipped, deprecated"),
            fvc("W095", "feature", "effort", &["xs", "s", "m", "l", "xl"],
                "feature '{id}' has invalid effort '{value}' — expected one of: xs, s, m, l, xl"),
            // No-incoming-edges checks
            no_incoming("W041", "feature", "feature '{id}' has no incoming edges — it may be unreferenced by any journey, milestone, or module"),
            no_incoming("W042", "journey", "journey '{id}' has no incoming edges — it may be unreferenced by any deliverable"),
            no_incoming("W044", "module", "module '{id}' has no incoming edges — it may be unreferenced by any deliverable or milestone"),
            // No-edges (info)
            ValidationRuleDescriptor {
                code: "I010".into(),
                severity: ValidationSeverity::Info,
                message_template: "term '{id}' has no edges — it may be unreferenced".into(),
                check: "no_edges".into(),
                target_kind: Some("term".into()),
                ..vrd_defaults()
            },
            // No-incoming (info)
            ValidationRuleDescriptor {
                code: "I046".into(),
                severity: ValidationSeverity::Info,
                message_template: "persona '{id}' has no incoming edges — it may be unreferenced by any journey".into(),
                check: "no_incoming_edges".into(),
                target_kind: Some("persona".into()),
                ..vrd_defaults()
            },
            ValidationRuleDescriptor {
                code: "I047".into(),
                severity: ValidationSeverity::Info,
                message_template: "channel '{id}' has no incoming edges — it may be unreferenced by any journey".into(),
                check: "no_incoming_edges".into(),
                target_kind: Some("channel".into()),
                ..vrd_defaults()
            },
            // Cycle detection
            cycle("E007", "module", "ModuleDependsOn", "module dependency cycle detected involving '{id}'"),
            cycle("E015", "milestone", "MilestoneDependsOn", "milestone dependency cycle detected involving '{id}'"),
            cycle("E016", "deliverable", "DeliverableDependsOn", "deliverable dependency cycle detected involving '{id}'"),
            cycle("W045", "feature", "FeatureDependsOn", "feature dependency cycle detected involving '{id}'"),
            cycle("W092", "release", "ReleaseDependsOn", "release dependency cycle detected involving '{id}'"),
            // Version format
            ValidationRuleDescriptor {
                code: "W093".into(),
                severity: ValidationSeverity::Warning,
                message_template: "release '{id}' has invalid version format — expected semver (e.g., 1.0.0)".into(),
                check: "field_value_constraint".into(),
                target_kind: Some("release".into()),
                field: Some("version".into()),
                constraint: Some(FieldConstraintDescriptor {
                    kind: "matches".into(),
                    pattern: Some(".".into()),
                    values: vec![],
                }),
                ..vrd_defaults()
            },
            // Missing field
            ValidationRuleDescriptor {
                code: "W049".into(),
                severity: ValidationSeverity::Warning,
                message_template: "milestone '{id}' has no features and no modules — it may be empty".into(),
                check: "missing_field_when_flag_set".into(),
                target_kind: Some("milestone".into()),
                field: Some("features".into()),
                ..vrd_defaults()
            },
            // Conditional field rules: when status=X, field Y must be present
            cfr("I059", ValidationSeverity::Info, "feature", "status", "deferred", "reason",
                "feature '{id}' has status 'deferred' but no reason --- consider adding a reason field"),
            cfr("W057", ValidationSeverity::Warning, "milestone", "status", "completed", "exit_criteria",
                "milestone '{id}' has status 'completed' but no exit_criteria"),
            cfr("I060", ValidationSeverity::Info, "milestone", "status", "blocked", "blockers",
                "milestone '{id}' has status 'blocked' but no blockers --- consider listing what is blocking"),
            cfr("I066", ValidationSeverity::Info, "deliverable", "status", "deprecated", "reason",
                "deliverable '{id}' has status 'deprecated' but no reason"),
            cfr("I069", ValidationSeverity::Info, "persona", "status", "deprecated", "reason",
                "persona '{id}' has status 'deprecated' but no reason"),
            cfr("I070", ValidationSeverity::Info, "channel", "status", "deprecated", "reason",
                "channel '{id}' has status 'deprecated' but no reason"),
        ]
    }
}

impl BuiltinExtension for ProductExtension {
    fn handshake(&self) -> HandshakeResponse {
        HandshakeResponse {
            protocol_version: "1.0.0".into(),
            name: "@specforge/product".into(),
            version: "1.0.0".into(),
            contribution_flags: ContributionFlags {
                entities: true,
                validators: true,
                ..Default::default()
            },
            peer_dependencies: vec![],
            sandbox_policy: None,
        }
    }

    fn describe(&self, category: &str) -> Option<DescribeResponse> {
        let items = match category {
            "entities" => serde_json::to_value(self.entity_kinds()).unwrap(),
            "edges" => serde_json::to_value(self.edge_types()).unwrap(),
            "fields" => serde_json::to_value(Vec::<FieldDescriptor>::new()).unwrap(),
            "shared_fields" => serde_json::to_value(self.shared_fields()).unwrap(),
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

fn edge(label: &str, src: &str, tgt: &str, style: &str, color: &str, arrowhead: Option<&str>) -> EdgeTypeDescriptor {
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

fn edge_desc(label: &str, src: &str, tgt: &str, style: &str, color: &str, desc: &str) -> EdgeTypeDescriptor {
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

fn cycle(code: &str, target: &str, edge_type: &str, msg: &str) -> ValidationRuleDescriptor {
    let severity = if code.starts_with('E') {
        ValidationSeverity::Error
    } else {
        ValidationSeverity::Warning
    };
    ValidationRuleDescriptor {
        code: code.into(),
        severity,
        message_template: msg.into(),
        check: "cycle_detection".into(),
        target_kind: Some(target.into()),
        edge_type: Some(edge_type.into()),
        ..vrd_defaults()
    }
}

/// Conditional field required: when `condition_field` = `condition_value`,
/// `required_field` must be present on the entity.
fn cfr(
    code: &str,
    severity: ValidationSeverity,
    target: &str,
    condition_field: &str,
    condition_value: &str,
    required_field: &str,
    msg: &str,
) -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: code.into(),
        severity,
        message_template: msg.into(),
        check: "conditional_field_required".into(),
        target_kind: Some(target.into()),
        field: Some(required_field.into()),
        constraint: Some(FieldConstraintDescriptor {
            kind: "when_field_equals".into(),
            pattern: Some(condition_field.into()),
            values: vec![condition_value.into()],
        }),
        ..vrd_defaults()
    }
}
