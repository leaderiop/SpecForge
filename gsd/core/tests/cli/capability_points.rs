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

/// `line` (the first line of `workflow` that starts with `var=`) run under `bash -c` in an empty project, with `bin`
/// first on the `PATH`: the stdout of `echo "$var"` and the stderr of the whole run.
fn run_line(workflow: &str, var: &str, bin: &std::path::Path) -> Option<(String, String)> {
    let has = |tool: &str| Command::new(tool).arg("--version").output().is_ok();
    if !has("bash") || !has("jq") {
        eprintln!("skipped: bash or jq is absent");
        return None;
    }
    let asset = std::fs::read_to_string(format!(
        "{}/assets/gsd-core/workflows/{workflow}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the workflow");
    let line = asset
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("{var}=")))
        .unwrap_or_else(|| panic!("{workflow} has no {var} line"));
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .current_dir(project("empty").root())
        .env("PATH", path)
        .args(["-c", &format!("{line}\necho \"${var}\"")])
        .output()
        .expect("bash runs");
    Some((
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// A directory holding `name` as an executable shell script.
fn shim(dir: &std::path::Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let file = dir.join(name);
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n")).expect("a shim");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("executable");
}

/// REVIEW-security-part1 m1: a loaded capability that refuses is warned about; "not loaded" (exit 2) stays silent.
#[test]
fn a_capability_that_fails_is_warned_about_not_hidden() {
    let bin = tempfile::tempdir().expect("a bin dir");
    shim(
        bin.path(),
        "specforge",
        "case \"$2\" in security-hooks) echo 'Invalid loop point' >&2; exit 1;; *) exit 2;; esac",
    );
    let Some((json, err)) = run_line("ship.md", "SHIP_PRE_HOOKS_JSON", bin.path()) else {
        return;
    };
    assert_eq!(json, r#"{"activeHooks":[],"skippedHooks":[]}"#);
    assert!(err.contains("security-hooks failed"), "{err}");
    assert!(!err.contains("ai-hooks"), "{err}");
}

/// REVIEW-security-part1 m1: a `jq` that cannot run does not turn into "no hooks" without a word.
#[test]
fn a_jq_that_fails_is_warned_about() {
    let bin = tempfile::tempdir().expect("a bin dir");
    shim(bin.path(), "specforge", "exit 2");
    shim(bin.path(), "jq", "exit 127");
    let Some((json, err)) = run_line("ship.md", "SHIP_PRE_HOOKS_JSON", bin.path()) else {
        return;
    };
    assert_eq!(json, "");
    assert!(err.contains("jq failed"), "{err}");
}
