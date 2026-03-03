use serde::{Deserialize, Serialize};

/// The status of a single test execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

impl TestStatus {
    /// Returns true if this is a passing status.
    pub fn is_passing(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Returns the "worst" of two statuses (Error > Fail > Skip > Pass).
    pub fn worst(self, other: Self) -> Self {
        match (self, other) {
            (Self::Error, _) | (_, Self::Error) => Self::Error,
            (Self::Fail, _) | (_, Self::Fail) => Self::Fail,
            (Self::Skip, _) | (_, Self::Skip) => Self::Skip,
            _ => Self::Pass,
        }
    }
}

/// Result of a single test function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test function name (e.g., `auth_login__rejects_invalid_password`).
    pub name: String,
    /// Pass/fail/skip/error.
    pub status: TestStatus,
    /// Duration in milliseconds, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    /// Error message on failure, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Results for a single spec entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityResult {
    /// The spec entity ID (e.g., `auth_login`).
    pub entity_id: String,
    /// Individual test results for this entity.
    pub tests: Vec<TestResult>,
}

impl EntityResult {
    /// Aggregate status: worst status across all tests.
    pub fn aggregate_status(&self) -> TestStatus {
        self.tests
            .iter()
            .map(|t| t.status)
            .fold(TestStatus::Pass, TestStatus::worst)
    }

    /// Number of passing tests.
    pub fn pass_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Pass).count()
    }

    /// Number of failing tests.
    pub fn fail_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Fail).count()
    }
}

/// A `specforge-report.json` file produced by test runner adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecForgeReport {
    /// Schema version for the report format.
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    /// Timestamp when the report was generated (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Name of the test runner adapter that produced this report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Per-entity test results.
    pub entities: Vec<EntityResult>,
}

fn default_schema_version() -> String {
    "1.0".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_report() {
        let json = r#"{
            "schema_version": "1.0",
            "adapter": "vitest",
            "entities": [
                {
                    "entity_id": "auth_login",
                    "tests": [
                        {"name": "auth_login__accepts_valid_creds", "status": "pass", "duration_ms": 12.5},
                        {"name": "auth_login__rejects_invalid", "status": "fail", "message": "expected 401"}
                    ]
                },
                {
                    "entity_id": "data_persistence",
                    "tests": [
                        {"name": "data_persistence__saves", "status": "pass"}
                    ]
                }
            ]
        }"#;
        let report: SpecForgeReport = serde_json::from_str(json).unwrap();
        assert_eq!(report.schema_version, "1.0");
        assert_eq!(report.adapter.as_deref(), Some("vitest"));
        assert_eq!(report.entities.len(), 2);
        assert_eq!(report.entities[0].entity_id, "auth_login");
        assert_eq!(report.entities[0].tests.len(), 2);
        assert_eq!(report.entities[0].aggregate_status(), TestStatus::Fail);
        assert_eq!(report.entities[1].aggregate_status(), TestStatus::Pass);
    }

    #[test]
    fn test_status_worst() {
        assert_eq!(TestStatus::Pass.worst(TestStatus::Pass), TestStatus::Pass);
        assert_eq!(TestStatus::Pass.worst(TestStatus::Fail), TestStatus::Fail);
        assert_eq!(TestStatus::Fail.worst(TestStatus::Skip), TestStatus::Fail);
        assert_eq!(TestStatus::Skip.worst(TestStatus::Error), TestStatus::Error);
        assert_eq!(TestStatus::Error.worst(TestStatus::Pass), TestStatus::Error);
    }

    #[test]
    fn entity_result_counts() {
        let entity = EntityResult {
            entity_id: "test".to_string(),
            tests: vec![
                TestResult { name: "a".into(), status: TestStatus::Pass, duration_ms: None, message: None },
                TestResult { name: "b".into(), status: TestStatus::Fail, duration_ms: None, message: Some("err".into()) },
                TestResult { name: "c".into(), status: TestStatus::Pass, duration_ms: None, message: None },
            ],
        };
        assert_eq!(entity.pass_count(), 2);
        assert_eq!(entity.fail_count(), 1);
    }
}
