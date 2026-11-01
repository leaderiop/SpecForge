//! What `specforge --version` prints: a script reads its first two words, a person the build behind them.

use crate::support::{Cli, TestProject};

#[test]
fn version_names_the_package_version() {
    let dir = TestProject::new();
    let run = Cli::on(&dir).bare(&["--version"]);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        format!("specforge {}\n", env!("CARGO_PKG_VERSION"))
    );
}
