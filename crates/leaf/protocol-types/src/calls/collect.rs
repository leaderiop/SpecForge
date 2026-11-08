//! The collector's payloads (`collect__<name>`): what the host passes the export, and what it answers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Fact, GraphWire, TestResult, TestStatus};
use crate::EditPlan;

/// What the host passes to a `collect__<name>` export: the runner's report files, read from the declared report
/// location, and the command's standard output when the collector captures it.
///
/// Every collector receives the project root as `cwd`, canonical (protocol 1.15.0, ADR 0327). A collector that
/// declares `writes` also receives the host's date, the graph and the revisions of the sources (protocol 1.9.0).
/// `G` is how the graph is held, as in [`CommandInput`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectInput<G = GraphWire> {
    #[serde(default)]
    pub reports: Vec<CollectReportFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub today: Option<String>,
    /// [`CommandInput::now`], for a collector that declares `writes` (protocol 1.11.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<G>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub revisions: BTreeMap<String, String>,
    /// The facts of the files the collector's `files` inputs name, by read name (ADR 0293).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: crate::FileMap,
    /// Each checked value's outcome, for a collector that declares `check_outcomes` (ADR 0336).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub check_outcomes: Vec<crate::CheckOutcome>,
    /// The admitted facts of the kinds its `facts` input names, sorted by kind and id (ADR 0345 D3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<crate::Fact>,
}

impl<G> Default for CollectInput<G> {
    fn default() -> Self {
        CollectInput {
            reports: Vec::new(),
            stdout: None,
            cwd: None,
            today: None,
            now: None,
            graph: None,
            revisions: BTreeMap::new(),
            files: crate::FileMap::new(),
            check_outcomes: Vec::new(),
            facts: Vec::new(),
        }
    }
}

/// One report file: its path relative to the project root, and its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectReportFile {
    pub path: String,
    pub content: String,
}

/// What a `collect__<name>` export returns: test results grouped by the
/// entity each test proves, and the tests the report doesn't link to any
/// entity, which the host links by naming convention when it can.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CollectOutput {
    #[serde(default)]
    pub entity_results: Vec<CollectEntityResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unlinked: Vec<CollectUnlinkedTest>,
    /// What a collector that declares `writes` asks the host to write (protocol 1.9.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<EditPlan>,
    /// The entities the collector observed (protocol 1.15.0, ADR 0327): stored by the host, served read-only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<Fact>,
}

/// A test the report doesn't link to an entity: its name, the name split
/// into its path segments (`["tests", "add_item", "rejects_a_duplicate"]`,
/// the test's own name last), and its [`TestStatus`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectUnlinkedTest {
    pub name: String,
    pub path: Vec<String>,
    pub status: TestStatus,
}

/// The tests a report links to one entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectEntityResult {
    pub entity_id: String,
    pub test_results: Vec<TestResult>,
}
