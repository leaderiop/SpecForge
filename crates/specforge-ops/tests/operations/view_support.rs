//! Projects for the read operations' tests: a graph parsed from source,
//! registries declaring what made-up extensions declare, and a root
//! directory to record a test report in.

use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_ops::plan::{PlanGapKind, PlanOutcome};
use specforge_ops::stats::Stats;
use specforge_ops::trace::{Target, TraceChain};
use specforge_ops::view::ProjectView;
use specforge_project::Environment;
use specforge_project::coverage::RecordedCoverage;
use specforge_protocol_types::{
    ExtensionDeclaration, ValidationRuleDescriptor, ValidationSeverity,
};
use specforge_registry::rules::{Registries, Rules};
use specforge_registry::{FieldRegistryEntry, KindRegistryEntry, RegistryBuild};
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

/// The W004 rule requiring `kind`'s entities to declare obligations, as
/// `@t/soft` declares it.
pub fn obligations_rule(kind: &str) -> ValidationRuleDescriptor {
    ValidationRuleDescriptor {
        code: "W004".into(),
        severity: ValidationSeverity::Warning,
        message_template: "{kind} '{id}' is testable but declares no verify obligations".into(),
        check: "no_verify_statements".into(),
        target_kind: Some(kind.to_string()),
        field: Some("verify".into()),
        ..Default::default()
    }
}

/// A boolean `kind.field` that, set, exempts the entity from obligations
/// (as `@specforge/formal` declares `abstract`).
pub fn exempting_field(kind: &str, field: &str) -> FieldRegistryEntry {
    FieldRegistryEntry::new(
        kind,
        "@t/formal",
        specforge_registry::FieldDescriptor {
            name: field.to_string(),
            field_type: "bool".to_string(),
            exempts_obligations: true,
            ..Default::default()
        },
    )
    .unwrap()
}

/// Registries whose `obligated` kinds are testable and must declare
/// obligations (W004 targets each), and whose `free` kinds are testable
/// with no such rule (governance kinds).
pub fn registries(obligated: &[&str], free: &[&str]) -> RegistryBuild {
    let mut build = RegistryBuild::default();
    for name in obligated.iter().chain(free) {
        build.kinds.register(kind(name, true));
    }
    // The rule set the testing extension's W004 rules make (it owns W004),
    // through the rules' build.
    let mut soft = ExtensionDeclaration::default();
    soft.handshake.name = "@specforge/testing".into();
    soft.validation_rules = obligated
        .iter()
        .map(|name| obligations_rule(name))
        .collect();
    let (rules, diagnostics) = Rules::build(
        &[soft],
        Registries {
            kinds: &build.kinds,
            fields: &build.fields,
            edges: &build.edges,
        },
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    build.rules = rules;
    build
}

/// A project on disk: its graph, the environment it was compiled in (the
/// default config and `registries`) and its coverage memo.
pub struct Project {
    pub dir: TempDir,
    pub graph: Graph,
    pub env: Environment,
    /// Made over the graph and environment as they are at the first view.
    recorded: std::sync::OnceLock<RecordedCoverage>,
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
            env: Environment::with_registries(registries),
            recorded: std::sync::OnceLock::new(),
        }
    }

    pub fn view(&self) -> ProjectView<'_> {
        ProjectView::new(
            &self.graph,
            &self.env,
            Some(self.dir.path()),
            self.recorded
                .get_or_init(|| RecordedCoverage::over(&self.graph, &self.env)),
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
    specforge_ops::stats::stats(&project.view().reporting(diagnostics)).unwrap()
}

/// `plan` checked against `graph`, whose `testable` kinds are testable and
/// must declare obligations.
pub fn plan_check(graph: &Graph, testable: &[&str], plan: &serde_json::Value) -> PlanOutcome {
    let project = Project::of_graph(graph.clone(), registries(testable, &[]));
    specforge_ops::plan::check(&project.view(), plan).unwrap()
}

/// The contexts of `outcome`'s gaps of `kind`, in order.
pub fn gap_texts(outcome: &PlanOutcome, kind: PlanGapKind) -> Vec<&str> {
    outcome
        .gaps
        .iter()
        .filter(|gap| gap.kind == kind)
        .map(|gap| gap.context.as_str())
        .collect()
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
