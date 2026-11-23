use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The API reader's api-extractor 7.60.1 recording: package `@demo/widget`.
const RECORDED: &str =
    include_str!("../../../code/api/fixtures/api-extractor-7.60.1.widget.api.json");
/// The served items of `@demo/widget`, as JSON (see `fixtures/README.md`).
const ITEMS: &str = include_str!("../fixtures/items.json");

/// The input the host sends for these reports, the package's items served.
fn input(reports: &[&str]) -> CollectInput {
    CollectInput {
        reports: reports
            .iter()
            .map(|content| CollectReportFile {
                path: REPORT.to_string(),
                content: content.to_string(),
            })
            .collect(),
        facts: serde_json::from_str(ITEMS).expect("the served facts parse"),
        ..CollectInput::default()
    }
}

#[specforge_test(requirement = "FEATURE-149")]
fn javascript_api_declares_api_extractor_its_config_and_its_peers() {
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
            ("@javascript/source", "^1.0", false)
        ]
    );
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.input_formats, ["api-extractor-json"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["tsconfig.json".to_string()][..])
    );
    assert_eq!(collector.answers, ["api_item"]);
    assert_eq!(collector.inputs, [Input::facts(&["item"])]);
    assert_eq!(collector.report.as_deref(), Some(REPORT));
    assert_eq!(
        collector.install.as_deref(),
        Some("npm install --save-dev @microsoft/api-extractor typescript")
    );
    let capabilities = serde_json::to_value(&declaration.capabilities).unwrap();
    assert_eq!(capabilities[0]["name"], COLLECTOR);
    assert_eq!(capabilities[0]["port"], "process");
    assert_eq!(
        capabilities[0]["argv"],
        json!([
            "npx",
            "--no",
            "--",
            "api-extractor",
            "run",
            "--local",
            "--config",
            "{config}"
        ]),
        "{capabilities}"
    );
    assert_eq!(
        capabilities[0]["config"],
        json!({"name": "api-extractor.json", "template": TEMPLATE})
    );
    assert_eq!(capabilities[0]["env"], json!(["NODE_*", "CI"]));
    assert!(capabilities[0].get("filter").is_none_or(Value::is_null));
}

#[specforge_test(requirement = "FEATURE-149")]
fn the_config_writes_only_the_doc_model_to_the_report_path_from_dist() {
    assert!(TEMPLATE.len() < 16 * 1024);
    // The host escapes the absolute report path for a quoted string (ADR 0348 D2): `\` and `"` take a backslash.
    let report = r#"C:\work\my "board"\.specforge\reports\javascript-api.api.json"#;
    let escaped = report.replace('\\', "\\\\").replace('"', "\\\"");
    let config: Value = serde_json::from_str(&TEMPLATE.replace("{report}", &escaped))
        .expect("the config parses as JSON");
    assert_eq!(config["projectFolder"], "../..");
    assert_eq!(
        config["mainEntryPointFilePath"],
        "<projectFolder>/dist/index.d.ts"
    );
    assert_eq!(
        config["docModel"],
        json!({"enabled": true, "apiJsonFilePath": report})
    );
    for off in ["apiReport", "dtsRollup", "tsdocMetadata"] {
        assert_eq!(config[off], json!({"enabled": false}), "{off}");
    }
    let messages = config["messages"]
        .as_object()
        .expect("the message settings");
    let reporting: Vec<&str> = messages.keys().map(String::as_str).collect();
    assert_eq!(
        reporting,
        [
            "compilerMessageReporting",
            "extractorMessageReporting",
            "tsdocMessageReporting"
        ]
    );
    assert!(
        messages
            .values()
            .all(|m| m == &json!({"default": {"logLevel": "none"}})),
        "{messages:?}"
    );
}

#[specforge_test(requirement = "FEATURE-149")]
fn a_recorded_api_extractor_model_mints_api_items_citing_the_served_items() {
    let out = collect(&input(&[RECORDED])).unwrap();
    let cited: Vec<(&str, Option<&str>)> = out
        .facts
        .iter()
        .map(|f| (f.id.as_str(), f.fields.get("item").and_then(Value::as_str)))
        .collect();
    let at = |name: &str| format!("js:@demo/widget/src/index.ts::{name}");
    let expected = [
        ("Id", None),
        ("Mode", Some(at("Mode"))),
        ("Mode.Fast", None),
        ("Mode.Slow", None),
        ("Options", Some(at("Options"))),
        ("Options.mode", Some(at("Options.mode"))),
        ("VERSION", Some(at("VERSION"))),
        ("Widget", Some(at("Widget"))),
        ("Widget.render", Some(at("Widget.render"))),
        ("Widget.size", Some(at("Widget.size"))),
        ("create", Some(at("create"))),
    ];
    let expected: Vec<(String, Option<String>)> = expected
        .into_iter()
        .map(|(name, item)| (format!("api:js:%40demo%2Fwidget::{name}"), item))
        .collect();
    let expected: Vec<(&str, Option<&str>)> = expected
        .iter()
        .map(|(id, item)| (id.as_str(), item.as_deref()))
        .collect();
    assert_eq!(cited, expected);
    assert!(out.facts.iter().all(|f| f.kind == "api_item"));
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-149")]
fn zero_or_two_reports_or_a_bad_one_is_the_collectors_error() {
    for reports in [&[][..], &[RECORDED, RECORDED][..]] {
        let why = collect(&input(reports)).unwrap_err();
        assert!(why.contains(COLLECTOR) && why.contains(REPORT), "{why}");
    }
    let why = collect(&input(&["garbage"])).unwrap_err();
    assert!(why.starts_with(&format!("{REPORT}: no JSON")), "{why}");
}
