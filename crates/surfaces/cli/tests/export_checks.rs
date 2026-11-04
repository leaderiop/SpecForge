//! `specforge export --with-checks` (ADR 0336). No extension a test binary can load declares a checker before plan 06's
//! `@specforge/checks`, so the outcomes themselves are pinned in `specforge-ops` (`value_checks::`).

use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "export_agent_graph_format",
    verify = "--with-checks on a project with no checked value is the plain export"
)]
fn with_checks_on_a_project_with_no_checked_value_is_the_plain_export() {
    let project = TestProject::new()
        .without_config()
        .files(&[("main.spec", "spec s \"S\" {}\n")]);
    let export = |args: &[&str]| {
        let output = Cli::on(&project).command(args).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout).unwrap()
    };
    assert_eq!(
        export(&["export", "--no-schema", "--with-checks"]),
        export(&["export", "--no-schema"])
    );
}
