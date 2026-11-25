use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// clippy 0.1.98's recording (see `fixtures/README.md`): crate `demo` at `/project`.
const RECORDED: &str = include_str!("../fixtures/clippy-0.1.98.jsonl");
/// The converter's accepted snapshot of the facts `RECORDED` mints with `fixtures/items.json` served.
const RECORDED_FACTS: &str =
    include_str!("snapshots/specforge_ext_rust_lint__tests__clippy__clippy_0_1_98.snap");

/// The input the host sends at `/project`: `stdout` as captured, these report files, the recorded crate's items.
fn input(stdout: Option<&str>, reports: &[&str]) -> CollectInput {
    CollectInput {
        stdout: stdout.map(ToString::to_string),
        reports: reports
            .iter()
            .map(|content| CollectReportFile {
                path: "saved/clippy.jsonl".to_string(),
                content: content.to_string(),
            })
            .collect(),
        cwd: Some("/project".to_string()),
        facts: serde_json::from_str(include_str!("../fixtures/items.json"))
            .expect("the served facts parse"),
        ..CollectInput::default()
    }
}

/// The JSON body of an insta snapshot, after its `---` header.
fn snapshot(text: &str) -> Value {
    let body = text.splitn(3, "---\n").nth(2).expect("a snapshot body");
    serde_json::from_str(body).expect("the snapshot parses")
}

#[specforge_test(requirement = "FEATURE-146")]
fn rust_lint_declares_clippy_with_its_stdout_captured_and_its_peers() {
    let declaration = crate::specforge_extension_build().declaration();
    let peers: Vec<(&str, &str, bool)> = declaration
        .handshake
        .peer_dependencies
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str(), p.optional))
        .collect();
    assert_eq!(
        peers,
        [
            ("@code/core", "^1.2", false),
            ("@rust/source", "^1.0", false)
        ]
    );
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.input_formats, ["cargo-json"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["Cargo.toml".to_string()][..])
    );
    assert_eq!(collector.answers, ["static_finding"]);
    assert_eq!(collector.inputs, [Input::facts(&["item", "package"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(collector.capture.as_deref(), Some("stdout"));
    assert_eq!(
        collector.install.as_deref(),
        Some("rustup component add clippy")
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(capabilities[0]["name"], COLLECTOR);
    assert_eq!(capabilities[0]["port"], "process");
    assert_eq!(
        capabilities[0]["argv"],
        json!([
            "cargo-clippy",
            "clippy",
            "--workspace",
            "--all-targets",
            "--message-format=json"
        ]),
        "{capabilities}"
    );
    assert_eq!(
        capabilities[0]["env"],
        json!([
            "CARGO_*",
            "RUSTUP_*",
            "RUSTC",
            "RUSTC_*",
            "RUSTFLAGS",
            "RUSTDOCFLAGS",
            "CLIPPY_*"
        ])
    );
    assert!(capabilities[0].get("filter").is_none_or(Value::is_null));
}

#[specforge_test(requirement = "FEATURE-146")]
fn a_captured_clippy_run_or_a_saved_one_mints_the_converters_facts() {
    for given in [input(Some(RECORDED), &[]), input(None, &[RECORDED])] {
        let out = collect(&given).unwrap();
        assert_eq!(
            serde_json::to_value(&out.facts).unwrap(),
            snapshot(RECORDED_FACTS)
        );
        assert_eq!(out.warnings, Vec::<String>::new());
    }
}

#[specforge_test(requirement = "FEATURE-146")]
fn no_capture_and_no_single_report_is_the_collectors_error() {
    for reports in [&[][..], &[RECORDED, RECORDED][..]] {
        let why = collect(&input(None, reports)).unwrap_err();
        assert!(
            why.contains(COLLECTOR) && why.contains("standard output was not captured"),
            "{why}"
        );
    }
    let why = collect(&input(Some("{\"reason\""), &[])).unwrap_err();
    assert!(
        why.starts_with(&format!("{CAPTURE}: line 1: no JSON")),
        "{why}"
    );
    let no_cwd = CollectInput {
        cwd: None,
        ..input(Some(RECORDED), &[])
    };
    assert!(collect(&no_cwd).unwrap_err().contains("no cwd"));
}

#[specforge_test(requirement = "FEATURE-146")]
fn a_span_outside_the_project_is_skipped_with_one_warning_naming_the_capture() {
    // The first message's primary span moves into a registry crate.
    let (first, rest) = RECORDED.split_once('\n').expect("more than one line");
    let mut first: Value = serde_json::from_str(first).expect("the first line parses");
    first["message"]["spans"][0]["file_name"] = json!("/home/u/.cargo/registry/src/x/lib.rs");
    let outside = format!("{first}\n{rest}");
    let out = collect(&input(Some(&outside), &[])).unwrap();
    assert_eq!(
        out.facts.len(),
        4,
        "the lib test copy of the first finding is kept"
    );
    assert_eq!(
        out.warnings,
        [format!(
            "{CAPTURE}: skipped 1 file(s) outside the project: /home/u/.cargo/registry/src/x/lib.rs"
        )]
    );
}

#[specforge_test(requirement = "FEATURE-146")]
fn a_span_naming_no_file_is_skipped_with_its_own_warning() {
    let (first, rest) = RECORDED.split_once('\n').expect("more than one line");
    let mut first: Value = serde_json::from_str(first).expect("the first line parses");
    first["message"]["spans"][0]["file_name"] = json!("<std macros>");
    let out = collect(&input(Some(&format!("{first}\n{rest}")), &[])).unwrap();
    assert_eq!(
        out.facts.len(),
        4,
        "the lib test copy of the first finding is kept"
    );
    assert_eq!(
        out.warnings,
        [format!(
            "{CAPTURE}: skipped 1 path(s) naming no file: <std macros>"
        )]
    );
}

/// The converter's unit tests: cargo JSON lines in, SARIF results out.
mod clippy {
    use crate::clippy::{results, Converted};
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

    /// `lines` as one capture, ended by the `build-finished` record cargo writes last.
    fn finished<S: AsRef<str>>(lines: &[S]) -> String {
        let mut text: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
        text.push(r#"{"reason":"build-finished","success":true}"#);
        text.join("\n")
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
        let text = finished(&[
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
        ]);
        let Converted {
            results: read,
            outside,
            ..
        } = results(&text, "/project").expect("the lines read");
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
        let Converted { results: read, .. } =
            results(&finished(&text), "/project").expect("the lines read");
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
        let Converted {
            results: read,
            outside,
            ..
        } = results(&text, "/project").expect("the lines read");
        let rules: Vec<(&str, Option<&str>)> = read
            .iter()
            .map(|r| (r.rule.as_str(), r.file.as_deref()))
            .collect();
        assert_eq!(rules, [("dead_code", None)]);
        assert!(outside.is_empty());
        assert_eq!(
            results(&finished::<&str>(&[]), "/project"),
            Ok(Converted::default())
        );
    }

    #[test]
    fn a_clippy_span_naming_no_file_is_counted_apart_not_kept() {
        let text = finished(&[
            message(
                Some("A"),
                "warning",
                "a",
                json!([span("<std macros>", 4, 4, true)]),
            ),
            message(
                Some("B"),
                "warning",
                "b",
                json!([span("<anon>", 1, 1, true)]),
            ),
            message(Some("C"), "warning", "c", json!([span(".", 1, 1, true)])),
            message(
                Some("D"),
                "warning",
                "d",
                json!([span("/project", 1, 1, true)]),
            ),
            message(
                Some("E"),
                "warning",
                "e",
                json!([span("src/lib.rs", 5, 5, true)]),
            ),
        ]);
        let converted = results(&text, "/project").expect("the lines read");
        let kept: Vec<(&str, Option<&str>)> = converted
            .results
            .iter()
            .map(|r| (r.rule.as_str(), r.file.as_deref()))
            .collect();
        assert_eq!(kept, [("E", Some("src/lib.rs"))]);
        assert_eq!(
            converted.no_file,
            ["<std macros>", "<anon>", ".", "/project"]
        );
        assert!(converted.outside.is_empty(), "{:?}", converted.outside);
    }

    #[test]
    fn a_clippy_capture_with_no_build_finished_record_is_cut_off() {
        // A run killed between two lines: every line is whole JSON, but cargo's last record is missing.
        let whole = message(
            Some("A"),
            "warning",
            "a",
            json!([span("src/lib.rs", 1, 1, true)]),
        );
        for text in [whole.as_str(), ""] {
            let error = results(text, "/project").expect_err("cut off");
            assert!(error.starts_with("no `build-finished` record"), "{error}");
        }
        let (recorded, _) = CLIPPY_0_1_98
            .trim_end()
            .rsplit_once('\n')
            .expect("more than one line");
        assert!(results(recorded, "/project").is_err());
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
        let text = finished(&[&a, &b, &a, &b, &a_elsewhere]);
        let Converted { results: read, .. } = results(&text, "/project").expect("the lines read");
        let read: Vec<(&str, Option<usize>)> =
            read.iter().map(|r| (r.rule.as_str(), r.line)).collect();
        assert_eq!(read, [("A", Some(1)), ("B", Some(2)), ("A", Some(3))]);
    }

    #[test]
    fn a_clippy_span_outside_the_project_is_counted_once_not_kept() {
        let registry = "/home/u/.cargo/registry/src/x/lib.rs";
        let text = finished(&[
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
        ]);
        let Converted {
            results: read,
            outside,
            ..
        } = results(&text, "/project").expect("the lines read");
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
        let Converted {
            results: read,
            outside,
            ..
        } = results(CLIPPY_0_1_98, "/project").expect("the recording reads");
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
