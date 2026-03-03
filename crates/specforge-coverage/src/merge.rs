use crate::report::{EntityResult, SpecForgeReport, TestResult, TestStatus};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A merged report combining results from multiple `specforge-report.json` files.
#[derive(Debug, Clone)]
pub struct MergedReport {
    /// Per-entity results keyed by entity_id. Duplicates are merged (worst-status wins).
    pub entities: HashMap<String, EntityResult>,
    /// Source files that were discovered and parsed.
    pub source_files: Vec<PathBuf>,
}

impl MergedReport {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            source_files: Vec::new(),
        }
    }

    /// Merge a single report into this merged report.
    pub fn merge_report(&mut self, report: SpecForgeReport) {
        for entity in report.entities {
            self.entities
                .entry(entity.entity_id.clone())
                .and_modify(|existing| {
                    merge_entity_results(existing, &entity);
                })
                .or_insert(entity);
        }
    }

    /// Get the result for a specific entity ID.
    pub fn get(&self, entity_id: &str) -> Option<&EntityResult> {
        self.entities.get(entity_id)
    }

    /// Total number of unique entities with test results.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// All entity IDs in the merged report.
    pub fn entity_ids(&self) -> Vec<&str> {
        self.entities.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for MergedReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Merge entity results: add new tests, update existing tests with worst status.
fn merge_entity_results(existing: &mut EntityResult, incoming: &EntityResult) {
    // Build a set of existing test names with their index for O(1) lookup
    let existing_names: HashMap<&str, usize> = existing
        .tests
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.as_str(), i))
        .collect();

    let mut updates: Vec<(usize, TestStatus, Option<String>)> = Vec::new();
    let mut new_tests: Vec<TestResult> = Vec::new();

    for test in &incoming.tests {
        if let Some(&idx) = existing_names.get(test.name.as_str()) {
            let prev_status = existing.tests[idx].status;
            let worst = prev_status.worst(test.status);
            if worst != prev_status {
                updates.push((idx, worst, test.message.clone()));
            }
        } else {
            new_tests.push(test.clone());
        }
    }

    for (idx, status, message) in updates {
        existing.tests[idx].status = status;
        if message.is_some() {
            existing.tests[idx].message = message;
        }
    }

    existing.tests.extend(new_tests);
}

/// Discover all `specforge-report.json` files in the given directories and merge them.
///
/// If `test_dirs` is empty, searches from `project_root`.
pub fn discover_and_merge(test_dirs: &[String], project_root: &Path) -> MergedReport {
    let mut merged = MergedReport::new();

    let search_dirs: Vec<PathBuf> = if test_dirs.is_empty() {
        vec![project_root.to_path_buf()]
    } else {
        test_dirs
            .iter()
            .map(|d| project_root.join(d))
            .collect()
    };

    for dir in &search_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file()
                && entry.file_name() == "specforge-report.json"
            {
                let path = entry.into_path();
                match load_report(&path) {
                    Ok(report) => {
                        merged.source_files.push(path);
                        merged.merge_report(report);
                    }
                    Err(e) => {
                        eprintln!(
                            "specforge: warning: failed to parse {}: {e}",
                            path.display()
                        );
                    }
                }
            }
        }
    }

    merged
}

/// Load and parse a single `specforge-report.json` file.
fn load_report(path: &Path) -> Result<SpecForgeReport, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("error reading {}: {e}", path.display()))?;
    serde_json::from_str::<SpecForgeReport>(&content)
        .map_err(|e| format!("error parsing {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{TestResult, TestStatus};

    fn make_entity(id: &str, tests: Vec<(&str, TestStatus)>) -> EntityResult {
        EntityResult {
            entity_id: id.to_string(),
            tests: tests
                .into_iter()
                .map(|(name, status)| TestResult {
                    name: name.to_string(),
                    status,
                    duration_ms: None,
                    message: None,
                })
                .collect(),
        }
    }

    fn make_report(entities: Vec<EntityResult>) -> SpecForgeReport {
        SpecForgeReport {
            schema_version: "1.0".to_string(),
            timestamp: None,
            adapter: None,
            entities,
        }
    }

    #[test]
    fn merge_disjoint_reports() {
        let mut merged = MergedReport::new();
        merged.merge_report(make_report(vec![
            make_entity("auth_login", vec![("test_a", TestStatus::Pass)]),
        ]));
        merged.merge_report(make_report(vec![
            make_entity("data_persist", vec![("test_b", TestStatus::Pass)]),
        ]));
        assert_eq!(merged.entity_count(), 2);
    }

    #[test]
    fn merge_worst_status_wins() {
        let mut merged = MergedReport::new();
        merged.merge_report(make_report(vec![
            make_entity("auth_login", vec![("test_a", TestStatus::Pass)]),
        ]));
        merged.merge_report(make_report(vec![
            make_entity("auth_login", vec![("test_a", TestStatus::Fail)]),
        ]));
        assert_eq!(merged.entity_count(), 1);
        let entity = merged.get("auth_login").unwrap();
        assert_eq!(entity.tests[0].status, TestStatus::Fail);
    }

    #[test]
    fn merge_adds_new_tests() {
        let mut merged = MergedReport::new();
        merged.merge_report(make_report(vec![
            make_entity("auth_login", vec![("test_a", TestStatus::Pass)]),
        ]));
        merged.merge_report(make_report(vec![
            make_entity("auth_login", vec![("test_b", TestStatus::Pass)]),
        ]));
        let entity = merged.get("auth_login").unwrap();
        assert_eq!(entity.tests.len(), 2);
    }
}
