use super::*;

fn full_steps() -> Vec<Step> {
    let names: Vec<&str> = CHECKS
        .iter()
        .filter(|c| c.name != "builtins-verify")
        .map(|c| c.name)
        .collect();
    CHECKS
        .iter()
        .filter(|c| names.contains(&c.name))
        .map(|c| Step {
            compiles: c.compiles,
            after: c
                .after
                .iter()
                .filter_map(|a| names.iter().position(|n| n == a))
                .collect(),
        })
        .collect()
}

fn index(name: &str) -> usize {
    CHECKS
        .iter()
        .filter(|c| c.name != "builtins-verify")
        .position(|c| c.name == name)
        .unwrap()
}

/// Drive `schedule` the way `run` does, finishing the running check that `finish` picks; returns the order checks
/// started in, asserting the rules at every step.
fn simulate(steps: &[Step], finish: impl Fn(&[usize]) -> usize) -> Vec<usize> {
    let mut states = vec![State::Pending; steps.len()];
    let mut order = Vec::new();
    loop {
        let wave = schedule(steps, &states);
        for &k in &wave.start {
            assert!(
                steps[k].after.iter().all(|&a| states[a] == State::Passed),
                "{k} started before its after passed"
            );
            states[k] = State::Running;
            order.push(k);
        }
        let running: Vec<usize> = (0..steps.len())
            .filter(|&k| states[k] == State::Running)
            .collect();
        let compiling = running.iter().filter(|&&k| steps[k].compiles).count();
        assert!(
            compiling <= 1,
            "two compiling checks run at once: {running:?}"
        );
        if running.is_empty() {
            break;
        }
        states[finish(&running)] = State::Passed;
    }
    assert!(states.iter().all(|s| *s == State::Passed), "{states:?}");
    order
}

#[test]
fn no_two_compiling_checks_overlap_and_after_is_respected() {
    let steps = full_steps();
    // Whichever running check finishes first, the rules hold.
    for pick in [0usize, 1, 2] {
        let order = simulate(&steps, |running| running[pick.min(running.len() - 1)]);
        let at = |name: &str| order.iter().position(|&k| k == index(name)).unwrap();
        assert!(at("build") < at("tests") && at("build") < at("doctests"));
        assert!(at("tests") < at("dogfood"));
    }
    // clippy never starts while build runs: the first wave holds build or clippy, not both.
    let first = schedule(&steps, &vec![State::Pending; steps.len()]);
    let compiling: Vec<usize> = first
        .start
        .iter()
        .copied()
        .filter(|&k| steps[k].compiles)
        .collect();
    assert_eq!(compiling.len(), 1);
}

#[test]
fn a_failed_after_blocks_its_dependents() {
    let steps = full_steps();
    let mut states = vec![State::Pending; steps.len()];
    states[index("build")] = State::Failed;
    let wave = schedule(&steps, &states);
    assert!(wave.blocked.contains(&index("tests")));
    assert!(wave.blocked.contains(&index("doctests")));
    assert!(!wave.blocked.contains(&index("clippy")));
    states[index("tests")] = State::Blocked;
    let wave = schedule(&steps, &states);
    assert!(wave.blocked.contains(&index("dogfood")));
}

#[test]
fn the_report_names_failures_and_the_fix() {
    let report = Report {
        outcomes: vec![
            (
                "rustfmt",
                Outcome::Failed("`cargo fmt` exit status: 1".into()),
                Duration::from_secs(2),
            ),
            ("tests", Outcome::Blocked("build"), Duration::ZERO),
            ("links", Outcome::Skipped, Duration::ZERO),
            ("zero-entity", Outcome::Passed, Duration::from_secs(1)),
        ],
        log_dir: PathBuf::from("/nowhere"),
    };
    assert!(!report.passed());
    let mut table = Vec::new();
    report.print(Mode::Quick, &mut table).unwrap();
    let table = String::from_utf8(table).unwrap();
    assert!(
        table.contains("rustfmt")
            && table.contains("FAILED")
            && table.contains("fix: cargo fmt --all"),
        "{table}"
    );
    assert!(
        table.contains("blocked (by build)") && !table.contains("links"),
        "{table}"
    );
    let mut hook = Vec::new();
    report.print(Mode::Hook, &mut hook).unwrap();
    let hook = String::from_utf8(hook).unwrap();
    assert!(hook.starts_with("pre-commit: rustfmt: "), "{hook}");
}

/// A command that answers on stdout (`specforge … --format json`) prints its reason for a failure there: the error
/// and the check's log both carry it, the log the command's stderr too. On success stdout comes back.
#[cfg(unix)]
#[test]
fn a_failed_answer_keeps_what_it_printed() {
    let repo = crate::repo::this_repo();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("check.log");
    let (specforge, changes) = (OnceLock::new(), ChangeSet::default());
    let ctx = Ctx {
        repo: &repo,
        mode: Mode::Quick,
        changes: &changes,
        log: File::create(&path).unwrap(),
        specforge: &specforge,
    };
    let script = "printf '{\\n  \"error\": \"no report\",\\n  \"code\": \"E045\"\\n}\\n'; echo warned >&2; exit 1";
    let failed = ctx.answer(ctx.cmd("sh").args(["-c", script])).unwrap_err();
    assert!(
        failed.starts_with("`sh -c ")
            && failed.ends_with(r#"exit status: 1: { "error": "no report", "code": "E045" }"#),
        "{failed}"
    );
    let log = std::fs::read_to_string(&path).unwrap();
    assert!(
        log.contains("\"code\": \"E045\"") && log.contains("warned"),
        "{log}"
    );
    assert_eq!(
        ctx.answer(ctx.cmd("sh").args(["-c", "echo '[]'"])),
        Ok("[]\n".to_string())
    );
}

#[test]
fn only_names_a_comma_separated_list_of_checks() {
    assert!(named(None, "audit"));
    assert!(named(Some("audit"), "audit"));
    assert!(named(Some("doc-links,links,specs"), "links"));
    assert!(!named(Some("doc-links,links,specs"), "audit"));
    assert!(!named(Some("doc-links"), "links"));
}

#[test]
fn a_compile_turn_excludes_another_until_it_drops() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("compile.lock");
    let turn = compile_turn(&path).expect("the first takes its turn");
    let other = File::create(&path).unwrap();
    assert!(other.try_lock().is_err(), "a second must wait");
    drop(turn);
    assert!(other.try_lock().is_ok(), "the lock goes with its holder");
}

#[test]
fn a_compile_turn_that_cannot_be_taken_does_not_stop_the_check() {
    let dir = tempfile::tempdir().unwrap();
    assert!(compile_turn(&dir.path().join("missing/compile.lock")).is_none());
}
