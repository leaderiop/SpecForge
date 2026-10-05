//! The host's coverage vocabulary: which kinds are testable, what an
//! entity's obligations are, and how the recorded test report is read.
//!
//! Every surface that asks "does this entity count toward coverage" or
//! "what does it promise to prove" (stats, plan validation, the context
//! exports, the MCP coverage, inspect, review and trace views) reads it
//! here, so they cannot disagree.

use crate::compile::build_validation_entities;
use serde::Deserialize;
use specforge_common::Diagnostic;
use specforge_graph::{FieldValue, Graph, Node};
use specforge_parser::UNION_VARIANTS_FIELD;
use specforge_registry::validation_engine::{
    ValidationEntity, ValidationPatternKind, ValidationRulePattern,
};
use specforge_registry::{FieldRegistry, KindRegistry};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The recorded test report, at the project root: `collect` writes it,
/// `analyze` and the coverage views read it.
pub const REPORT_FILE: &str = "specforge-report.json";

/// The recorded test report (`specforge-report.json`, RES-15).
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TestReport {
    #[serde(default)]
    pub runner: Option<String>,
    #[serde(default)]
    pub results: std::collections::BTreeMap<String, ReportedEntity>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct ReportedEntity {
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub tests: Vec<ReportedTest>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct ReportedTest {
    #[serde(default)]
    pub name: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    /// The `verify` obligation the test proves, when it says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
    /// The collector that recorded the test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<String>,
}

pub use specforge_coverage::{Status, Summary, Verdict};

/// The report as a pass receives it (`PassInput::test_results`): per entity
/// id, each test's name, status and the obligation it proves.
impl From<&TestReport> for specforge_protocol_types::PassTestResults {
    fn from(report: &TestReport) -> Self {
        use specforge_protocol_types::{PassEntityResults, PassTestResult};
        specforge_protocol_types::PassTestResults {
            runner: report.runner.clone(),
            results: report
                .results
                .iter()
                .map(|(id, entity)| {
                    let tests = entity
                        .tests
                        .iter()
                        .map(|test| PassTestResult {
                            name: test.name.clone(),
                            status: test.status.clone(),
                            verify: test.verify.clone(),
                        })
                        .collect();
                    (id.clone(), PassEntityResults { tests })
                })
                .collect(),
        }
    }
}

/// The kinds that count toward coverage: those an extension's manifest
/// declares `testable`. Nothing is testable by default, and accepting
/// `verify` statements (`supports_verify`) does not make a kind testable.
pub fn testable_kinds(reg: &KindRegistry) -> BTreeSet<&str> {
    reg.iter()
        .filter(|(_, kind)| kind.testable)
        .map(|(name, _)| name.as_str())
        .collect()
}

/// Why a test report could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    /// The report is there (or was named) but could not be read.
    Unreadable {
        path: PathBuf,
        detail: String,
        /// The named file does not exist.
        missing: bool,
    },
    /// The report does not parse as a `specforge-report.json`.
    Malformed { path: PathBuf, detail: String },
}

impl ReportError {
    pub fn path(&self) -> &Path {
        match self {
            ReportError::Unreadable { path, .. } | ReportError::Malformed { path, .. } => path,
        }
    }

    /// The error as a diagnostic (E045, an invalid test report).
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::error("E045", self.to_string()).with_suggestion(
            "run `specforge collect` again to rewrite the report, or fix or remove the file",
        )
    }
}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportError::Unreadable { path, detail, .. } => {
                write!(f, "cannot read test results {}: {detail}", path.display())
            }
            ReportError::Malformed { path, detail } => write!(
                f,
                "invalid test results {}: {detail} (expected the RES-15 specforge-report.json shape)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ReportError {}

/// The project's recorded test results (`specforge-report.json` at `root`,
/// written by `specforge collect`). No report means no recorded tests
/// (`Ok(None)`); a report that is there but unreadable or malformed is an
/// error, never read as empty, so coverage cannot silently drop.
pub fn read_report(root: &Path) -> Result<Option<TestReport>, ReportError> {
    let path = root.join(REPORT_FILE);
    if !path.exists() {
        return Ok(None);
    }
    read_report_file(&path).map(Some)
}

/// A test report at an explicit path (`--test-results`), which must exist.
pub fn read_report_file(path: &Path) -> Result<TestReport, ReportError> {
    let raw = std::fs::read_to_string(path).map_err(|e| ReportError::Unreadable {
        path: path.to_path_buf(),
        detail: e.to_string(),
        missing: e.kind() == std::io::ErrorKind::NotFound,
    })?;
    serde_json::from_str(&raw).map_err(|e| ReportError::Malformed {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })
}

/// Whether an entity owes no obligations of its own, whatever it declares
/// (ADR 0004, D2-b): a union (`type X = A | B`, which has no body to hold
/// them), or an entity that sets a field its kind's registry entry declares
/// `exempts_obligations` (as `@specforge/formal` declares `abstract true`).
/// Decided from the entity's structure and the field registry, never from
/// field names: a struct member that only happens to be named `abstract`
/// exempts nothing.
pub fn obligation_exempt(node: &Node, fields: &FieldRegistry) -> bool {
    let kind = node.kind.raw.as_str();
    node.fields
        .entries()
        .iter()
        .any(|entry| match &entry.value {
            // The union syntax is structural: its body is the variant list,
            // under the parser's own key (a user's `values [a, b]` is a
            // variant list too, and exempts nothing).
            FieldValue::VariantList(variants) if entry.key.as_str() == UNION_VARIANTS_FIELD => {
                !variants.is_empty()
            }
            value => {
                is_set(value)
                    && fields
                        .get(kind, entry.key.as_str())
                        .is_some_and(|f| f.declared.exempts_obligations)
            }
        })
}

/// A field value that turns an exempting flag on: `true`, or any value
/// that is not empty.
fn is_set(value: &FieldValue) -> bool {
    match value {
        FieldValue::Boolean(b) => *b,
        FieldValue::String(s) | FieldValue::Identifier(s) => !s.is_empty(),
        FieldValue::StringList(list) => !list.is_empty(),
        FieldValue::ReferenceList(refs) => !refs.is_empty(),
        _ => false,
    }
}

/// The kinds whose entities must declare obligations: those a
/// `no_verify_statements` rule (W004) targets. A testable kind no such rule
/// targets (a governance `constraint` or `failure_mode`) need not declare
/// any, so its entities that declare none are exempt.
pub fn obligated_kinds(rules: &[(ValidationRulePattern, String)]) -> BTreeSet<&str> {
    rules
        .iter()
        .filter(|(rule, _)| rule.check == ValidationPatternKind::NoVerifyStatements)
        .filter_map(|(rule, _)| rule.target_kind.as_deref())
        .collect()
}

/// What decides how the coverage rule sees each entity: which kinds are
/// testable, which must declare obligations, and which fields exempt.
#[derive(Clone, Copy)]
pub struct CoverageRegistries<'a> {
    pub kinds: &'a KindRegistry,
    pub fields: &'a FieldRegistry,
    pub rules: &'a [(ValidationRulePattern, String)],
}

impl CoverageRegistries<'_> {
    /// The rule's view of every entity in `graph`, alongside the snapshot
    /// it was taken from (what the extension passes receive).
    pub fn entities(&self, graph: &Graph) -> Vec<(ValidationEntity, specforge_coverage::Entity)> {
        let testable = testable_kinds(self.kinds);
        let obligated = obligated_kinds(self.rules);
        build_validation_entities(graph, self.fields)
            .into_iter()
            .map(|e| {
                let entity = rule_entity(
                    &e,
                    testable.contains(e.kind.as_str()),
                    obligated.contains(e.kind.as_str()),
                );
                (e, entity)
            })
            .collect()
    }
}

/// An entity as the coverage rule (`specforge-coverage`) sees it: the same
/// facts the host hands the `@specforge/testing:coverage` pass, so a
/// per-entity view and the pass cannot disagree. `obligated`: its kind must
/// declare obligations ([`obligated_kinds`]).
pub fn rule_entity(
    entity: &ValidationEntity,
    testable: bool,
    obligated: bool,
) -> specforge_coverage::Entity {
    specforge_coverage::Entity {
        id: entity.id.clone(),
        kind: entity.kind.clone(),
        testable,
        exempt: entity.obligation_exempt || !obligated,
        verify_kinds: entity.verify_kinds.clone(),
        verify_texts: entity.verify_texts.clone(),
        // The host grades no kind by risk (ADR 0009, B): the testing
        // pass reads risk for the kind it grades.
        risk: None,
        referenced: entity.incoming_edge_count > 0,
    }
}

/// A report's recorded tests, per entity id, as the rule reads them.
pub fn recorded_tests(report: &TestReport) -> specforge_coverage::TestResults {
    specforge_coverage::TestResults {
        runner: report.runner.clone(),
        entities: report
            .results
            .iter()
            .map(|(id, entity)| {
                let tests = entity
                    .tests
                    .iter()
                    .map(|t| specforge_coverage::RecordedTest {
                        name: t.name.clone(),
                        status: t.status.clone(),
                        verify: t.verify.clone(),
                    })
                    .collect();
                (id.clone(), tests)
            })
            .collect(),
    }
}

/// A project's coverage, computed by the one rule the `coverage` pass
/// applies (ADR 0004, D2-f). Per-entity views (the MCP coverage, inspect,
/// query and review surfaces) and stats read it, so none of them
/// re-derives "testable", "proven" or "covered".
///
/// Formal discharge needs the prove pass, which a per-entity view does not
/// run: as `analyze coverage` without `--prove`, a `verify property`
/// obligation is proven only by a passing test. The host grades no kind by
/// risk (ADR 0009, B): its summary has no risk tallies and its findings no
/// A002; the risk grading is `@specforge/testing`'s.
#[derive(Debug, Clone, Default)]
pub struct ProjectCoverage {
    /// Per entity id, for every entity in the graph.
    pub verdicts: BTreeMap<String, Verdict>,
    /// The summary the `coverage` pass reports for the same inputs, less
    /// what its risk grading adds (the risk tallies and enforcement counts).
    pub summary: Summary,
}

impl ProjectCoverage {
    /// Score `graph` against its recorded tests (`None` without a report).
    pub fn compute(
        graph: &Graph,
        registries: CoverageRegistries<'_>,
        report: Option<&TestReport>,
    ) -> Self {
        let entities: Vec<specforge_coverage::Entity> = registries
            .entities(graph)
            .into_iter()
            .map(|(_, entity)| entity)
            .collect();
        Self::assess(&entities, report)
    }

    /// Score `graph` knowing only which kinds are testable: every testable
    /// kind must declare obligations, and only structure exempts (a union
    /// type). For callers without the project's registries.
    pub fn with_testable_kinds(
        graph: &Graph,
        testable_kinds: &[&str],
        report: Option<&TestReport>,
    ) -> Self {
        let entities: Vec<specforge_coverage::Entity> =
            build_validation_entities(graph, &FieldRegistry::new())
                .iter()
                .map(|e| {
                    let testable = testable_kinds.contains(&e.kind.as_str());
                    rule_entity(e, testable, testable)
                })
                .collect();
        Self::assess(&entities, report)
    }

    fn assess(entities: &[specforge_coverage::Entity], report: Option<&TestReport>) -> Self {
        let results = report.map(recorded_tests);
        let assessment = specforge_coverage::assess(entities, results.as_ref(), None, None);
        ProjectCoverage {
            verdicts: assessment.verdicts,
            summary: assessment.summary,
        }
    }

    /// The entity's verdict, if the graph has it.
    pub fn verdict(&self, id: &str) -> Option<&Verdict> {
        self.verdicts.get(id)
    }

    /// The entity's status; an entity the graph doesn't have is uncovered.
    pub fn status(&self, id: &str) -> Status {
        self.verdict(id).map_or(Status::Uncovered, Verdict::status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_registry::KindRegistryEntry;
    use specforge_test_macros::test as specforge_test;

    fn kind(name: &str, testable: bool, supports_verify: bool) -> KindRegistryEntry {
        KindRegistryEntry {
            kind_name: name.into(),
            source_extension: "@test/ext".into(),
            testable,
            supports_verify,
            allowed_verify_kinds: Vec::new(),
            lifecycle_field: None,
            ..Default::default()
        }
    }

    fn graph_of(source: &str) -> Graph {
        let (graph, _) =
            specforge_graph::build_graph(&[specforge_parser::parse(source, "test.spec")]);
        graph
    }

    /// The field registry of a project whose `behavior` kind declares the
    /// `abstract` flag (as @specforge/formal does).
    fn abstract_behaviors() -> FieldRegistry {
        let mut fields = FieldRegistry::new();
        fields.register(specforge_registry::FieldRegistryEntry {
            kind_name: "behavior".into(),
            field_type: specforge_registry::ManifestFieldType::Bool,
            source_extension: "@test/formal".into(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "abstract".into(),
                exempts_obligations: true,
                ..Default::default()
            },
        });
        fields
    }

    fn w004(kind: &str) -> ValidationRulePattern {
        ValidationRulePattern {
            code: "W004".into(),
            severity: specforge_common::Severity::Warning,
            message_template: "{kind} '{id}' is testable but declares no verify obligations".into(),
            check: ValidationPatternKind::NoVerifyStatements,
            target_kind: Some(kind.into()),
            edge_type: None,
            edge_peer_kind: None,
            field: Some("verify".into()),
            constraint: None,
            wasm_function: None,
        }
    }

    /// The ids W004 reports on `source` (rules on `behavior` and `type`).
    fn w004_ids(source: &str, fields: &FieldRegistry) -> Vec<String> {
        let entities = build_validation_entities(&graph_of(source), fields);
        let mut ids: Vec<String> = ["behavior", "type"]
            .into_iter()
            .flat_map(|kind| {
                specforge_registry::validation_engine::execute_pattern(&w004(kind), &entities, None)
            })
            .map(|d| d.message.split('\'').nth(1).unwrap().to_string())
            .collect();
        ids.sort();
        ids
    }

    #[specforge_test(
        behavior = "te_validate_unverified_testable",
        verify = "a union type never produces W004"
    )]
    fn a_union_type_owes_no_obligations() {
        let ids = w004_ids(
            "type Status = active | inactive\n\ntype Plain \"Plain\" {\n  id string\n}\n",
            &FieldRegistry::new(),
        );
        assert_eq!(ids, ["Plain"]);
    }

    #[test]
    fn an_enum_values_list_is_not_a_union() {
        let ids = w004_ids(
            "type Priority \"Priority\" {\n  values [high, low]\n}\n",
            &FieldRegistry::new(),
        );
        assert_eq!(ids, ["Priority"]);
    }

    #[specforge_test(
        behavior = "te_validate_unverified_testable",
        verify = "an abstract entity never produces W004"
    )]
    fn an_abstract_entity_owes_no_obligations_when_its_kind_declares_the_flag() {
        let source = "behavior base \"Base\" {\n  contract \"The system MUST work\"\n  abstract true\n}\n\n\
                      behavior concrete \"Concrete\" {\n  contract \"The system MUST work\"\n  abstract false\n}\n";
        assert_eq!(w004_ids(source, &abstract_behaviors()), ["concrete"]);
        // Without a registry entry declaring it, `abstract` is just a name.
        assert_eq!(
            w004_ids(source, &FieldRegistry::new()),
            ["base", "concrete"]
        );
    }

    #[specforge_test(
        invariant = "testable_entity_classification",
        verify = "no default testability assumed by core"
    )]
    fn no_kind_is_testable_unless_an_extension_says_so() {
        assert!(testable_kinds(&KindRegistry::new()).is_empty());

        let mut reg = KindRegistry::new();
        reg.register(kind("behavior", false, false));
        assert!(testable_kinds(&reg).is_empty());
    }

    #[specforge_test(
        invariant = "testable_entity_classification",
        verify = "testable=false entity excluded from coverage"
    )]
    fn only_kinds_declared_testable_count() {
        let mut reg = KindRegistry::new();
        reg.register(kind("behavior", true, true));
        reg.register(kind("type", true, true));
        // Accepts verify statements but does not count toward coverage.
        reg.register(kind("property", false, true));
        reg.register(kind("feature", false, false));
        assert_eq!(
            testable_kinds(&reg).into_iter().collect::<Vec<_>>(),
            ["behavior", "type"]
        );
    }
}
