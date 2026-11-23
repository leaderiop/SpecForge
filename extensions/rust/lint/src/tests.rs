/// The converter's unit tests: cargo JSON lines in, SARIF results out.
mod clippy {
    use crate::clippy::results;
    use serde_json::json;
    use specforge_code_facts::{finding_facts, Level, SarifResult};
    use specforge_extension_sdk::prelude::Fact;

    /// clippy 0.1.98's recording (see `fixtures/README.md`): crate `demo` at `/project`.
    const CLIPPY_0_1_98: &str = include_str!("../fixtures/clippy-0.1.98.jsonl");
    /// The served facts of the recorded crate, as JSON.
    const ITEMS: &str = include_str!("../fixtures/items.json");

    /// One `compiler-message` line: a diagnostic of `level` with `code`, its `spans` as written.
    fn message(code: Option<&str>, level: &str, text: &str, spans: serde_json::Value) -> String {
        json!({
            "reason": "compiler-message",
            "package_id": "path+file:///project#demo@0.1.0",
            "message": {
                "code": code.map(|code| json!({"code": code, "explanation": null})),
                "level": level,
                "message": text,
                "spans": spans,
                "children": [],
            },
        })
        .to_string()
    }

    /// A span of `file` over the lines `start` to `end`.
    fn span(file: &str, start: u64, end: u64, primary: bool) -> serde_json::Value {
        json!({"file_name": file, "line_start": start, "line_end": end, "is_primary": primary})
    }

    fn result(rule: &str, level: Level, text: &str, place: (&str, usize, usize)) -> SarifResult {
        SarifResult {
            tool: "clippy".to_string(),
            rule: rule.to_string(),
            level,
            message: text.to_string(),
            file: Some(place.0.to_string()),
            line: Some(place.1),
            end_line: Some(place.2),
        }
    }

    #[test]
    fn a_clippy_message_reads_its_code_level_message_and_primary_span() {
        let text = [
            message(
                Some("clippy::needless_return"),
                "warning",
                "unneeded `return` statement",
                json!([
                    span("src/other.rs", 2, 2, false),
                    span("src/lib.rs", 7, 8, true)
                ]),
            ),
            message(
                Some("unused_variables"),
                "error",
                "unused variable",
                json!([span("src/lib.rs", 12, 12, true)]),
            ),
        ]
        .join("\n");
        let (read, outside) = results(&text, "/project").expect("the lines read");
        assert_eq!(
            read,
            [
                result(
                    "clippy::needless_return",
                    Level::Warning,
                    "unneeded `return` statement",
                    ("src/lib.rs", 7, 8)
                ),
                result(
                    "unused_variables",
                    Level::Error,
                    "unused variable",
                    ("src/lib.rs", 12, 12)
                ),
            ]
        );
        assert!(outside.is_empty());
    }

    #[test]
    fn a_clippy_level_maps_to_sarif_and_a_codeless_message_with_a_span_is_unknown() {
        let levels = ["error", "warning", "note", "help", "failure-note"];
        let text: Vec<String> = levels
            .iter()
            .enumerate()
            .map(|(at, level)| {
                message(
                    None,
                    level,
                    "m",
                    json!([span("src/lib.rs", at as u64 + 1, at as u64 + 1, true)]),
                )
            })
            .collect();
        let (read, _) = results(&text.join("\n"), "/project").expect("the lines read");
        let read: Vec<(&str, Level)> = read.iter().map(|r| (r.rule.as_str(), r.level)).collect();
        assert_eq!(
            read,
            [
                ("unknown", Level::Error),
                ("unknown", Level::Warning),
                ("unknown", Level::Note),
                ("unknown", Level::Note),
                ("unknown", Level::Unknown),
            ]
        );
    }

    #[test]
    fn other_clippy_reasons_summaries_and_blank_lines_are_ignored() {
        let text = [
            json!({"reason": "compiler-artifact", "package_id": "x"}).to_string(),
            json!({"reason": "build-script-executed", "package_id": "x"}).to_string(),
            String::new(),
            "   ".to_string(),
            message(None, "warning", "2 warnings emitted", json!([])),
            message(Some("dead_code"), "warning", "never used", json!([])),
            json!({"reason": "build-finished", "success": true}).to_string(),
        ]
        .join("\n");
        let (read, outside) = results(&text, "/project").expect("the lines read");
        let rules: Vec<(&str, Option<&str>)> = read
            .iter()
            .map(|r| (r.rule.as_str(), r.file.as_deref()))
            .collect();
        assert_eq!(rules, [("dead_code", None)]);
        assert!(outside.is_empty());
        assert_eq!(results("", "/project"), Ok((Vec::new(), Vec::new())));
    }

    #[test]
    fn a_clippy_finding_repeated_by_all_targets_appears_once_in_first_seen_order() {
        let a = message(
            Some("A"),
            "warning",
            "a",
            json!([span("src/lib.rs", 1, 1, true)]),
        );
        let b = message(
            Some("B"),
            "warning",
            "b",
            json!([span("src/lib.rs", 2, 2, true)]),
        );
        let a_elsewhere = message(
            Some("A"),
            "warning",
            "a",
            json!([span("src/lib.rs", 3, 3, true)]),
        );
        let text = [&a, &b, &a, &b, &a_elsewhere]
            .map(String::as_str)
            .join("\n");
        let (read, _) = results(&text, "/project").expect("the lines read");
        let read: Vec<(&str, Option<usize>)> =
            read.iter().map(|r| (r.rule.as_str(), r.line)).collect();
        assert_eq!(read, [("A", Some(1)), ("B", Some(2)), ("A", Some(3))]);
    }

    #[test]
    fn a_clippy_span_outside_the_project_is_counted_once_not_kept() {
        let registry = "/home/u/.cargo/registry/src/x/lib.rs";
        let text = [
            message(
                Some("A"),
                "warning",
                "a",
                json!([span(registry, 4, 4, true)]),
            ),
            message(
                Some("B"),
                "warning",
                "b",
                json!([span(registry, 9, 9, true)]),
            ),
            message(
                Some("C"),
                "warning",
                "c",
                json!([span("../beside/src/lib.rs", 1, 1, true)]),
            ),
            message(
                Some("D"),
                "warning",
                "d",
                json!([span("/project/src/lib.rs", 5, 5, true)]),
            ),
        ]
        .join("\n");
        let (read, outside) = results(&text, "/project").expect("the lines read");
        let kept: Vec<(&str, Option<&str>)> = read
            .iter()
            .map(|r| (r.rule.as_str(), r.file.as_deref()))
            .collect();
        assert_eq!(kept, [("D", Some("src/lib.rs"))]);
        assert_eq!(outside, [registry, "../beside/src/lib.rs"]);
    }

    #[test]
    fn a_clippy_line_that_is_no_json_is_an_error_naming_it() {
        let whole = message(
            Some("A"),
            "warning",
            "a",
            json!([span("src/lib.rs", 1, 1, true)]),
        );
        let cut = &whole[..whole.len() / 2];
        let error = results(&format!("{whole}\n\n{cut}"), "/project").expect_err("cut off");
        assert!(error.starts_with("line 3: no JSON"), "{error}");
    }

    #[test]
    fn a_recorded_clippy_0_1_98_run_mints_each_finding_once() {
        let (read, outside) = results(CLIPPY_0_1_98, "/project").expect("the recording reads");
        assert!(outside.is_empty());
        let placed: Vec<(&str, Option<usize>)> =
            read.iter().map(|r| (r.rule.as_str(), r.line)).collect();
        assert_eq!(
            placed,
            [
                ("unused_variables", Some(12)),
                ("clippy::len_without_is_empty", Some(6)),
                ("clippy::needless_return", Some(7)),
                ("clippy::unwrap_used", Some(13)),
            ]
        );
        let served: Vec<Fact> = serde_json::from_str(ITEMS).expect("the served facts parse");
        let facts = finding_facts(&read, &served);
        insta::assert_json_snapshot!("clippy_0_1_98", facts);
    }
}
