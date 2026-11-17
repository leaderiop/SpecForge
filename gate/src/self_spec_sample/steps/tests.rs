use super::*;
use serde_json::json;

#[test]
fn a_specforge_call_carries_no_cargo_credential() {
    let dir = tempfile::tempdir().unwrap();
    let env = Env::prepare(
        dir.path(),
        Path::new("/checkout"),
        &dir.path().join("target"),
    )
    .unwrap();
    let inherited = [
        "CARGO_REGISTRY_TOKEN",
        "CARGO_REGISTRIES_X_TOKEN",
        "CARGO_REGISTRIES_X_INDEX",
        "CARGO_REGISTRY_CREDENTIAL_PROVIDER",
        "CARGO_CREDENTIAL_PROVIDER",
        "CARGO_HTTP_PROXY",
        "CARGO_TARGET_DIR",
        "CARGO_PROFILE_DEV_DEBUG",
        "RUSTFLAGS",
        "PATH",
    ];
    let mut cmd = Command::new("specforge");
    env.apply(&mut cmd, inherited.iter().map(OsString::from));
    let envs: Vec<(String, Option<String>)> = cmd
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    let removed: Vec<&str> = envs
        .iter()
        .filter(|(_, v)| v.is_none())
        .map(|(k, _)| k.as_str())
        .collect();
    let mut expected = inherited[..6].to_vec();
    expected.push(REPORT_ENV);
    expected.extend(GIT_ELSEWHERE);
    expected.sort_unstable();
    let mut removed = removed;
    removed.sort_unstable();
    assert_eq!(removed, expected);
    let set = |name: &str| {
        envs.iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| v.clone())
    };
    assert_eq!(
        set("CARGO_TARGET_DIR"),
        Some(dir.path().join("target").to_string_lossy().into_owned())
    );
    assert_eq!(
        set("HOME"),
        Some(dir.path().join("home").to_string_lossy().into_owned())
    );
    assert_eq!(set(COPY_OF_ENV), Some("/checkout".to_string()));
    assert_eq!(COPY_OF_ENV, specforge_repo_root::COPY_OF_ENV);
    assert_eq!(TEST_REPORT_DIR_ENV, specforge_test::atexit::REPORT_DIR_ENV);
    assert_eq!(REPORT_ENV, specforge_test::atexit::REPORT_ENV);
    assert_eq!(
        set(TEST_REPORT_DIR_ENV),
        Some(
            dir.path()
                .join("test-reports")
                .to_string_lossy()
                .into_owned()
        )
    );
    assert_eq!(
        set("SPECFORGE_CONSENT_FILE"),
        Some(
            dir.path()
                .join("consent/consent.json")
                .to_string_lossy()
                .into_owned()
        )
    );
}

#[test]
fn the_consent_file_is_removed_first_in_a_private_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("consent")).unwrap();
    std::fs::write(dir.path().join("consent/consent.json"), "{}").unwrap();
    let env = Env::prepare(dir.path(), dir.path(), dir.path()).unwrap();
    assert!(!env.consent.exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.path().join("consent"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
    }
}

#[test]
fn the_run_ids_are_the_junit_testcases_that_were_not_skipped() {
    let junit = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="specforge" tests="3">
  <testsuite name="a::b" tests="2">
    <testcase name="m::t" classname="a::b" timestamp="x" time="0.1">
    </testcase>
    <testcase name="m::ignored" classname="a::b" time="0"><skipped/></testcase>
  </testsuite>
  <testsuite name="c" tests="1">
    <testcase name="n::u&amp;v" classname="c" time="0.2"/>
  </testsuite>
</testsuites>"#;
    assert_eq!(ran(junit), ["rust:a::b#m::t", "rust:c#n::u&v"]);
    assert!(ran("").is_empty());
}

#[test]
fn only_a_cargo_build_in_the_project_is_a_build_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!built_in(dir.path()));
    std::fs::create_dir_all(dir.path().join("target/nextest/specforge")).unwrap();
    assert!(!built_in(dir.path()), "nextest's store is no build");
    std::fs::write(dir.path().join("target/CACHEDIR.TAG"), "").unwrap();
    assert!(built_in(dir.path()));
}

#[test]
fn a_unit_written_after_the_mark_was_compiled_in_either_layout() {
    let dir = tempfile::tempdir().unwrap();
    // The shared layout (`.fingerprint/<unit>`) and the one with a directory per unit (cargo 1.100).
    let fingerprints = [
        ".fingerprint/old-1",
        ".fingerprint/new-2",
        "build/pkg/aaaa/fingerprint",
        "build/pkg/bbbb/fingerprint",
    ];
    for unit in fingerprints {
        std::fs::create_dir_all(dir.path().join(unit)).unwrap();
        std::fs::write(dir.path().join(unit).join("lib-x"), "").unwrap();
    }
    // A unit directory with no fingerprint (a binary's own output) is no unit.
    std::fs::create_dir_all(dir.path().join("build/pkg/cccc/out")).unwrap();
    let past = SystemTime::now() - std::time::Duration::from_secs(3600);
    for path in [
        ".fingerprint/old-1/lib-x",
        ".fingerprint/old-1",
        ".fingerprint/new-2",
        "build/pkg/aaaa/fingerprint/lib-x",
        "build/pkg/aaaa/fingerprint",
        "build/pkg/bbbb/fingerprint",
    ] {
        let file = std::fs::File::open(dir.path().join(path)).unwrap();
        file.set_modified(past).unwrap();
    }
    let units = units(dir.path()).unwrap();
    let names: Vec<&str> = units.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["new-2", "old-1", "pkg-aaaa", "pkg-bbbb"]);
    let mark = SystemTime::now() - std::time::Duration::from_secs(60);
    assert_eq!(built_since(&units, mark), ["new-2", "pkg-bbbb"]);
}

#[test]
fn no_fingerprint_where_cargo_builds_is_an_error_never_nothing_compiled() {
    let dir = tempfile::tempdir().unwrap();
    let missing = units(&dir.path().join("debug")).unwrap_err();
    assert!(missing.contains("cannot be judged"), "{missing}");
    std::fs::create_dir_all(dir.path().join("debug/.fingerprint")).unwrap();
    std::fs::create_dir_all(dir.path().join("debug/build/pkg/aaaa/out")).unwrap();
    let empty = units(&dir.path().join("debug")).unwrap_err();
    assert!(empty.contains("cannot be judged"), "{empty}");
}

/// absorb cites no test since ADR 0370 D8, so an empty `cited_tests` is a run with none; a missing key or another
/// shape is absorb's output changing, never read as "none cited".
#[test]
fn a_summary_without_a_cited_tests_array_is_refused() {
    assert_eq!(
        cited_tests(&json!({"cited_tests": []})).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        cited_tests(&json!({"cited_tests": ["rust:a#b"]})).unwrap(),
        ["rust:a#b"]
    );
    for summary in [
        json!({"converted": 1}),
        json!({"cited_tests": null}),
        json!({"cited_tests": "rust:a#b"}),
        json!({"cited_tests": ["rust:a#b", 2]}),
    ] {
        let why = cited_tests(&summary).unwrap_err();
        assert!(why.contains("no cited_tests array"), "{why}");
    }
}

#[test]
fn the_build_directory_is_cargos_own_else_its_target_directory() {
    let both = r#"{"target_directory": "/t", "build_directory": "/b", "packages": []}"#;
    assert_eq!(build_directory(both).unwrap(), PathBuf::from("/b"));
    let older = r#"{"target_directory": "/t", "packages": []}"#;
    assert_eq!(build_directory(older).unwrap(), PathBuf::from("/t"));
    assert!(build_directory("{}").is_err());
    assert!(build_directory("warning: not JSON").is_err());
}

#[test]
fn the_head_names_what_was_sampled_what_the_target_holds_and_the_work() {
    let run = SampleRun {
        selection: r#"{"feature": ["a", "b"], "term": ["t"]}"#.into(),
        work: Work {
            cited: vec!["rust:a#x".into(), "rust:a#y".into()],
            ran: vec!["rust:a#x".into()],
            absorb: serde_json::json!({"compiles": 3, "edit_plans": 1}),
            collects: 4,
            checks: 1,
            ..Work::default()
        },
        ..SampleRun::default()
    };
    let stats = json!({
        "entity_count": 4,
        "entity_counts": [{"kind": "requirement", "count": 3}, {"kind": "decision", "count": 1}],
    });
    assert_eq!(
        head(&run, &stats),
        [
            "sample wasm-host: 3 old entities (feature 2, term 1)",
            "target: 4 entities (requirement 3, decision 1)",
            "tests: 2 cited, 1 run, 1 cited, not run, 0 units compiled",
            "work: absorb compiles 3, edit plans 1; collects 4; checks 1",
        ]
    );
}

#[test]
fn the_checks_error_is_the_verdicts_reason() {
    let error = serde_json::json!({"severity": "Error", "code": "gsd/E101", "file": "spec/r.spec",
        "line": 2, "message": "maps to no phase"});
    let on_code = serde_json::json!({"severity": "Info", "code": "code/I402",
        "file": "crates/x/src/lib.rs", "line": 9, "message": "names `Fs`"});
    let found = SampleRun {
        diagnostics: vec![on_code, error],
        work: Work {
            absorb: serde_json::json!({"compiles": 3, "edit_plans": 1}),
            ..Work::default()
        },
        ..SampleRun::default()
    };
    let judged = verdict(&found, &SampleGoldens::default(), false);
    assert_eq!(
        failure(&judged).as_deref(),
        Some("gsd/E101 spec/r.spec:2: maps to no phase (read <target>/gate/self-spec-sample.log)")
    );
}

#[cfg(unix)]
#[test]
fn a_variable_that_is_not_unicode_is_read_lossily_and_named() {
    use std::os::unix::ffi::OsStringExt;
    let odd = |bytes: &[u8]| OsString::from_vec(bytes.to_vec());
    let names = [
        odd(b"CARGO_REGISTRY_\xff"),
        odd(b"OTHER_\xff"),
        "PATH".into(),
    ];
    assert_eq!(withheld(names.into_iter()), [odd(b"CARGO_REGISTRY_\xff")]);
    let vars = [
        (odd(b"OTHER_\xff"), OsString::from("v")),
        (OsString::from("VALUE"), odd(b"\xff")),
        (OsString::from("PATH"), OsString::from("/bin")),
    ];
    assert_eq!(
        not_unicode(vars.into_iter()),
        ["OTHER_\u{fffd}".to_string(), "VALUE".to_string()]
    );
}

#[test]
fn a_credentials_name_is_found_in_any_case_on_windows() {
    let lower = withheld(["cargo_registry_token".into()].into_iter());
    assert_eq!(lower.is_empty(), !cfg!(windows));
}

#[cfg(unix)]
#[test]
fn checks_output_is_a_diagnostic_array_and_a_failed_exit_is_explained_by_an_error() {
    use std::os::unix::process::ExitStatusExt;
    let (ok, failed) = (
        std::process::ExitStatus::from_raw(0),
        std::process::ExitStatus::from_raw(1 << 8),
    );
    let error = r#"[{"severity": "Error", "code": "E1"}]"#;
    assert_eq!(check_diagnostics(&ok, "[]", "").unwrap().len(), 0);
    assert_eq!(check_diagnostics(&failed, error, "").unwrap().len(), 1);
    let object = check_diagnostics(&ok, r#"{"diagnostics": []}"#, "").unwrap_err();
    assert!(object.contains("no diagnostic array"), "{object}");
    let text = check_diagnostics(&failed, "panicked", "thread main panicked").unwrap_err();
    assert!(
        text.contains("printed no JSON") && text.contains("thread main panicked"),
        "{text}"
    );
    let unexplained = check_diagnostics(&failed, "[]", "cannot read the lock").unwrap_err();
    assert!(
        unexplained.contains("failed with no error")
            && unexplained.contains("cannot read the lock"),
        "{unexplained}"
    );
}

#[test]
fn a_specforge_call_carries_no_bless_switch_and_insta_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let env = Env::prepare(
        dir.path(),
        Path::new("/checkout"),
        &dir.path().join("target"),
    )
    .unwrap();
    let inherited = [
        "SPECFORGE_BLESS",
        "GSD_BLESS",
        "UPDATE_EXPECT",
        "INSTA_FORCE_PASS",
        "INSTA_WORKSPACE_ROOT",
        "INSTA_UPDATE",
        "SPECFORGE_LOG",
    ];
    let mut cmd = Command::new("specforge");
    env.apply(&mut cmd, inherited.iter().map(OsString::from));
    let value = |name: &str| {
        cmd.get_envs()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.map(|v| v.to_string_lossy().into_owned()))
    };
    for name in &inherited[..5] {
        assert_eq!(value(name), Some(None), "{name} is removed");
    }
    assert_eq!(value("INSTA_UPDATE"), Some(Some("no".to_string())));
    assert_eq!(value("SPECFORGE_LOG"), None, "passed as inherited");
    assert_eq!(BLESS_SWITCHES[0], specforge_repo_root::BLESS_ENV);
}

#[test]
fn a_passed_sample_removes_its_copies_and_a_failed_one_keeps_and_names_them() {
    let dir = tempfile::tempdir().unwrap();
    let copies = [dir.path().join("src"), dir.path().join("dst")];
    let make = || {
        for copy in &copies {
            std::fs::create_dir_all(copy.join("crates")).unwrap();
        }
    };
    make();
    assert_eq!(settled(&copies, Ok(())), Ok(()));
    assert!(copies.iter().all(|copy| !copy.exists()));
    assert_eq!(settled(&copies, Ok(())), Ok(()), "none left to remove");
    make();
    let failed = settled(&copies, Err("gsd/E101 …".into())).unwrap_err();
    assert_eq!(
        failed,
        format!(
            "gsd/E101 …; kept for reading: {}, {}",
            copies[0].display(),
            copies[1].display()
        )
    );
    assert!(copies.iter().all(|copy| copy.is_dir()));
    std::fs::remove_dir_all(&copies[1]).unwrap();
    std::fs::remove_dir_all(&copies[0]).unwrap();
    assert_eq!(settled(&copies, Err("early".into())), Err("early".into()));
}

#[test]
fn every_call_finds_no_repository_around_the_copies_wherever_the_target_is() {
    let dir = tempfile::tempdir().unwrap();
    let run = dir.path().join("target/gate/self-spec-sample");
    let env = Env::prepare(&run, Path::new("/checkout"), &dir.path().join("target")).unwrap();
    let mut cmd = Command::new("specforge");
    let inherited = [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_CEILING_DIRECTORIES",
    ];
    env.apply(&mut cmd, inherited.iter().map(OsString::from));
    let value = |name: &str| {
        cmd.get_envs()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.map(std::ffi::OsStr::to_os_string))
    };
    // Both copies are directly in the run directory: the ceiling stops git's search above them.
    assert_eq!(value(GIT_CEILING), Some(Some(run.clone().into_os_string())));
    for name in GIT_ELSEWHERE {
        assert_eq!(value(name), Some(None), "{name} is removed");
    }
}
