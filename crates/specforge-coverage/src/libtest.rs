use crate::junit::{RawTestResult, RawTestStatus};

/// Parse libtest `--format json` NDJSON output into raw test results.
///
/// Expects one JSON object per line. Only processes events with `"type": "test"`.
/// Ignores `"type": "suite"` and other non-test events.
pub fn parse_libtest_json(input: &str) -> Result<Vec<RawTestResult>, String> {
    let mut results = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("JSON parse error: {e}"))?;

        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        // Only process test events
        let event_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if event_type != "test" {
            continue;
        }

        let event = match obj.get("event").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => continue,
        };

        let name = match obj.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let status = match event {
            "ok" => RawTestStatus::Pass,
            "failed" => RawTestStatus::Fail,
            "ignored" => RawTestStatus::Skip,
            _ => continue,
        };

        let duration_secs = obj
            .get("exec_time")
            .and_then(|v| v.as_f64());

        let message = if status == RawTestStatus::Fail {
            obj.get("stdout").and_then(|v| v.as_str()).map(|s| s.to_string())
        } else {
            None
        };

        results.push(RawTestResult {
            name,
            classname: None,
            status,
            duration_secs,
            message,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_passing_tests() {
        let input = r#"{ "type": "suite", "event": "started", "test_count": 2 }
{ "type": "test", "event": "ok", "name": "validate_input__rejects_empty", "exec_time": 0.001 }
{ "type": "test", "event": "ok", "name": "validate_input__accepts_valid", "exec_time": 0.002 }
{ "type": "suite", "event": "ok", "passed": 2, "failed": 0 }"#;
        let results = parse_libtest_json(input).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "validate_input__rejects_empty");
        assert_eq!(results[0].status, RawTestStatus::Pass);
        assert_eq!(results[0].duration_secs, Some(0.001));
    }

    #[test]
    fn parse_failure() {
        let input = r#"{ "type": "test", "event": "failed", "name": "data_check__fails", "stdout": "assertion failed" }"#;
        let results = parse_libtest_json(input).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, RawTestStatus::Fail);
        assert_eq!(results[0].message.as_deref(), Some("assertion failed"));
    }

    #[test]
    fn parse_ignored() {
        let input = r#"{ "type": "test", "event": "ignored", "name": "skipped_test" }"#;
        let results = parse_libtest_json(input).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, RawTestStatus::Skip);
    }

    #[test]
    fn suite_events_ignored() {
        let input = r#"{ "type": "suite", "event": "started", "test_count": 0 }
{ "type": "suite", "event": "ok", "passed": 0, "failed": 0 }"#;
        let results = parse_libtest_json(input).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn empty_input() {
        let results = parse_libtest_json("").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn malformed_line_returns_error() {
        let input = "not valid json";
        let result = parse_libtest_json(input);
        assert!(result.is_err());
    }
}
