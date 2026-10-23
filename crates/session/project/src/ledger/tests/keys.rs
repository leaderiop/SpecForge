//! One key for a source, however its path is spelled (ADR 0106 D3); the key rule itself is
//! `specforge_common::SourceRoot::key`'s.

use crate::ledger::SessionInputs;

#[cfg(unix)]
#[test]
fn a_reached_path_and_its_spellings_have_one_region() {
    let dir = tempfile::TempDir::new().unwrap();
    let dir = std::fs::canonicalize(dir.path()).unwrap();
    let real = dir.join("elsewhere");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("a.spec"), "").unwrap();
    // The spec root reached through a symlinked alias.
    let alias = dir.join("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();

    let config = specforge_common::ProjectConfig::default();
    let inputs = SessionInputs::opened(&alias, &config, Vec::new());
    let reach = inputs.reach([real.join("a.spec").as_path()]);

    assert_eq!(reach.regions, ["a.spec"]);
}
