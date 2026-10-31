//! The capability points of the ported workflows (plan 16 T4): the hook line asks every capability extension, and
//! with none loaded it answers what every branch reads as "no hooks".

use std::process::Command;

use super::harness::project;

#[test]
fn the_hook_line_is_empty_without_a_capability() {
    let has = |tool: &str| Command::new(tool).arg("--version").output().is_ok();
    if !has("bash") || !has("jq") {
        eprintln!("skipped: bash or jq is absent");
        return;
    }
    let asset = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/gsd-core/workflows/ship.md"
    ))
    .expect("ship.md");
    let line = asset
        .lines()
        .find(|l| l.trim_start().starts_with("SHIP_PRE_HOOKS_JSON="))
        .expect("the ship:pre line");
    let p = project("empty");
    let bin = tempfile::tempdir().expect("a bin dir");
    std::os::unix::fs::symlink(
        std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"),
        bin.path().join("specforge"),
    )
    .expect("a link");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .current_dir(p.root())
        .env("PATH", path)
        .args(["-c", &format!("{line}\necho \"$SHIP_PRE_HOOKS_JSON\"")])
        .output()
        .expect("bash runs");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        r#"{"activeHooks":[],"skippedHooks":[]}"#,
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
