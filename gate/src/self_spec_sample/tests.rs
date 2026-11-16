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
        collects: 5,
        checks: 1,
    }
}

fn run(diagnostics: Value) -> SampleRun {
    SampleRun {
        diagnostics: diagnostics.as_array().unwrap().clone(),
        selection: "{\n  \"term\": [\n    \"t_a\"\n  ]\n}\n".into(),
        ledger: format!(
            "{{\n  \"from\": \"{SOURCE}\",\n  \"entries\": [\n    {{\"id\": \"t_a\"}}\n  ]\n}}\n"
        ),
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
fn an_info_on_the_collected_code_is_counted_and_never_golden() {
    let on_code = json!({"severity": "Info", "code": "code/I402",
        "span": {"file": "crates/x/src/lib.rs", "start_line": 9}, "message": "names `Fs`, which is no entity"});
    let found = run(json!([info(), on_code.clone(), on_code]));
    let judged = verdict(&found, &blessed(), false);
    assert!(
        !judged.failed,
        "the golden holds only the converted project's: {:?}",
        judged.lines
    );
    assert_eq!(judged.lines[0], "check: 0 errors, 0 warnings, 1 infos");
    assert!(
        judged.lines.contains(
            &"  2 infos on the collected code, counted, not golden: code/I402 2".to_string()
        ),
        "{:?}",
        judged.lines
    );
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
    // One group per binary, sorted; a test cited twice is named once.
    assert_eq!(
        filterset(&[
            "rust:z#t1".into(),
            "rust:a#t2".into(),
            "rust:z#t3".into(),
            "rust:z#t1".into()
        ])
        .unwrap(),
        "(binary_id(=a) & test(=t2)) | (binary_id(=z) & (test(=t1) | test(=t3)))"
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

#[test]
fn a_failed_sample_names_what_failed_it_never_the_counted_infos() {
    let on_code = json!({"severity": "Info", "code": "code/I402",
        "span": {"file": "crates/x/src/lib.rs", "start_line": 9}, "message": "names `Fs`"});
    let error = json!({"severity": "Error", "code": "gsd/E537", "file": "spec/m.spec", "line": 2,
        "message": "ships before it starts"});
    let found = run(json!([info(), on_code.clone(), error]));
    for bless in [false, true] {
        let judged = verdict(&found, &blessed(), bless);
        assert_eq!(
            judged.reason.as_deref(),
            Some("gsd/E537 spec/m.spec:2: ships before it starts"),
            "{:?}",
            judged.lines
        );
        // The counted line follows the judged ones.
        let at = |prefix: &str| judged.lines.iter().position(|l| l.starts_with(prefix));
        assert!(at("  gsd/E537") < at("  1 infos on the collected code"));
    }
    // A broken bound, then a golden that differs, when nothing is judged.
    let mut bound = run(json!([info(), on_code.clone()]));
    bound.work.target_dir = true;
    let reason = verdict(&bound, &blessed(), false).reason.unwrap();
    assert!(
        reason.starts_with("bound: the converted project holds a target directory"),
        "{reason}"
    );
    let mut differs = run(json!([info(), on_code]));
    differs.ledger.push_str("more\n");
    let reason = verdict(&differs, &blessed(), false).reason.unwrap();
    assert_eq!(
        reason,
        format!("ledger: differs from {GOLDENS}/ledger.json")
    );
    assert_eq!(
        verdict(&run(json!([info()])), &blessed(), false).reason,
        None
    );
}

#[test]
fn a_bless_never_writes_an_empty_golden() {
    for hollow_run in [
        SampleRun {
            selection: "{}\n".into(),
            ..run(json!([info()]))
        },
        run(json!([])),
        SampleRun {
            ledger: format!("{{\n  \"from\": \"{SOURCE}\",\n  \"entries\": []\n}}\n"),
            ..run(json!([info()]))
        },
    ] {
        let refused = verdict(&hollow_run, &SampleGoldens::default(), true);
        assert!(
            refused.failed && refused.bless.is_none(),
            "{:?}",
            refused.lines
        );
        let reason = refused.reason.unwrap();
        assert!(
            reason.starts_with("not blessed: the run found no entry for "),
            "{reason}"
        );
    }
    assert!(hollow("") && hollow("[]") && hollow("{}") && hollow("{\"entries\": []}"));
    assert!(!hollow("[1]") && !hollow("{\"term\": [\"t\"]}"));
}

#[test]
fn a_filterset_over_the_collector_limit_is_refused_as_the_gates_own() {
    let cited = |n: usize| -> Vec<String> {
        (0..n)
            .map(|i| {
                format!(
                    "rust:specforge-ops::operations#a_rather_long_module::a_long_test_name_{i:05}"
                )
            })
            .collect()
    };
    assert!(filterset(&cited(100)).unwrap().len() <= FILTER_MAX);
    let refused = filterset(&cited(1000)).unwrap_err();
    assert!(
        refused.starts_with(
            "the gate's own sample filter, for the 1000 tests absorb cited in 1 binaries"
        ),
        "{refused}"
    );
    assert_eq!(FILTER_MAX, specforge_protocol_types::FILTER_MAX);
}

#[test]
fn a_sample_that_cites_no_test_breaks_the_bound() {
    let mut none = run(json!([info()]));
    none.work.cited.clear();
    none.work.ran.clear();
    let judged = verdict(&none, &blessed(), false);
    assert!(judged.failed);
    assert_eq!(
        judged.reason.as_deref(),
        Some(
            "bound: absorb cited no test; the sample proves nothing by a test, so its run is skipped"
        )
    );
}
