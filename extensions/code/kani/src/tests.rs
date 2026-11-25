use super::*;

const ONE_CRATE: &str = include_str!("../fixtures/one-crate.txt");
const WORKSPACE: &str = include_str!("../fixtures/workspace.txt");
const FAILED: &str = include_str!("../fixtures/failed.txt");
const NONE: &str = include_str!("../fixtures/none.txt");
const TRUNCATED: &str = include_str!("../fixtures/truncated.txt");

/// A harness block: its `Checking harness` line, the summary and the verdict (six lines).
fn block(name: &str, verdict: &str) -> String {
    format!(
        "Checking harness {name}...\n\nSUMMARY:\n ** 0 of 1 failed\n\nVERIFICATION:- {verdict}\n"
    )
}

/// Each harness's `(name, verdict)`, run by run.
fn verdicts(runs: &[Run]) -> Vec<Vec<(&str, Verdict)>> {
    runs.iter()
        .map(|run| {
            run.harnesses
                .iter()
                .map(|h| (h.name.as_str(), h.verdict))
                .collect()
        })
        .collect()
}

#[test]
fn one_crates_harness_is_one_run() {
    let runs = read(ONE_CRATE).unwrap();
    assert_eq!(
        runs,
        [Run {
            harnesses: vec![Harness {
                name: "proofs::check_add".to_string(),
                verdict: Verdict::Successful,
                failed_checks: Some(0),
                checks: Some(12),
                seconds: Some(0.3897019),
            }],
            successes: 1,
            failures: 0,
            total: 1,
        }]
    );
}

#[test]
fn a_workspace_is_one_run_per_complete_line_in_order() {
    let runs = read(WORKSPACE).unwrap();
    assert_eq!(
        verdicts(&runs),
        [
            vec![
                ("proofs::balance_never_negative", Verdict::Successful),
                ("proofs::transfer_conserves", Verdict::Successful),
            ],
            vec![("checks::seal_roundtrip", Verdict::Successful)],
        ]
    );
    assert_eq!(
        runs.iter()
            .map(|r| (r.successes, r.failures, r.total))
            .collect::<Vec<_>>(),
        [(2, 0, 2), (1, 0, 1)]
    );
    assert_eq!(runs[1].harnesses[0].seconds, Some(1.25));
}

#[test]
fn a_failed_harness_is_failed_and_counts_its_failed_checks() {
    let runs = read(FAILED).unwrap();
    assert_eq!(
        verdicts(&runs),
        [vec![
            ("proofs::check_add", Verdict::Successful),
            ("proofs::check_bound", Verdict::Failed),
        ]]
    );
    let [add, bound] = &runs[0].harnesses[..] else {
        panic!("two harnesses: {runs:?}");
    };
    assert_eq!((add.failed_checks, add.checks), (Some(0), Some(5)));
    assert_eq!((bound.failed_checks, bound.checks), (Some(1), Some(7)));
    assert_eq!(bound.seconds, Some(0.31));
    assert_eq!(
        (runs[0].successes, runs[0].failures, runs[0].total),
        (1, 1, 2)
    );
}

#[test]
fn only_the_verification_line_sets_a_verdict() {
    let checks = |status: &str, verdict: &str| {
        format!(
            "Checking harness p...\nCheck 1: p.cover.1\n\t - Status: {status}\n\t - Description: \"unwinding \
             assertion loop 0\"\n\nSUMMARY:\n ** 0 of 1 failed\n\nVERIFICATION:- {verdict}\nComplete - {} \
             successfully verified harnesses, {} failures, 1 total.\n",
            usize::from(verdict == "SUCCESSFUL"),
            usize::from(verdict == "FAILED"),
        )
    };
    for (status, verdict, expected) in [
        ("FAILURE", "SUCCESSFUL", Verdict::Successful),
        ("SATISFIED", "FAILED", Verdict::Failed),
        ("UNSATISFIABLE", "SUCCESSFUL", Verdict::Successful),
        ("SUCCESS", "FAILED", Verdict::Failed),
    ] {
        let runs = read(&checks(status, verdict)).unwrap();
        assert_eq!(
            verdicts(&runs),
            [vec![("p", expected)]],
            "{status} {verdict}"
        );
    }
}

#[test]
fn a_harness_without_a_summary_or_a_time_keeps_them_unknown() {
    let text = "Checking harness p...\nVERIFICATION:- SUCCESSFUL\n\nComplete - 1 successfully verified harnesses, \
                0 failures, 1 total.\n";
    let harness = &read(text).unwrap()[0].harnesses[0];
    assert_eq!(
        (harness.failed_checks, harness.checks, harness.seconds),
        (None, None, None)
    );
}

#[test]
fn no_harness_is_no_run_and_no_error() {
    for text in [NONE, "", "\n\n"] {
        assert_eq!(read(text), Ok(Vec::new()), "{text:?}");
    }
}

#[test]
fn a_cut_off_or_inconsistent_output_is_an_error_naming_its_line() {
    let one = block("a", "SUCCESSFUL");
    let complete = |s: usize, f: usize, t: usize| {
        format!("Complete - {s} successfully verified harnesses, {f} failures, {t} total.\n")
    };
    let after_one_crate = format!(
        "1 harnesses from line {} have no Complete line",
        ONE_CRATE.lines().count() + 1
    );
    let rows: Vec<(String, &str)> = vec![
        (TRUNCATED.to_string(), "(line 4) has no VERIFICATION line"),
        (
            format!("{ONE_CRATE}{}", block("b", "FAILED")),
            &after_one_crate,
        ),
        (
            format!("{one}{}", complete(1, 0, 2)),
            "line 7: Complete says",
        ),
        (
            format!("{one}{}", complete(0, 1, 1)),
            "line 7: Complete says",
        ),
        (
            format!("{}{}", block("a", "FAILED"), complete(1, 0, 1)),
            "line 7: Complete says",
        ),
        (
            format!("Checking harness a...\n{one}"),
            "line 2: the harness 'a' starts before the harness 'a' (line 1)",
        ),
        (
            format!("Checking harness a...\n{}", complete(0, 0, 0)),
            "line 2: Complete comes before the harness 'a' (line 1)",
        ),
        (
            format!("{one}Complete - x successfully verified harnesses, 0 failures, 1 total.\n"),
            "line 7: 'Complete - x",
        ),
        (
            format!(
                "{one}Complete - 1 successfully verified harnesses, 0 failures, 1 total, 2 more.\n"
            ),
            "line 7: 'Complete - 1",
        ),
        (
            "Checking harness a...\nSUMMARY:\n ** 1x of 3 failed\n".to_string(),
            "line 3: '** 1x of 3 failed'",
        ),
        (
            format!("{one}Verification Time: NaNs\n{}", complete(1, 0, 1)),
            "line 7: 'NaNs' is no verification time",
        ),
        (
            "VERIFICATION:- SUCCESSFUL\n".to_string(),
            "line 1: a VERIFICATION line",
        ),
        (
            "Checking harness a...\nVERIFICATION:- UNDETERMINED\n".to_string(),
            "line 2: 'UNDETERMINED' is no Kani verdict",
        ),
        (
            "Checking harness ...\n".to_string(),
            "line 1: a 'Checking harness' line",
        ),
    ];
    for (text, expected) in &rows {
        let why = read(text).unwrap_err();
        assert!(
            why.contains(expected),
            "{expected:?} not in {why:?} for {text:?}"
        );
        assert!(why.contains("line") || why.contains("Complete"), "{why}");
    }
}

#[test]
fn crlf_text_reads_like_lf() {
    for text in [ONE_CRATE, WORKSPACE, FAILED] {
        assert_eq!(read(&text.replace('\n', "\r\n")), read(text));
    }
}

#[test]
fn a_200000_line_output_reads_in_one_pass() {
    let harnesses = 200_000usize.div_ceil(6);
    let mut text = String::with_capacity(harnesses * 64);
    for at in 0..harnesses {
        text.push_str(&block(&format!("proofs::p{at}"), "SUCCESSFUL"));
    }
    text.push_str(&format!(
        "Complete - {harnesses} successfully verified harnesses, 0 failures, {harnesses} total.\n"
    ));
    assert!(text.lines().count() >= 200_000);
    let runs = read(&text).unwrap();
    assert_eq!(runs[0].harnesses.len(), harnesses);
    assert_eq!(
        runs[0].harnesses[harnesses - 1].name,
        format!("proofs::p{}", harnesses - 1)
    );
}
