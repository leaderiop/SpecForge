use serde_json::Value;
use specforge_emitter::analyze::{ReportedTest, TestReport};
use specforge_graph::{FieldValue, Node};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

/// How a report's recorded status names a passing test.
const PASSING_STATUS: &str = "pass";

/// The project's `specforge-report.json` (written by `specforge collect`),
/// if there is one. Tests link themselves to entities by annotation
/// (ADR 0002), so recorded results are the linkage.
pub(crate) fn recorded_report(state: &McpState) -> Option<TestReport> {
    let raw =
        std::fs::read_to_string(state.project_root.as_ref()?.join("specforge-report.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

/// How an entity's recorded tests cover its `verify` obligations: the
/// rule `analyze coverage` applies (A015, A014), so the two never disagree.
pub(crate) struct EntityCoverage<'a> {
    pub obligations: usize,
    /// Verify texts no passing test names, in declaration order.
    pub unproven: Vec<&'a str>,
    pub tests: usize,
    pub failing: usize,
}

impl<'a> EntityCoverage<'a> {
    pub fn of(node: &'a Node, report: Option<&'a TestReport>) -> Self {
        let texts: Vec<&str> = match node.fields.get("verify") {
            Some(FieldValue::VerifyList(stmts)) => {
                stmts.iter().map(|s| s.description.as_str()).collect()
            }
            _ => Vec::new(),
        };
        let tests: &[ReportedTest] = report
            .and_then(|r| r.results.get(node.id.raw.as_str()))
            .map_or(&[], |e| e.tests.as_slice());
        let passing = |text: &str| {
            tests
                .iter()
                .any(|t| t.status == PASSING_STATUS && t.verify.as_deref() == Some(text))
        };
        EntityCoverage {
            obligations: texts.len(),
            unproven: texts.into_iter().filter(|t| !passing(t)).collect(),
            tests: tests.len(),
            failing: tests.iter().filter(|t| t.status != PASSING_STATUS).count(),
        }
    }

    pub fn proven(&self) -> usize {
        self.obligations - self.unproven.len()
    }

    /// `covered` when every obligation is proven and no test fails;
    /// `partial` when some obligation is proven or a test fails;
    /// `uncovered` otherwise, including an entity with no obligations.
    pub fn status(&self) -> &'static str {
        if self.obligations > 0 && self.unproven.is_empty() && self.failing == 0 {
            "covered"
        } else if self.proven() > 0 || self.failing > 0 {
            "partial"
        } else {
            "uncovered"
        }
    }
}

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let report = recorded_report(state);
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());
    let kind_filter = args.get("kind").and_then(|v| v.as_str());
    let status_filter = args.get("status_filter").and_then(|v| v.as_str());

    let results: Vec<Value> = state
        .graph
        .nodes()
        .into_iter()
        .filter(|n| {
            if let Some(eid) = entity_filter {
                return n.id.raw == eid;
            }
            // Testability is the extensions' call (their kinds' manifests).
            let testable = state
                .kind_registry
                .get(n.kind.raw.as_str())
                .is_some_and(|kind| kind.testable);
            testable && kind_filter.is_none_or(|kind| n.kind.raw == kind)
        })
        .map(|n| (n, EntityCoverage::of(n, report.as_ref())))
        .filter(|(_, coverage)| status_filter.is_none_or(|s| coverage.status() == s))
        .map(|(n, coverage)| {
            serde_json::json!({
                "entity_id": n.id.raw,
                "kind": n.kind.raw,
                "status": coverage.status(),
                "declared": coverage.obligations > 0,
                "linked": coverage.tests > 0,
                "evidence_collected": coverage.tests > 0,
                "obligations": coverage.obligations,
                "proven": coverage.proven(),
                "unproven": coverage.unproven,
            })
        })
        .collect();

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results).unwrap()
            }]
        }),
    )
}
