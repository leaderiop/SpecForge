use super::*;

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
    env.apply(&mut cmd, inherited.iter().map(ToString::to_string));
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
            ..Work::default()
        },
        ..SampleRun::default()
    };
    let stats = serde_json::json!({"entity_count": 4, "entity_counts": [{"kind": "requirement", "count": 3}, {"kind": "term", "count": 1}]});
    assert_eq!(
        head(&run, &stats),
        [
            "sample wasm-host: 3 old entities (feature 2, term 1)",
            "target: 4 entities (requirement 3, term 1)",
            "tests: 2 cited, 1 run, 1 held as skipped (ignored), 0 units compiled",
            "work: absorb compiles 3, edit plans 1; collects 5; checks 1",
        ]
    );
}
