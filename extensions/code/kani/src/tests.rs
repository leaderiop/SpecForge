use super::*;

const ONE_CRATE: &str = include_str!("../fixtures/one-crate.txt");

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
