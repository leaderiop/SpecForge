use specforge_common::{
    CompilerConfig, Diagnostic, DiagnosticBag, EdgeType, EntityId, EntityKind, FieldMap,
    FieldValue, FormatVersion, Module, SourceSpan, ValidationCode,
};
use specforge_graph::SpecGraph;
use specforge_parser::SpecFile;

/// Run all validation passes on the spec graph.
///
/// E001/E002/E003/I004/I005 are already emitted by the resolver.
/// The validator handles graph-structural and semantic checks.
///
/// Passes are conditionally enabled based on installed plugins:
/// - Core passes always run
/// - Product passes only if `@specforge/product` is installed
/// - Governance passes only if `@specforge/governance` is installed
pub fn validate(
    files: &[SpecFile],
    graph: &SpecGraph,
    config: &CompilerConfig,
) -> DiagnosticBag {
    let mut bag = DiagnosticBag::new();

    // Core passes (always run)
    check_orphan_behaviors(graph, &mut bag);      // W001
    check_unused_invariants(graph, &mut bag);      // W003
    check_unverified_behaviors(files, &mut bag);   // W004
    check_empty_scenarios(files, &mut bag);        // E004
    check_duplicate_scenario_titles(files, &mut bag); // E015
    check_scenario_steps(files, &mut bag);         // W015, W016
    check_event_trigger(files, graph, &mut bag);   // E006
    check_orphan_events(graph, &mut bag);          // W007
    check_orphan_refs(graph, &mut bag);            // W012
    check_naming_conventions(files, &mut bag);     // E013, E014, W013
    check_unused_entities(graph, &mut bag);         // W017
    check_format_version(config, files, &mut bag);  // I003

    // Product passes
    if config.has_plugin(Module::Product) {
        check_orphan_features(graph, &mut bag);          // W002
        check_persona_defined(files, config, &mut bag);  // E008
        check_surface_defined(files, config, &mut bag);  // E009
        check_library_cycles(files, graph, &mut bag);    // E007
        check_uncovered_capability(graph, &mut bag);     // W008
        check_orphan_libraries(graph, &mut bag);         // W009
        check_deprecated_feature(files, &mut bag);       // W010
        check_orphan_capabilities(graph, &mut bag);      // W011
        check_deliverables_with_no_capabilities(graph, &mut bag); // W018
        check_unused_glossary_terms(files, &mut bag);    // I006
    }

    // Governance passes
    if config.has_plugin(Module::Governance) {
        check_rpn_mismatch(files, &mut bag);                   // E005
        check_unmitigated_invariants(files, graph, &mut bag);  // W005
        check_unconstrained_behaviors(graph, &mut bag);        // W006
        check_stale_proposals(files, &mut bag);                // I001
        check_constraints_with_no_protects(graph, &mut bag);   // W019
    }

    bag
}

// ── W001: orphan behavior ──────────────────────────────────────────────

fn check_orphan_behaviors(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Behavior) {
        let incoming = graph.incoming_edges(node.id.raw());
        // A behavior is orphan if nothing references it (no feature, capability, etc.)
        if incoming.is_empty() {
            bag.push(Diagnostic::new(
                ValidationCode::W001,
                node.span.clone(),
                format!("behavior `{}` is not referenced by any feature or capability", node.id.raw()),
            ).with_help("add this behavior to a feature's `behaviors` list"));
        }
    }
}

// ── W002: orphan feature (product) ─────────────────────────────────────

fn check_orphan_features(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Feature) {
        let incoming = graph.incoming_edges(node.id.raw());
        if incoming.is_empty() {
            bag.push(Diagnostic::new(
                ValidationCode::W002,
                node.span.clone(),
                format!("feature `{}` is not referenced by any capability or deliverable", node.id.raw()),
            ).with_help("add this feature to a capability's `features` list"));
        }
    }
}

// ── W003: unused invariant ─────────────────────────────────────────────

fn check_unused_invariants(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Invariant) {
        let incoming = graph.incoming_edges(node.id.raw());
        if incoming.is_empty() {
            bag.push(Diagnostic::new(
                ValidationCode::W003,
                node.span.clone(),
                format!("invariant `{}` is not referenced by any behavior", node.id.raw()),
            ).with_help("add this invariant to a behavior's `invariants` list"));
        }
    }
}

// ── W004: unverified testable entity ───────────────────────────────────

fn check_unverified_behaviors(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if entity.kind.is_testable() {
                let has_verify = entity.fields.get("verify").is_some();
                let has_scenario = entity.fields.get("scenario").is_some();
                if !has_verify && !has_scenario {
                    bag.push(Diagnostic::new(
                        ValidationCode::W004,
                        entity.span.clone(),
                        format!(
                            "{} `{}` has no verify statements or scenarios",
                            entity.kind, entity.id.raw()
                        ),
                    ).with_help("add `verify unit \"description\"` or `scenario \"title\" { ... }`"));
                }
            }
        }
    }
}

// ── E004: empty scenario block ─────────────────────────────────────────

fn check_empty_scenarios(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if let Some(FieldValue::ScenarioList(scenarios)) = entity.fields.get("scenario") {
                for scenario in scenarios {
                    if scenario.steps.is_empty() {
                        bag.push(Diagnostic::new(
                            ValidationCode::E004,
                            scenario.span.clone(),
                            format!(
                                "scenario \"{}\" in {} `{}` has no steps",
                                scenario.title, entity.kind, entity.id.raw()
                            ),
                        ).with_help("add at least one given/when/then step"));
                    }
                }
            }
        }
    }
}

// ── E015: duplicate scenario title ─────────────────────────────────────

fn check_duplicate_scenario_titles(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if let Some(FieldValue::ScenarioList(scenarios)) = entity.fields.get("scenario") {
                let mut seen: std::collections::HashMap<&str, &specforge_common::SourceSpan> =
                    std::collections::HashMap::new();
                for scenario in scenarios {
                    if let Some(first_span) = seen.get(scenario.title.as_str()) {
                        bag.push(Diagnostic::new(
                            ValidationCode::E015,
                            scenario.span.clone(),
                            format!(
                                "duplicate scenario title \"{}\" in {} `{}`",
                                scenario.title, entity.kind, entity.id.raw()
                            ),
                        ).with_label((*first_span).clone(), "first declared here"));
                    } else {
                        seen.insert(&scenario.title, &scenario.span);
                    }
                }
            }
        }
    }
}

// ── W015/W016: scenario missing when/then steps ───────────────────────

fn check_scenario_steps(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if let Some(FieldValue::ScenarioList(scenarios)) = entity.fields.get("scenario") {
                for scenario in scenarios {
                    if scenario.steps.is_empty() {
                        continue; // Already caught by E004
                    }
                    let has_when = scenario
                        .steps
                        .iter()
                        .any(|s| s.kind == specforge_common::ScenarioStepKind::When);
                    let has_then = scenario
                        .steps
                        .iter()
                        .any(|s| s.kind == specforge_common::ScenarioStepKind::Then);
                    if !has_when {
                        bag.push(Diagnostic::new(
                            ValidationCode::W015,
                            scenario.span.clone(),
                            format!(
                                "scenario \"{}\" has no 'when' step",
                                scenario.title
                            ),
                        ).with_help("add a `when` step describing the action"));
                    }
                    if !has_then {
                        bag.push(Diagnostic::new(
                            ValidationCode::W016,
                            scenario.span.clone(),
                            format!(
                                "scenario \"{}\" has no 'then' step",
                                scenario.title
                            ),
                        ).with_help("add a `then` step describing the expected outcome"));
                    }
                }
            }
        }
    }
}

// ── W005: unmitigated high-risk invariant (governance) ─────────────────

fn check_unmitigated_invariants(
    files: &[SpecFile],
    graph: &SpecGraph,
    bag: &mut DiagnosticBag,
) {
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::Invariant {
                let is_high_risk = matches!(
                    entity.fields.get("risk"),
                    Some(FieldValue::Enum(s)) if s == "high" || s == "critical"
                );
                if is_high_risk {
                    // Check if any failure_mode mitigates this invariant
                    let incoming = graph.incoming_edges(entity.id.raw());
                    let has_mitigation = incoming
                        .iter()
                        .any(|(_, edge)| edge.edge_type == EdgeType::Mitigates);
                    if !has_mitigation {
                        bag.push(Diagnostic::new(
                            ValidationCode::W005,
                            entity.span.clone(),
                            format!(
                                "high-risk invariant `{}` has no failure mode mitigation",
                                entity.id.raw()
                            ),
                        ).with_help("create a failure_mode block that mitigates this invariant"));
                    }
                }
            }
        }
    }
}

// ── W006: unconstrained behavior (governance) ──────────────────────────

fn check_unconstrained_behaviors(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Behavior) {
        let incoming = graph.incoming_edges(node.id.raw());
        let has_constraint = incoming
            .iter()
            .any(|(_, edge)| edge.edge_type == EdgeType::Constrains);
        if !has_constraint {
            bag.push(Diagnostic::new(
                ValidationCode::W006,
                node.span.clone(),
                format!("behavior `{}` has no constraints", node.id.raw()),
            ).with_help("add a constraint block that constrains this behavior"));
        }
    }
}

// ── W007: orphan event ─────────────────────────────────────────────────

fn check_orphan_events(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Event) {
        let incoming = graph.incoming_edges(node.id.raw());
        let outgoing = graph.outgoing_edges(node.id.raw());
        if incoming.is_empty() && outgoing.is_empty() {
            bag.push(Diagnostic::new(
                ValidationCode::W007,
                node.span.clone(),
                format!("event `{}` is not produced or consumed by any behavior", node.id.raw()),
            ).with_help("add this event to a behavior's `produces` or `events` list"));
        }
    }
}

// ── W008: uncovered capability (product) ───────────────────────────────

fn check_uncovered_capability(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Capability) {
        let outgoing = graph.outgoing_edges(node.id.raw());
        let has_features = outgoing
            .iter()
            .any(|(_, edge)| edge.edge_type == EdgeType::TracesTo);
        if !has_features {
            bag.push(Diagnostic::new(
                ValidationCode::W008,
                node.span.clone(),
                format!("capability `{}` has no features", node.id.raw()),
            ).with_help("add features to this capability's `features` list"));
        }
    }
}

// ── W009: orphan library (product) ─────────────────────────────────────

fn check_orphan_libraries(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Library) {
        let incoming = graph.incoming_edges(node.id.raw());
        if incoming.is_empty() {
            bag.push(Diagnostic::new(
                ValidationCode::W009,
                node.span.clone(),
                format!("library `{}` is not referenced by any deliverable", node.id.raw()),
            ).with_help("add this library to a deliverable's `libraries` list"));
        }
    }
}

// ── W010: deprecated feature (product) ─────────────────────────────────

fn check_deprecated_feature(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::Feature {
                let is_deprecated = matches!(
                    entity.fields.get("status"),
                    Some(FieldValue::Enum(s)) if s == "deprecated"
                );
                if is_deprecated {
                    bag.push(Diagnostic::new(
                        ValidationCode::W010,
                        entity.span.clone(),
                        format!("feature `{}` is deprecated", entity.id.raw()),
                    ).with_help("consider removing references to this feature"));
                }
            }
        }
    }
}

// ── W011: orphan capability (product) ──────────────────────────────────

fn check_orphan_capabilities(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Capability) {
        let incoming = graph.incoming_edges(node.id.raw());
        if incoming.is_empty() {
            bag.push(Diagnostic::new(
                ValidationCode::W011,
                node.span.clone(),
                format!("capability `{}` is not referenced by any deliverable", node.id.raw()),
            ).with_help("add this capability to a deliverable's `capabilities` list"));
        }
    }
}

// ── W012: orphan ref ───────────────────────────────────────────────────

fn check_orphan_refs(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Ref) {
        let incoming = graph.incoming_edges(node.id.raw());
        if incoming.is_empty() {
            bag.push(Diagnostic::new(
                ValidationCode::W012,
                node.span.clone(),
                format!("ref `{}` is not linked from any entity", node.id.raw()),
            ).with_help("add this ref to an entity's `refs` or `links` list"));
        }
    }
}

// ── W017: unused entity (generic) ──────────────────────────────────────

fn check_unused_entities(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    use EntityKind::*;
    // Kinds that have dedicated orphan checks or are singletons/roots
    const SKIP: &[EntityKind] = &[
        Spec,       // root
        Glossary,   // singleton
        Behavior,   // W001
        Feature,    // W002
        Invariant,  // W003
        Event,      // W007
        Library,    // W009
        Capability, // W011
        Ref,        // W012
    ];

    for node in graph.nodes() {
        if SKIP.contains(&node.kind) {
            continue;
        }
        let incoming = graph.incoming_edges(node.id.raw());
        let outgoing = graph.outgoing_edges(node.id.raw());
        if incoming.is_empty() && outgoing.is_empty() {
            bag.push(
                Diagnostic::new(
                    ValidationCode::W017,
                    node.span.clone(),
                    format!(
                        "{} `{}` is not connected to any entity",
                        node.kind,
                        node.id.raw()
                    ),
                )
                .with_help("ensure this entity is referenced somewhere in your spec"),
            );
        }
    }
}

// ── E005: RPN mismatch (governance) ────────────────────────────────────

fn check_rpn_mismatch(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::FailureMode {
                let severity = get_int_field(entity, "severity");
                let occurrence = get_int_field(entity, "occurrence");
                let detection = get_int_field(entity, "detection");

                if let (Some(s), Some(o), Some(d)) = (severity, occurrence, detection) {
                    let computed_rpn = s * o * d;
                    if let Some(declared_rpn) = get_int_field(entity, "rpn") {
                        if computed_rpn != declared_rpn {
                            bag.push(Diagnostic::new(
                                ValidationCode::E005,
                                entity.span.clone(),
                                format!(
                                    "RPN mismatch in `{}`: declared {} but severity({}) * occurrence({}) * detection({}) = {}",
                                    entity.id.raw(), declared_rpn, s, o, d, computed_rpn
                                ),
                            ).with_help(format!("change rpn to {computed_rpn}")));
                        }
                    }
                }
            }
        }
    }
}

// ── E006: invalid event trigger ────────────────────────────────────────

fn check_event_trigger(files: &[SpecFile], graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if entity.kind != EntityKind::Event {
                continue;
            }
            // Extract the trigger target name — parser may produce Reference or Enum
            let trigger_name = match entity.fields.get("trigger") {
                Some(FieldValue::Reference(ref_id)) => Some(ref_id.raw().to_string()),
                Some(FieldValue::Enum(name)) => Some(name.clone()),
                _ => None,
            };
            if let Some(name) = trigger_name {
                // Trigger must reference a behavior
                if let Some(target) = graph.get_node(&name) {
                    if target.kind != EntityKind::Behavior {
                        bag.push(Diagnostic::new(
                            ValidationCode::E006,
                            entity.span.clone(),
                            format!(
                                "event `{}` trigger `{}` is a {} but must be a behavior",
                                entity.id.raw(), name, target.kind
                            ),
                        ));
                    }
                }
                // If target doesn't exist in graph, E001 was already emitted by resolver
            }
        }
    }
}

// ── E007: circular library dependency (product) ────────────────────────

fn check_library_cycles(_files: &[SpecFile], graph: &SpecGraph, bag: &mut DiagnosticBag) {
    // Check for cycles in the depends_on relationship between libraries using DFS with path tracking.
    let libraries: Vec<_> = graph.nodes_of_kind(EntityKind::Library);
    if libraries.is_empty() {
        return;
    }

    let mut globally_visited = std::collections::HashSet::new();
    let mut path = std::collections::HashSet::new();

    for lib_node in &libraries {
        let id = lib_node.id.raw().to_string();
        if globally_visited.contains(&id) {
            continue;
        }
        if dfs_detect_cycle(&id, graph, &mut globally_visited, &mut path) {
            bag.push(Diagnostic::new(
                ValidationCode::E007,
                lib_node.span.clone(),
                format!("circular library dependency involving `{}`", lib_node.id.raw()),
            ).with_help("break the cycle by restructuring library dependencies"));
        }
    }
}

fn dfs_detect_cycle(
    node_id: &str,
    graph: &SpecGraph,
    visited: &mut std::collections::HashSet<String>,
    path: &mut std::collections::HashSet<String>,
) -> bool {
    let id = node_id.to_string();
    if path.contains(&id) {
        return true; // Back edge = cycle
    }
    if visited.contains(&id) {
        return false; // Already fully explored, no cycle through here
    }

    visited.insert(id.clone());
    path.insert(id.clone());

    let outgoing = graph.outgoing_edges(node_id);
    for (target, edge) in outgoing {
        if edge.edge_type == EdgeType::DependsOn
            && target.kind == EntityKind::Library
            && dfs_detect_cycle(target.id.raw(), graph, visited, path)
        {
            path.remove(&id);
            return true;
        }
    }

    path.remove(&id);
    false
}

// ── E008: persona not defined (product) ────────────────────────────────

fn check_persona_defined(
    files: &[SpecFile],
    config: &CompilerConfig,
    bag: &mut DiagnosticBag,
) {
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::Capability {
                if let Some(FieldValue::Enum(persona)) = entity.fields.get("persona") {
                    if !config.has_persona(persona) {
                        bag.push(Diagnostic::new(
                            ValidationCode::E008,
                            entity.span.clone(),
                            format!(
                                "persona `{persona}` in capability `{}` is not defined in spec root",
                                entity.id.raw()
                            ),
                        ).with_help("add a `persona` block in your spec root"));
                    }
                }
            }
        }
    }
}

// ── E009: surface not defined (product) ────────────────────────────────

fn check_surface_defined(
    files: &[SpecFile],
    config: &CompilerConfig,
    bag: &mut DiagnosticBag,
) {
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::Capability {
                if let Some(FieldValue::Enum(surface)) = entity.fields.get("surface") {
                    if !config.has_surface(surface) {
                        bag.push(Diagnostic::new(
                            ValidationCode::E009,
                            entity.span.clone(),
                            format!(
                                "surface `{surface}` in capability `{}` is not defined in spec root",
                                entity.id.raw()
                            ),
                        ).with_help("add a `surface` block in your spec root"));
                    }
                }
            }
        }
    }
}

// ── E013/E014/W013: naming convention validation ────────────────────────

/// Vague names that are too generic to be meaningful entity identifiers.
const VAGUE_NAMES: &[&str] = &[
    "data", "test", "temp", "tmp", "foo", "bar", "baz", "misc", "util",
    "utils", "helper", "helpers", "stuff", "thing", "things", "item",
    "items", "object", "objects", "info", "main", "core", "base", "new",
    "old", "my",
];

fn check_naming_conventions(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            // Only check entities that use named identifiers (skip singletons and refs)
            if !entity.kind.uses_identifier() {
                continue;
            }

            let name = match entity.id.name() {
                Some(n) => n,
                None => continue, // SchemeRef — skip
            };

            // E013: reserved word
            if EntityId::is_reserved_word(name) {
                bag.push(Diagnostic::new(
                    ValidationCode::E013,
                    entity.span.clone(),
                    format!(
                        "`{name}` is a reserved keyword and cannot be used as a {} name",
                        entity.kind
                    ),
                ).with_help("choose a more specific name (e.g., `auth_behavior` instead of `behavior`)"));
                continue; // Skip further checks — the name is fundamentally invalid
            }

            // E014: invalid identifier characters
            if !EntityId::is_valid_identifier(name) {
                bag.push(Diagnostic::new(
                    ValidationCode::E014,
                    entity.span.clone(),
                    format!(
                        "{} name `{name}` contains invalid characters; expected identifier (2-60 chars, letters/digits/underscores)",
                        entity.kind
                    ),
                ).with_help("use a valid identifier: starts with a letter, letters/digits/underscores, 2-60 chars"));
                continue; // Skip vague name check for invalid identifiers
            }

            // W013: vague name — single word under 4 chars or in vague words list
            let lower = name.to_lowercase();
            let is_vague = VAGUE_NAMES.contains(&lower.as_str())
                || (name.len() < 4 && !name.contains('_'));
            if is_vague {
                bag.push(Diagnostic::new(
                    ValidationCode::W013,
                    entity.span.clone(),
                    format!(
                        "{} name `{name}` is too vague to be a meaningful identifier",
                        entity.kind
                    ),
                ).with_help("use a more descriptive name (e.g., `user_authentication` instead of `data`)"));
            }
        }
    }
}

// ── I001: stale proposal (governance) ──────────────────────────────────

fn check_stale_proposals(files: &[SpecFile], bag: &mut DiagnosticBag) {
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::Decision {
                let is_proposed = matches!(
                    entity.fields.get("status"),
                    Some(FieldValue::Enum(s)) if s == "proposed"
                );
                if is_proposed {
                    bag.push(Diagnostic::new(
                        ValidationCode::I001,
                        entity.span.clone(),
                        format!("decision `{}` is still in proposed status", entity.id.raw()),
                    ).with_help("review and update the decision status to accepted or rejected"));
                }
            }
        }
    }
}

// ── I003: newer format available ───────────────────────────────────────

fn check_format_version(config: &CompilerConfig, files: &[SpecFile], bag: &mut DiagnosticBag) {
    if config.version < FormatVersion::CURRENT {
        let span = files
            .iter()
            .flat_map(|f| f.entities.iter())
            .find(|e| e.kind == EntityKind::Spec)
            .map(|e| e.span.clone())
            .unwrap_or_else(|| SourceSpan::file_start("unknown"));
        bag.push(
            Diagnostic::new(
                ValidationCode::I003,
                span,
                format!(
                    "spec format version {} is older than compiler version {}; consider running `specforge migrate`",
                    config.version, FormatVersion::CURRENT
                ),
            ),
        );
    }
}

// ── W018: deliverable with no capabilities (product) ────────────────────

fn check_deliverables_with_no_capabilities(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Deliverable) {
        let outgoing = graph.outgoing_edges(node.id.raw());
        let has_capabilities = outgoing
            .iter()
            .any(|(_, edge)| edge.edge_type == EdgeType::Bundles);
        if !has_capabilities {
            bag.push(
                Diagnostic::new(
                    ValidationCode::W018,
                    node.span.clone(),
                    format!("deliverable `{}` has no capabilities", node.id.raw()),
                )
                .with_help("add capabilities to this deliverable's `capabilities` list"),
            );
        }
    }
}

// ── W019: constraint with no protected invariants (governance) ──────────

fn check_constraints_with_no_protects(graph: &SpecGraph, bag: &mut DiagnosticBag) {
    for node in graph.nodes_of_kind(EntityKind::Constraint) {
        let outgoing = graph.outgoing_edges(node.id.raw());
        let has_protects = outgoing
            .iter()
            .any(|(_, edge)| edge.edge_type == EdgeType::Protects);
        if !has_protects {
            bag.push(
                Diagnostic::new(
                    ValidationCode::W019,
                    node.span.clone(),
                    format!("constraint `{}` protects no invariants", node.id.raw()),
                )
                .with_help("add invariants to this constraint's `protects` list"),
            );
        }
    }
}

// ── I006: unused glossary term (product) ────────────────────────────────

fn check_unused_glossary_terms(files: &[SpecFile], bag: &mut DiagnosticBag) {
    // Find the glossary entity and extract term names
    let glossary = files
        .iter()
        .flat_map(|f| f.entities.iter())
        .find(|e| e.kind == EntityKind::Glossary);

    let glossary = match glossary {
        Some(g) => g,
        None => return,
    };

    let term_names: Vec<&str> = glossary
        .fields
        .iter()
        .filter_map(|(key, _)| key.strip_prefix("term:"))
        .collect();

    if term_names.is_empty() {
        return;
    }

    // Collect all text content from all entities (titles, all string fields, verify
    // descriptions, scenario titles/steps, and nested blocks — recursively).
    let mut all_text = String::new();
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::Glossary {
                continue;
            }
            if let Some(title) = &entity.title {
                all_text.push(' ');
                all_text.push_str(title);
            }
            collect_text_from_fields(&entity.fields, &mut all_text);
        }
    }

    let all_text_lower = all_text.to_lowercase();

    for term in &term_names {
        let term_lower = term.to_lowercase();
        if !all_text_lower.contains(&term_lower) {
            bag.push(
                Diagnostic::new(
                    ValidationCode::I006,
                    glossary.span.clone(),
                    format!("glossary term `{term}` is not referenced in any entity text"),
                )
                .with_help("consider removing unused terms or referencing them in entity descriptions"),
            );
        }
    }
}

/// Recursively collect all text content from a FieldMap into the output buffer.
fn collect_text_from_fields(fields: &FieldMap, out: &mut String) {
    for (_key, value) in fields.iter() {
        match value {
            FieldValue::String(text) => {
                out.push(' ');
                out.push_str(text);
            }
            FieldValue::StringList(items) => {
                for item in items {
                    out.push(' ');
                    out.push_str(item);
                }
            }
            FieldValue::Block(sub_fields) => {
                collect_text_from_fields(sub_fields, out);
            }
            FieldValue::VerifyList(stmts) => {
                for stmt in stmts {
                    out.push(' ');
                    out.push_str(&stmt.description);
                }
            }
            FieldValue::ScenarioList(scenarios) => {
                for scenario in scenarios {
                    out.push(' ');
                    out.push_str(&scenario.title);
                    for step in &scenario.steps {
                        out.push(' ');
                        out.push_str(&step.description);
                    }
                }
            }
            _ => {}
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn get_int_field(entity: &specforge_parser::AstEntity, field: &str) -> Option<i64> {
    match entity.fields.get(field) {
        Some(FieldValue::Integer(n)) => Some(*n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{EntityId, FieldMap, FormatVersion, SourceSpan};
    use specforge_graph::build_graph;
    use specforge_parser::SpecFile;

    fn make_file(path: &str, entities: Vec<specforge_parser::AstEntity>) -> SpecFile {
        SpecFile {
            path: path.to_string(),
            imports: Vec::new(),
            entities,
            errors: Vec::new(),
        }
    }

    fn make_entity(
        id: &str,
        kind: EntityKind,
        fields: FieldMap,
    ) -> specforge_parser::AstEntity {
        specforge_parser::AstEntity {
            kind,
            id: EntityId::parse(id),
            title: Some(format!("Test {id}")),
            fields,
            span: SourceSpan::new("test.spec", 1, 1, 1, 1),
        }
    }

    fn validate_files(files: &[SpecFile], config: &CompilerConfig) -> DiagnosticBag {
        let result = build_graph(files);
        validate(files, &result.graph, config)
    }

    fn diags_with_code(bag: &DiagnosticBag, code: ValidationCode) -> Vec<&Diagnostic> {
        bag.iter().filter(|d| d.code == code).collect()
    }

    // ── W001: orphan behavior ──────────────────────────────────────────

    #[test]
    fn w001_orphan_behavior() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("create_user", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W001).len(), 1);
    }

    #[test]
    fn w001_referenced_behavior_not_orphan() {
        let mut feat_fields = FieldMap::new();
        feat_fields.insert(
            "behaviors",
            FieldValue::ReferenceList(vec![EntityId::parse("create_user")]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("create_user", EntityKind::Behavior, FieldMap::new()),
                make_entity("user_login", EntityKind::Feature, feat_fields),
            ],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W001).len(), 0);
    }

    // ── W003: unused invariant ─────────────────────────────────────────

    #[test]
    fn w003_unused_invariant() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("data_persistence", EntityKind::Invariant, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W003).len(), 1);
    }

    // ── W004: unverified behavior ──────────────────────────────────────

    #[test]
    fn w004_unverified_behavior() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("create_user", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W004).len(), 1);
    }

    #[test]
    fn w004_verified_behavior_ok() {
        let mut fields = FieldMap::new();
        fields.insert(
            "verify",
            FieldValue::VerifyList(vec![specforge_common::VerifyStatement {
                kind: specforge_common::VerifyKind::Unit,
                description: "test it".to_string(),
            }]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("create_user", EntityKind::Behavior, fields)],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W004).len(), 0);
    }

    #[test]
    fn w004_behavior_with_scenario_ok() {
        let mut fields = FieldMap::new();
        fields.insert(
            "scenario",
            FieldValue::ScenarioList(vec![specforge_common::Scenario {
                title: "test flow".to_string(),
                steps: vec![
                    specforge_common::ScenarioStep {
                        kind: specforge_common::ScenarioStepKind::Given,
                        description: "a user".to_string(),
                        span: SourceSpan::new("test.spec", 2, 1, 2, 20),
                    },
                    specforge_common::ScenarioStep {
                        kind: specforge_common::ScenarioStepKind::When,
                        description: "user logs in".to_string(),
                        span: SourceSpan::new("test.spec", 3, 1, 3, 20),
                    },
                    specforge_common::ScenarioStep {
                        kind: specforge_common::ScenarioStepKind::Then,
                        description: "access granted".to_string(),
                        span: SourceSpan::new("test.spec", 4, 1, 4, 20),
                    },
                ],
                span: SourceSpan::new("test.spec", 1, 1, 5, 1),
            }]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("login_flow", EntityKind::Behavior, fields)],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W004).len(), 0);
    }

    // ── E004: empty scenario ──────────────────────────────────────────

    #[test]
    fn e004_empty_scenario() {
        let mut fields = FieldMap::new();
        fields.insert(
            "scenario",
            FieldValue::ScenarioList(vec![specforge_common::Scenario {
                title: "empty flow".to_string(),
                steps: vec![],
                span: SourceSpan::new("test.spec", 1, 1, 1, 20),
            }]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("login_flow", EntityKind::Behavior, fields)],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E004).len(), 1);
    }

    // ── E015: duplicate scenario title ────────────────────────────────

    #[test]
    fn e015_duplicate_scenario_title() {
        let make_scenario = |title: &str| specforge_common::Scenario {
            title: title.to_string(),
            steps: vec![specforge_common::ScenarioStep {
                kind: specforge_common::ScenarioStepKind::When,
                description: "action".to_string(),
                span: SourceSpan::new("test.spec", 2, 1, 2, 20),
            }],
            span: SourceSpan::new("test.spec", 1, 1, 3, 1),
        };
        let mut fields = FieldMap::new();
        fields.insert(
            "scenario",
            FieldValue::ScenarioList(vec![
                make_scenario("same title"),
                make_scenario("same title"),
            ]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("login_flow", EntityKind::Behavior, fields)],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E015).len(), 1);
    }

    // ── W015/W016: scenario missing when/then ─────────────────────────

    #[test]
    fn w015_scenario_missing_when() {
        let mut fields = FieldMap::new();
        fields.insert(
            "scenario",
            FieldValue::ScenarioList(vec![specforge_common::Scenario {
                title: "no when".to_string(),
                steps: vec![
                    specforge_common::ScenarioStep {
                        kind: specforge_common::ScenarioStepKind::Given,
                        description: "setup".to_string(),
                        span: SourceSpan::new("test.spec", 2, 1, 2, 20),
                    },
                    specforge_common::ScenarioStep {
                        kind: specforge_common::ScenarioStepKind::Then,
                        description: "result".to_string(),
                        span: SourceSpan::new("test.spec", 3, 1, 3, 20),
                    },
                ],
                span: SourceSpan::new("test.spec", 1, 1, 4, 1),
            }]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("login_flow", EntityKind::Behavior, fields)],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W015).len(), 1);
        assert_eq!(diags_with_code(&bag, ValidationCode::W016).len(), 0);
    }

    #[test]
    fn w016_scenario_missing_then() {
        let mut fields = FieldMap::new();
        fields.insert(
            "scenario",
            FieldValue::ScenarioList(vec![specforge_common::Scenario {
                title: "no then".to_string(),
                steps: vec![
                    specforge_common::ScenarioStep {
                        kind: specforge_common::ScenarioStepKind::Given,
                        description: "setup".to_string(),
                        span: SourceSpan::new("test.spec", 2, 1, 2, 20),
                    },
                    specforge_common::ScenarioStep {
                        kind: specforge_common::ScenarioStepKind::When,
                        description: "action".to_string(),
                        span: SourceSpan::new("test.spec", 3, 1, 3, 20),
                    },
                ],
                span: SourceSpan::new("test.spec", 1, 1, 4, 1),
            }]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("login_flow", EntityKind::Behavior, fields)],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W015).len(), 0);
        assert_eq!(diags_with_code(&bag, ValidationCode::W016).len(), 1);
    }

    // ── W007: orphan event ─────────────────────────────────────────────

    #[test]
    fn w007_orphan_event() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("user_created", EntityKind::Event, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W007).len(), 1);
    }

    // ── E005: RPN mismatch (governance) ────────────────────────────────

    #[test]
    fn e005_rpn_mismatch() {
        let mut fields = FieldMap::new();
        fields.insert("severity", FieldValue::Integer(5));
        fields.insert("occurrence", FieldValue::Integer(3));
        fields.insert("detection", FieldValue::Integer(2));
        fields.insert("rpn", FieldValue::Integer(99)); // Should be 30

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("database_failure", EntityKind::FailureMode, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::E005).len(), 1);
    }

    #[test]
    fn e005_rpn_correct() {
        let mut fields = FieldMap::new();
        fields.insert("severity", FieldValue::Integer(5));
        fields.insert("occurrence", FieldValue::Integer(3));
        fields.insert("detection", FieldValue::Integer(2));
        fields.insert("rpn", FieldValue::Integer(30));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("database_failure", EntityKind::FailureMode, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::E005).len(), 0);
    }

    // ── E006: invalid event trigger ────────────────────────────────────

    #[test]
    fn e006_event_trigger_not_behavior() {
        let mut event_fields = FieldMap::new();
        event_fields.insert("trigger", FieldValue::Reference(EntityId::parse("data_persistence")));

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("data_persistence", EntityKind::Invariant, FieldMap::new()),
                make_entity("user_created", EntityKind::Event, event_fields),
            ],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E006).len(), 1);
    }

    #[test]
    fn e006_event_trigger_is_behavior_ok() {
        let mut event_fields = FieldMap::new();
        event_fields.insert("trigger", FieldValue::Reference(EntityId::parse("create_user")));

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("create_user", EntityKind::Behavior, FieldMap::new()),
                make_entity("user_created", EntityKind::Event, event_fields),
            ],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E006).len(), 0);
    }

    // ── E008: persona not defined (product) ────────────────────────────

    #[test]
    fn e008_persona_not_defined() {
        let mut fields = FieldMap::new();
        fields.insert("persona", FieldValue::Enum("admin".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("manage_accounts", EntityKind::Capability, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        // No personas defined
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::E008).len(), 1);
    }

    #[test]
    fn e008_persona_defined_ok() {
        let mut fields = FieldMap::new();
        fields.insert("persona", FieldValue::Enum("developer".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("manage_accounts", EntityKind::Capability, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        config.personas.push(("developer".to_string(), "Developer".to_string()));
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::E008).len(), 0);
    }

    // ── E009: surface not defined (product) ────────────────────────────

    #[test]
    fn e009_surface_not_defined() {
        let mut fields = FieldMap::new();
        fields.insert("surface", FieldValue::Enum("web".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("manage_accounts", EntityKind::Capability, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::E009).len(), 1);
    }

    // ── I001: stale proposal (governance) ──────────────────────────────

    #[test]
    fn i001_stale_proposal() {
        let mut fields = FieldMap::new();
        fields.insert("status", FieldValue::Enum("proposed".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("use_postgres", EntityKind::Decision, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::I001).len(), 1);
    }

    #[test]
    fn i001_accepted_decision_ok() {
        let mut fields = FieldMap::new();
        fields.insert("status", FieldValue::Enum("accepted".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("use_postgres", EntityKind::Decision, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::I001).len(), 0);
    }

    // ── Plugin-conditional passes ──────────────────────────────────────

    #[test]
    fn product_passes_skipped_without_plugin() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("user_login", EntityKind::Feature, FieldMap::new())],
        )];
        // No product plugin installed — W002 should not fire
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W002).len(), 0);
    }

    #[test]
    fn governance_passes_skipped_without_plugin() {
        let mut fields = FieldMap::new();
        fields.insert("status", FieldValue::Enum("proposed".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("use_postgres", EntityKind::Decision, fields)],
        )];
        // No governance plugin — I001 should not fire
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::I001).len(), 0);
    }

    // ── W002: orphan feature (product) ────────────────────────────────

    #[test]
    fn w002_orphan_feature() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("user_login", EntityKind::Feature, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W002).len(), 1);
    }

    // ── W005: unmitigated high-risk invariant (governance) ──────────

    #[test]
    fn w005_unmitigated_high_risk() {
        let mut fields = FieldMap::new();
        fields.insert("risk", FieldValue::Enum("high".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("data_persistence", EntityKind::Invariant, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W005).len(), 1);
    }

    // ── W006: unconstrained behavior (governance) ───────────────────

    #[test]
    fn w006_unconstrained_behavior() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("create_user", EntityKind::Behavior, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W006).len(), 1);
    }

    // ── W008: uncovered capability (product) ────────────────────────

    #[test]
    fn w008_uncovered_capability() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("manage_accounts", EntityKind::Capability, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W008).len(), 1);
    }

    // ── W009: orphan library (product) ──────────────────────────────

    #[test]
    fn w009_orphan_library() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("auth_core", EntityKind::Library, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W009).len(), 1);
    }

    // ── W010: deprecated feature (product) ──────────────────────────

    #[test]
    fn w010_deprecated_feature() {
        let mut fields = FieldMap::new();
        fields.insert("status", FieldValue::Enum("deprecated".to_string()));

        let files = vec![make_file(
            "test.spec",
            vec![make_entity("user_login", EntityKind::Feature, fields)],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W010).len(), 1);
    }

    // ── W011: orphan capability (product) ───────────────────────────

    #[test]
    fn w011_orphan_capability() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("manage_accounts", EntityKind::Capability, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W011).len(), 1);
    }

    // ── W012: orphan ref ────────────────────────────────────────────

    #[test]
    fn w012_orphan_ref() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("gh.issue:42", EntityKind::Ref, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W012).len(), 1);
    }

    // ── E007: circular library dependency (product) ─────────────────

    #[test]
    fn e007_library_cycle() {
        let mut lib_a_fields = FieldMap::new();
        lib_a_fields.insert(
            "depends_on",
            FieldValue::ReferenceList(vec![EntityId::parse("auth_utils")]),
        );
        let mut lib_b_fields = FieldMap::new();
        lib_b_fields.insert(
            "depends_on",
            FieldValue::ReferenceList(vec![EntityId::parse("auth_core")]),
        );

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("auth_core", EntityKind::Library, lib_a_fields),
                make_entity("auth_utils", EntityKind::Library, lib_b_fields),
            ],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert!(diags_with_code(&bag, ValidationCode::E007).len() >= 1);
    }

    // ── I003: older format version ───────────────────────────────────

    #[test]
    fn i003_older_version_emits_diagnostic() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("data_persistence", EntityKind::Invariant, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.version = FormatVersion::new(0, 9);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::I003).len(), 1);
    }

    #[test]
    fn i003_current_version_no_diagnostic() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("data_persistence", EntityKind::Invariant, FieldMap::new())],
        )];
        let config = CompilerConfig::default(); // version is CURRENT
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::I003).len(), 0);
    }

    #[test]
    fn i003_older_version_still_compiles() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("data_persistence", EntityKind::Invariant, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.version = FormatVersion::new(0, 9);
        let bag = validate_files(&files, &config);
        // Has I003 info but no errors
        assert_eq!(bag.error_count(), 0);
        assert_eq!(diags_with_code(&bag, ValidationCode::I003).len(), 1);
    }

    // ── Clean file produces no errors ──────────────────────────────────

    #[test]
    fn clean_file_no_errors() {
        let mut beh_fields = FieldMap::new();
        beh_fields.insert(
            "invariants",
            FieldValue::ReferenceList(vec![EntityId::parse("data_persistence")]),
        );
        beh_fields.insert("contract", FieldValue::String("must work".to_string()));
        beh_fields.insert(
            "verify",
            FieldValue::VerifyList(vec![specforge_common::VerifyStatement {
                kind: specforge_common::VerifyKind::Unit,
                description: "test it".to_string(),
            }]),
        );

        let mut feat_fields = FieldMap::new();
        feat_fields.insert(
            "behaviors",
            FieldValue::ReferenceList(vec![EntityId::parse("create_user")]),
        );

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("data_persistence", EntityKind::Invariant, FieldMap::new()),
                make_entity("create_user", EntityKind::Behavior, beh_fields),
                make_entity("user_login", EntityKind::Feature, feat_fields),
            ],
        )];

        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(bag.error_count(), 0);
    }

    // ── E013: reserved word used as identifier ──────────────────────

    #[test]
    fn e013_reserved_word() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("behavior", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E013).len(), 1);
    }

    #[test]
    fn e013_non_reserved_word_ok() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("auth_login", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E013).len(), 0);
    }

    // ── E014: invalid identifier characters ─────────────────────────

    #[test]
    fn e014_invalid_chars() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("my-behavior!", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E014).len(), 1);
    }

    #[test]
    fn e014_valid_snake_case_ok() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("auth_login", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E014).len(), 0);
    }

    // ── W013: vague entity name ─────────────────────────────────────

    #[test]
    fn w013_vague_name() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("data", EntityKind::Invariant, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W013).len(), 1);
    }

    #[test]
    fn w013_short_name() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("ab", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W013).len(), 1);
    }

    #[test]
    fn w013_descriptive_name_ok() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("user_authentication", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W013).len(), 0);
    }

    // ── Any case on any entity is valid ───────────────────────────

    #[test]
    fn any_case_on_any_entity_ok() {
        // PascalCase behavior and snake_case typedef — both valid
        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("CreateUser", EntityKind::Behavior, FieldMap::new()),
                make_entity("user_type", EntityKind::TypeDef, FieldMap::new()),
            ],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E014).len(), 0);
    }

    #[test]
    fn camel_case_identifier_valid() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("myBehavior", EntityKind::Behavior, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::E014).len(), 0);
    }

    // ── Naming: skips singletons and refs ───────────────────────────

    #[test]
    fn naming_skips_ref_entities() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("gh.issue:42", EntityKind::Ref, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        // No naming diagnostics for refs
        assert_eq!(diags_with_code(&bag, ValidationCode::E013).len(), 0);
        assert_eq!(diags_with_code(&bag, ValidationCode::E014).len(), 0);
        assert_eq!(diags_with_code(&bag, ValidationCode::W013).len(), 0);
    }

    // ── W017: unused entity (generic) ────────────────────────────────

    #[test]
    fn w017_unused_type() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("UserProfile", EntityKind::TypeDef, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W017).len(), 1);
    }

    #[test]
    fn w017_type_used_by_behavior_ok() {
        let mut beh_fields = FieldMap::new();
        beh_fields.insert(
            "types",
            FieldValue::ReferenceList(vec![EntityId::parse("UserProfile")]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("UserProfile", EntityKind::TypeDef, FieldMap::new()),
                make_entity("create_user", EntityKind::Behavior, beh_fields),
            ],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W017).len(), 0);
    }

    #[test]
    fn w017_unused_port() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("UserRepository", EntityKind::Port, FieldMap::new())],
        )];
        let bag = validate_files(&files, &CompilerConfig::default());
        assert_eq!(diags_with_code(&bag, ValidationCode::W017).len(), 1);
    }

    #[test]
    fn w017_constraint_with_constrains_ok() {
        let mut con_fields = FieldMap::new();
        con_fields.insert(
            "constrains",
            FieldValue::ReferenceList(vec![EntityId::parse("some_behavior")]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("some_behavior", EntityKind::Behavior, FieldMap::new()),
                make_entity("perf_constraint", EntityKind::Constraint, con_fields),
            ],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W017).len(), 0);
    }

    // ── W018: deliverable with no capabilities (product) ─────────────

    #[test]
    fn w018_deliverable_with_no_capabilities() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("cli_app", EntityKind::Deliverable, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W018).len(), 1);
    }

    #[test]
    fn w018_deliverable_with_capabilities_ok() {
        let mut dlv_fields = FieldMap::new();
        dlv_fields.insert(
            "capabilities",
            FieldValue::ReferenceList(vec![EntityId::parse("manage_accounts")]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("manage_accounts", EntityKind::Capability, FieldMap::new()),
                make_entity("cli_app", EntityKind::Deliverable, dlv_fields),
            ],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W018).len(), 0);
    }

    // ── W019: constraint with no protected invariants (governance) ────

    #[test]
    fn w019_constraint_no_protects() {
        let files = vec![make_file(
            "test.spec",
            vec![make_entity("response_time", EntityKind::Constraint, FieldMap::new())],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W019).len(), 1);
    }

    #[test]
    fn w019_constraint_with_protects_ok() {
        let mut con_fields = FieldMap::new();
        con_fields.insert(
            "protects",
            FieldValue::ReferenceList(vec![EntityId::parse("data_integrity")]),
        );
        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("data_integrity", EntityKind::Invariant, FieldMap::new()),
                make_entity("response_time", EntityKind::Constraint, con_fields),
            ],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Governance);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::W019).len(), 0);
    }

    // ── I006: unused glossary term (product) ─────────────────────────

    #[test]
    fn i006_unused_glossary_term() {
        let mut glossary_fields = FieldMap::new();
        let mut term_fields = FieldMap::new();
        term_fields.insert("definition", FieldValue::String("A named block".to_string()));
        glossary_fields.insert("term:entity", FieldValue::Block(term_fields));

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("glossary", EntityKind::Glossary, glossary_fields),
                make_entity("create_user", EntityKind::Behavior, FieldMap::new()),
            ],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::I006).len(), 1);
    }

    #[test]
    fn i006_referenced_glossary_term_ok() {
        let mut glossary_fields = FieldMap::new();
        let mut term_fields = FieldMap::new();
        term_fields.insert("definition", FieldValue::String("A named block".to_string()));
        glossary_fields.insert("term:entity", FieldValue::Block(term_fields));

        let mut beh_fields = FieldMap::new();
        beh_fields.insert(
            "description",
            FieldValue::String("Creates a new entity in the system".to_string()),
        );

        let files = vec![make_file(
            "test.spec",
            vec![
                make_entity("glossary", EntityKind::Glossary, glossary_fields),
                make_entity("create_user", EntityKind::Behavior, beh_fields),
            ],
        )];
        let mut config = CompilerConfig::default();
        config.plugins.push(Module::Product);
        let bag = validate_files(&files, &config);
        assert_eq!(diags_with_code(&bag, ValidationCode::I006).len(), 0);
    }
}
