use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The API reader's api-extractor 7.60.1 recording: package `@demo/widget`.
const RECORDED: &str =
    include_str!("../../../code/api/fixtures/api-extractor-7.60.1.widget.api.json");
/// The served items of `@demo/widget`, as JSON (see `fixtures/README.md`).
const ITEMS: &str = include_str!("../fixtures/items.json");
/// The doc model of a run that breaks the API `TAGGED` accepted (the API reader's recordings).
const CHANGED: &str =
    include_str!("../../../code/api/fixtures/api-extractor-7.60.1.widget-changed.api.json");
/// The committed API report of `@demo/widget` before the break.
const TAGGED: &str =
    include_str!("../../../code/api/fixtures/api-extractor-7.60.1.widget-tagged.api.md");

/// The input of a run writing `model`, the host serving `files` (each read by its name).
fn against(model: &str, files: Value) -> CollectInput {
    CollectInput {
        files: serde_json::from_value(files).expect("the reads parse"),
        ..input(&[model])
    }
}

/// One read serving `text` at `path`.
fn served(path: &str, text: &str) -> Value {
    json!({"facts": [{"path": path, "exists": true, "text": text}]})
}

/// Each fact's name path (its id after `::`) and its `change`.
fn changes(out: &CollectOutput) -> Vec<(&str, Option<&str>)> {
    out.facts
        .iter()
        .map(|f| {
            let name = f.id.split_once("::").map_or("", |(_, name)| name);
            (name, f.fields.get("change").and_then(Value::as_str))
        })
        .collect()
}

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
            ("@code/core", "^1.4", false),
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
    // Never `**/*.api.md`: api-extractor's `temp/` holds the fresh run's own report.
    assert_eq!(
        collector.inputs,
        [
            Input::facts(&["item"]),
            Input::files(
                "accepted",
                FileReadSpec::glob("**/etc/*.api.md").text_up_to(4_194_304)
            ),
            Input::files(
                "reviews",
                FileReadSpec::glob("common/reviews/api/*.api.md").text_up_to(4_194_304)
            )
        ]
    );
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

#[specforge_test(requirement = "FEATURE-149")]
fn a_committed_report_in_etc_or_in_rushs_reviews_marks_each_items_change() {
    let mut seen = Vec::new();
    for files in [
        json!({"accepted": served("etc/widget.api.md", TAGGED)}),
        json!({"reviews": served("common/reviews/api/widget.api.md", TAGGED)}),
    ] {
        let out = collect(&against(CHANGED, files)).unwrap();
        assert_eq!(out.warnings, Vec::<String>::new());
        let changes = changes(&out);
        for (name, change) in [
            ("create", "changed"),
            ("Widget.render", "narrowed"),
            ("Widget.size", "narrowed"),
            ("trial", "removed"),
            ("added", "added"),
        ] {
            assert!(
                changes.contains(&(name, Some(change))),
                "{name} {change}: {changes:?}"
            );
        }
        seen.push(serde_json::to_value(&out.facts).unwrap());
    }
    assert_eq!(seen[0], seen[1]);
}

#[specforge_test(requirement = "FEATURE-149")]
fn a_report_variant_is_not_read() {
    let today = collect(&input(&[CHANGED])).unwrap();
    let files = json!({"accepted": served("etc/widget.beta.api.md", TAGGED)});
    let out = collect(&against(CHANGED, files)).unwrap();
    assert_eq!(out.facts, today.facts);
    assert_eq!(out.warnings, Vec::<String>::new());
}

#[specforge_test(requirement = "FEATURE-149")]
fn two_reports_of_the_package_or_a_bad_one_is_the_collectors_error_naming_them() {
    let files = json!({
        "accepted": served("etc/widget.api.md", TAGGED),
        "reviews": served("common/reviews/api/widget.api.md", TAGGED),
    });
    let why = collect(&against(CHANGED, files)).unwrap_err();
    assert!(
        why.contains("etc/widget.api.md") && why.contains("common/reviews/api/widget.api.md"),
        "{why}"
    );
    let conflicted = format!("<<<<<<< HEAD\n{TAGGED}");
    let files = json!({"accepted": served("etc/widget.api.md", &conflicted)});
    let why = collect(&against(CHANGED, files)).unwrap_err();
    assert!(why.starts_with("etc/widget.api.md: "), "{why}");
}

#[specforge_test(requirement = "FEATURE-149")]
fn prose_or_a_broken_report_of_another_package_is_one_warning_and_the_collect_goes_on() {
    let today = collect(&input(&[CHANGED])).unwrap();
    let other = TAGGED.replace("\"@demo/widget\"", "\"@demo/other\"");
    for (path, text) in [
        (
            "etc/notes.api.md",
            "# Notes\n\nThe users API, in prose.\n".to_string(),
        ),
        ("other/etc/other.api.md", format!("{other}\n<<<<<<< HEAD\n")),
    ] {
        let out = collect(&against(CHANGED, json!({"accepted": served(path, &text)}))).unwrap();
        assert_eq!(out.facts, today.facts);
        assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
        assert!(
            out.warnings[0].starts_with(&format!(
                "{path}: not an API report of a package this run read ("
            )),
            "{:?}",
            out.warnings
        );
    }
}

#[specforge_test(requirement = "FEATURE-149")]
fn a_report_not_served_or_of_another_package_is_one_warning_and_no_change() {
    let today = collect(&input(&[CHANGED])).unwrap();
    let other = TAGGED.replace("\"@demo/widget\"", "\"@demo/other\"");
    let unserved =
        json!({"facts": [{"path": "etc/widget.api.md", "exists": true, "error": "too_large"}]});
    for (read, path, says) in [
        (
            served("etc/other.api.md", &other),
            "etc/other.api.md",
            "names no package this run read",
        ),
        (
            unserved,
            "etc/widget.api.md",
            "the accepted API was not read (too_large)",
        ),
    ] {
        let out = collect(&against(CHANGED, json!({ "accepted": read }))).unwrap();
        assert_eq!(out.facts, today.facts);
        assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
        assert!(
            out.warnings[0].starts_with(&format!("{path}: ")) && out.warnings[0].contains(says),
            "{:?}",
            out.warnings
        );
    }
}

#[specforge_test(requirement = "FEATURE-149")]
fn a_truncated_read_names_the_walks_entry_cap_and_the_file_cap() {
    let mut read = served("etc/widget.api.md", TAGGED);
    read["truncated"] = json!(true);
    let out = collect(&against(CHANGED, json!({ "accepted": read }))).unwrap();
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert_eq!(
        out.warnings[0],
        "the read 'accepted' (**/etc/*.api.md) stopped early: the host's walk reached its cap of 20000 entries, \
         or more than 2000 files matched; no break was judged against a file not read"
    );
}
