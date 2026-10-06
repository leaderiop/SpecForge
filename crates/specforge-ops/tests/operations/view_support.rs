//! Projects for the read operations' tests: a graph parsed from source,
//! registries declaring what made-up extensions declare, and a root
//! directory to record a test report in.

use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_ops::plan::PlanOutcome;
use specforge_ops::stats::{Stats, StatsRequest};
use specforge_ops::trace::{Target, TraceChain};
use specforge_ops::view::ProjectView;
use specforge_project::coverage::RecordedCoverage;
use specforge_registry::validation_engine::{ValidationPatternKind, ValidationRulePattern};
use specforge_registry::{FieldRegistryEntry, KindRegistryEntry, ManifestFieldType, RegistryBuild};
use tempfile::TempDir;

/// A kind `@t/soft` declares.
pub fn kind(name: &str, testable: bool) -> KindRegistryEntry {
    KindRegistryEntry {
        kind_name: name.to_string(),
        source_extension: "@t/soft".into(),
        testable,
        supports_verify: testable,
        allowed_verify_kinds: Vec::new(),
        lifecycle_field: None,
        ..Default::default()
    }
}

/// The W004 rule requiring `kind`'s entities to declare obligations.
pub fn obligations_rule(kind: &str) -> (ValidationRulePattern, String) {
    (
        ValidationRulePattern {
            code: "W004".into(),
            severity: specforge_common::Severity::Warning,
            message_template: "{kind} '{id}' is testable but declares no verify obligations".into(),
            check: ValidationPatternKind::NoVerifyStatements,
            target_kind: Some(kind.to_string()),
            edge_type: None,
            edge_peer_kind: None,
            field: Some("verify".into()),
            constraint: None,
            wasm_function: None,
        },
        "@t/soft".into(),
    )
}

/// A boolean `kind.field` that, set, exempts the entity from obligations
/// (as `@specforge/formal` declares `abstract`).
pub fn exempting_field(kind: &str, field: &str) -> FieldRegistryEntry {
    FieldRegistryEntry {
        kind_name: kind.to_string(),
        field_type: ManifestFieldType::Bool,
        source_extension: "@t/formal".into(),
        proof_role: None,
        declared: specforge_registry::FieldDescriptor {
            name: field.to_string(),
            exempts_obligations: true,
            ..Default::default()
        },
    }
}

/// Registries whose `obligated` kinds are testable and must declare
/// obligations (W004 targets each), and whose `free` kinds are testable
/// with no such rule (governance kinds).
pub fn registries(obligated: &[&str], free: &[&str]) -> RegistryBuild {
    let mut build = RegistryBuild::default();
    for name in obligated {
        build.kinds.register(kind(name, true));
        build.rules.push(obligations_rule(name));
    }
    for name in free {
        build.kinds.register(kind(name, true));
    }
    build
}

/// A project on disk: its graph, registries and coverage memo.
pub struct Project {
    pub dir: TempDir,
    pub graph: Graph,
    pub registries: RegistryBuild,
    pub recorded: RecordedCoverage,
}

impl Project {
    /// The project whose one file is `source`.
    pub fn new(source: &str, registries: RegistryBuild) -> Self {
        let (graph, _) =
            specforge_graph::build_graph(&[specforge_parser::parse(source, "main.spec")]);
        Self::of_graph(graph, registries)
    }

    pub fn of_graph(graph: Graph, registries: RegistryBuild) -> Self {
        Project {
            dir: TempDir::new().unwrap(),
            graph,
            registries,
            recorded: RecordedCoverage::default(),
        }
    }

    pub fn view(&self) -> ProjectView<'_> {
        ProjectView::new(
            &self.graph,
            &self.registries,
            Some(self.dir.path()),
            &self.recorded,
        )
    }

    /// Record, as `specforge collect` would, one passing test per
    /// `(entity, obligation)`.
    pub fn record_passing(&self, proofs: &[(&str, &str)]) {
        let mut results = serde_json::Map::new();
        for (entity, obligation) in proofs {
            results.insert(
                entity.to_string(),
                serde_json::json!({"tests": [
                    {"name": format!("{entity}_test"), "status": "pass", "verify": obligation}
                ]}),
            );
        }
        std::fs::write(
            self.dir.path().join("specforge-report.json"),
            serde_json::json!({"runner": "fixture", "results": results}).to_string(),
        )
        .unwrap();
    }
}

/// The stats of `graph`, whose `testable` kinds are testable and must
/// declare obligations, with no recorded report, reporting `diagnostics`.
pub fn stats_of(graph: &Graph, testable: &[&str], diagnostics: &[Diagnostic]) -> Stats {
    let project = Project::of_graph(graph.clone(), registries(testable, &[]));
    specforge_ops::stats::stats(&project.view(), &StatsRequest { diagnostics }).unwrap()
}

/// `plan` checked against `graph`, whose `testable` kinds are testable and
/// must declare obligations.
pub fn plan_check(graph: &Graph, testable: &[&str], plan: &serde_json::Value) -> PlanOutcome {
    let project = Project::of_graph(graph.clone(), registries(testable, &[]));
    specforge_ops::plan::check(&project.view(), plan).unwrap()
}

/// The chain of `entity_id` in `graph`, its expected edges from
/// `registries`; `None` when the graph lacks it.
pub fn chain_in(graph: &Graph, registries: RegistryBuild, entity_id: &str) -> Option<TraceChain> {
    let project = Project::of_graph(graph.clone(), registries);
    specforge_ops::trace::trace(&project.view(), Target::Entity(entity_id))
        .ok()
        .map(|outcome| outcome.chains.into_iter().next().unwrap())
}

/// The chain of `entity_id` in `graph`, with no extension: nothing is
/// expected, so nothing is missing.
pub fn chain(graph: &Graph, entity_id: &str) -> TraceChain {
    chain_in(graph, RegistryBuild::default(), entity_id).unwrap()
}

/// Every entity's chain in `graph`, with `registries`, as the trace
/// operation returns them.
pub fn every_chain(graph: &Graph, registries: RegistryBuild) -> specforge_ops::trace::TraceOutcome {
    let project = Project::of_graph(graph.clone(), registries);
    specforge_ops::trace::trace(&project.view(), Target::Every).unwrap()
}
