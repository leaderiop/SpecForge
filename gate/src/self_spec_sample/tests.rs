use super::*;

const SOURCE: &str = "/scratch/run/src";
const TARGET: &str = "/scratch/run/dst";

fn clean_work() -> Work {
    Work {
        compiled: Vec::new(),
        target_dir: false,
        ran: vec!["rust:a::b#m::t".into()],
        cited: vec!["rust:a::b#m::t".into(), "rust:a#n::u".into()],
        absorb: json!({"compiles": 3, "edit_plans": 1}),
    }
}

fn run(diagnostics: Value) -> SampleRun {
    SampleRun {
        diagnostics: diagnostics.as_array().unwrap().clone(),
        selection: "{\n  \"term\": [\n    \"t_a\"\n  ]\n}\n".into(),
        ledger: format!("{{\n  \"from\": \"{SOURCE}\",\n  \"entries\": []\n}}\n"),
        paths: vec![
            (SOURCE.into(), "<source>"),
            (TARGET.into(), "<target>"),
            ("/scratch/gsd/gsd_core.wasm".into(), "<gsd_core.wasm>"),
        ],
        work: clean_work(),
    }
}

fn info() -> Value {
    json!({"severity": "Info", "code": "gsd/I536", "file": "spec/a.spec", "line": 3,
           "message": format!("held in {TARGET}/spec/a.spec")})
}

/// The goldens a clean run with one info blesses.
fn blessed() -> SampleGoldens {
    verdict(&run(json!([info()])), &SampleGoldens::default(), true)
        .bless
        .unwrap()
}

#[test]
fn an_error_or_a_warning_fails_the_sample_and_is_never_blessed() {
    for (severity, code) in [("Error", "code/E101"), ("Warning", "code/W102")] {
        let found = run(json!([info(), {"severity": severity, "code": code,
            "span": {"file": "spec/w.spec", "start_line": 4}, "message": format!("in {TARGET}/spec/w.spec")}]));
        let judged = verdict(&found, &blessed(), false);
        assert!(judged.failed, "{severity}");
        assert!(
            judged
                .lines
                .contains(&format!("  {code} spec/w.spec:4: in <target>/spec/w.spec")),
            "{:?}",
            judged.lines
        );
        let bless = verdict(&found, &SampleGoldens::default(), true);
        assert!(bless.failed && bless.bless.is_none(), "{severity}");
    }
}

#[test]
fn a_machine_warning_is_named_and_not_judged() {
    let found = run(json!([info(),
        {"severity": "Warning", "code": "checks/W172", "message": "offline", "machine": true}]));
    let judged = verdict(&found, &blessed(), false);
    assert!(!judged.failed, "{:?}", judged.lines);
    assert!(
        judged
            .lines
            .contains(&"  1 machine warnings not judged: checks/W172 1".to_string())
    );
    assert_eq!(judged.lines[0], "check: 0 errors, 0 warnings, 1 infos");
}

#[test]
fn infos_selection_and_ledger_must_equal_their_goldens() {
    let goldens = blessed();
    assert!(!verdict(&run(json!([info()])), &goldens, false).failed);
    let mut other_info =
        run(json!([info(), {"severity": "Info", "code": "gsd/I536", "message": "m"}]));
    assert!(verdict(&other_info, &goldens, false).failed, "an info more");
    other_info = run(json!([info()]));
    other_info.selection = "{}\n".into();
    assert!(
        verdict(&other_info, &goldens, false).failed,
        "the selection"
    );
    let mut ledger = run(json!([info()]));
    ledger.ledger.push_str("x\n");
    let judged = verdict(&ledger, &goldens, false);
    assert!(judged.failed, "the ledger");
    assert!(
        judged
            .lines
            .contains(&"selection: 5 lines (golden)".to_string())
    );
    assert!(
        judged
            .lines
            .contains(&format!("ledger: differs from {GOLDENS}/ledger.json"))
    );
}

#[test]
fn a_golden_difference_names_the_lines_added_and_removed() {
    assert_eq!(
        difference("a\nb\nb\n", "b\nc\nb\nb\n"),
        ["  + b", "  + c", "  - a", "  2 lines added, 1 removed"]
    );
    assert!(
        difference("a\nb\n", "b\na\n").is_empty(),
        "a multiset: order moves nothing"
    );
    let many: String = (0..30).map(|n| format!("{n}\n")).collect();
    let lines = difference("", &many);
    assert_eq!(lines.len(), SHOWN + 1);
    assert_eq!(lines.last().unwrap(), "  30 lines added, 0 removed");
}

#[test]
fn bless_returns_the_three_goldens_only_when_the_check_is_clean_and_the_bound_holds() {
    let clean = verdict(&run(json!([info()])), &SampleGoldens::default(), true);
    assert!(!clean.failed);
    let goldens = clean.bless.unwrap();
    assert!(
        goldens.infos.contains("\"code\": \"gsd/I536\""),
        "{}",
        goldens.infos
    );
    assert!(goldens.infos.ends_with("]\n"));
    let mut broken = run(json!([info()]));
    broken.work.target_dir = true;
    let refused = verdict(&broken, &SampleGoldens::default(), true);
    assert!(refused.failed && refused.bless.is_none());
}

#[test]
fn the_run_s_absolute_paths_are_placeholders_in_every_golden() {
    let goldens = blessed();
    for text in [&goldens.selection, &goldens.infos, &goldens.ledger] {
        assert!(!text.contains("/scratch/"), "{text}");
    }
    assert!(goldens.infos.contains("held in <target>/spec/a.spec"));
    assert!(goldens.ledger.contains("\"from\": \"<source>\""));
    let paths = vec![
        ("/r".to_string(), "<run>"),
        ("/r/src".to_string(), "<source>"),
    ];
    assert_eq!(
        placeheld("/r/src/a and /r/b", &paths),
        "<source>/a and <run>/b"
    );
}

#[test]
fn a_compiled_unit_a_target_dir_or_an_uncited_test_breaks_the_bound() {
    let mut compiled = run(json!([info()]));
    compiled.work.compiled = vec!["specforge-ops-1a2b".into()];
    let judged = verdict(&compiled, &blessed(), false);
    assert!(judged.failed);
    assert!(
        judged
            .lines
            .iter()
            .any(|l| l.contains("1 units compiled") && l.contains("specforge-ops-1a2b"))
    );
    let mut target = run(json!([info()]));
    target.work.target_dir = true;
    assert!(verdict(&target, &blessed(), false).failed);
    let mut uncited = run(json!([info()]));
    uncited.work.ran.push("rust:x#y".into());
    let judged = verdict(&uncited, &blessed(), false);
    assert!(judged.failed);
    assert!(judged.lines.iter().any(|l| l.contains("rust:x#y")));
    let mut fewer = run(json!([info()]));
    fewer.work.ran.clear();
    assert!(
        !verdict(&fewer, &blessed(), false).failed,
        "an ignored cited test may not run"
    );
}

#[test]
fn absorb_work_other_than_three_compiles_and_one_plan_breaks_the_bound() {
    for work in [
        json!({"compiles": 4, "edit_plans": 1}),
        json!({"compiles": 3, "edit_plans": 2}),
        json!({}),
    ] {
        let mut found = run(json!([info()]));
        found.work.absorb = work.clone();
        let judged = verdict(&found, &blessed(), false);
        assert!(judged.failed, "{work}");
        assert!(
            judged
                .lines
                .iter()
                .any(|l| l.starts_with("  bound: absorb's work"))
        );
    }
}

#[test]
fn cited_test_ids_become_one_nextest_filterset() {
    assert_eq!(
        filterset(&[
            "rust:a::b#m::t".into(),
            "rust:specforge-cli::bin/x#u".into()
        ])
        .unwrap(),
        "(binary_id(=a::b) & test(=m::t)) | (binary_id(=specforge-cli::bin/x) & test(=u))"
    );
    assert_eq!(filterset(&[]).unwrap(), "none()");
    for refused in [
        "a::b#t",
        "rust:a::b",
        "rust:#t",
        "rust:a#t)",
        "rust:a#t | all()",
    ] {
        let why = filterset(&[refused.into()]).unwrap_err();
        assert!(why.contains(refused), "{why}");
    }
}

#[test]
fn the_fingerprint_directory_follows_the_cargo_profile_rust_test_runs() {
    for (profile, dir) in [
        ("dev", "debug"),
        ("test", "debug"),
        ("release", "release"),
        ("bench", "release"),
        ("ci", "ci"),
    ] {
        assert_eq!(profile_dir(profile), dir);
    }
    let declared =
        |argv: Value| json!({"items": [{"name": "rust-test", "argv": argv}]}).to_string();
    assert_eq!(
        rust_test_profile(&declared(json!([
            "cargo-nextest",
            "nextest",
            "run",
            "--cargo-profile",
            "ci"
        ])))
        .unwrap(),
        "ci"
    );
    assert_eq!(
        rust_test_profile(&declared(json!([
            "cargo-nextest",
            "nextest",
            "run",
            "--cargo-profile=ci"
        ])))
        .unwrap(),
        "ci"
    );
    assert_eq!(
        rust_test_profile(&declared(json!(["cargo-nextest"]))).unwrap(),
        "test"
    );
    let none = rust_test_profile(&json!({"items": []}).to_string()).unwrap_err();
    assert!(none.contains("no `rust-test` capability"), "{none}");
}

#[test]
fn rust_test_runs_the_test_profile_today() {
    let declared = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../crates/runtime/component/tests/declarations/rust_test/describe_capabilities.json"
    ))
    .unwrap();
    let profile = rust_test_profile(&declared).unwrap();
    assert_eq!(profile, "test");
    assert_eq!(profile_dir(&profile), "debug");
}
