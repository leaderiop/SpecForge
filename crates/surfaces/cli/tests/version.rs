//! What `specforge --version` prints: a script reads its first two words, a person the build behind them.

use crate::support::{Cli, TestProject};
use specforge_protocol_types::PROTOCOL_VERSION;

#[test]
fn version_names_the_commit_and_the_protocol() {
    let dir = TestProject::new();
    let run = Cli::on(&dir).bare(&["--version"]);

    assert_eq!(run.code, 0, "{}", run.stderr);
    let line = run.stdout.strip_suffix('\n').unwrap();
    let rest = line
        .strip_prefix(&format!("specforge {} (", env!("CARGO_PKG_VERSION")))
        .unwrap_or_else(|| panic!("{line}"));
    let commit = rest
        .strip_suffix(&format!("protocol {PROTOCOL_VERSION})"))
        .unwrap_or_else(|| panic!("{line}"));
    // This checkout is a git one, unless the build was told the commit: either way the line names it.
    let hash = commit
        .strip_suffix(", ")
        .unwrap_or_else(|| panic!("{line}"));
    assert!(
        hash.len() == 9 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "{line}"
    );
}
