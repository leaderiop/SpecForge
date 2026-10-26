//! A report file reaches the collector by its project path, however the call named it (ADR 0272).

use super::{Fixture, Runner, paths};
use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "dispatch_collector",
    verify = "each report file reaches the collector by its project path, however it was named"
)]
#[ignore = "flipped by T6"]
fn each_report_file_reaches_the_collector_by_its_project_path() {
    let fx = Fixture::new(vec![Runner::new("acme").proving("a", "t")]);
    fx.write("elsewhere/mine.json", "{}");
    let mut spellings = vec![fx.root().join("elsewhere/../elsewhere/mine.json")];
    #[cfg(unix)]
    let scratch = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        let at = scratch.path().join("link");
        std::os::unix::fs::symlink(fx.root(), &at).unwrap();
        spellings.push(at.join("elsewhere/mine.json"));
    }
    for (i, spelling) in spellings.iter().enumerate() {
        let files = [spelling.clone()];
        fx.no_run_reports(&files).unwrap();
        assert_eq!(paths(&fx.dispatched()[i]), ["elsewhere/mine.json"]);
    }
}
