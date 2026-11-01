//! The capability points of the ported workflows (plan 16 T4): the hook line asks every capability extension, and
//! with none loaded it answers what every branch reads as "no hooks".

use std::process::Command;

use super::harness::project;

/// The `SHIP_PRE_HOOKS_JSON=` line of ship.md, run under `bash -c` in `project`, with `bin` first on the `PATH`:
/// the stdout of `echo "$SHIP_PRE_HOOKS_JSON"` and the stderr of the whole run.
fn ship_pre_in(project: &std::path::Path, bin: &std::path::Path) -> (String, String) {
    let asset = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/gsd-core/workflows/ship.md"
    ))
    .expect("ship.md");
    let line = asset
        .lines()
        .find(|l| l.trim_start().starts_with("SHIP_PRE_HOOKS_JSON="))
        .expect("the ship:pre line");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .current_dir(project)
        .env("PATH", path)
        .args(["-c", &format!("{line}\necho \"$SHIP_PRE_HOOKS_JSON\"")])
        .output()
        .expect("bash runs");
    (
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A directory holding `specforge` as a link to the binary under test.
fn linked_bin() -> tempfile::TempDir {
    let bin = tempfile::tempdir().expect("a bin dir");
    std::os::unix::fs::symlink(
        std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"),
        bin.path().join("specforge"),
    )
    .expect("a link");
    bin
}

#[test]
fn the_hook_line_is_empty_without_a_capability() {
    let p = project("empty");
    let bin = linked_bin();
    let (json, err) = ship_pre_in(p.root(), bin.path());
    assert_eq!(json, "", "{err}");
}

/// G4 (plan 05 T1): the hook line holds one compact JSON answer per capability extension that answers, in the
/// loop's order, with no jq: `@gsd/security` as built, and an `ai` capability a shim answers for.
#[test]
fn the_hook_line_holds_one_answer_per_capability_extension() {
    let p = project("empty").with_extension("@gsd/security", "GSD_SECURITY_WASM");
    let bin = tempfile::tempdir().expect("a bin dir");
    let real = std::env::var("SPECFORGE_BIN").expect("SPECFORGE_BIN");
    shim(
        bin.path(),
        "specforge",
        &format!(
            "case \"$2\" in ai-hooks) echo '{{\"activeHooks\":[{{\"capId\":\"ai\",\"kind\":\"step\"}}],\"skippedHooks\":[]}}';; \
             *) exec '{real}' \"$@\";; esac"
        ),
    );
    let (lines, err) = ship_pre_in(p.root(), bin.path());
    let answers: Vec<serde_json::Value> = lines
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{e}: {l}\n{err}")))
        .collect();
    assert_eq!(answers.len(), 2, "{lines}\n{err}");
    assert_eq!(answers[0]["activeHooks"][0]["capId"], "ai", "{lines}");
    for answer in &answers {
        assert!(answer["activeHooks"].is_array(), "{answer}");
    }
    // the security gate asks `security-hooks` for its active security entries (plan 05 review R9): the pick prints
    // the capId of each, as the answer's activeHooks hold them
    let out = Command::new(std::env::var("SPECFORGE_BIN").expect("SPECFORGE_BIN"))
        .current_dir(p.root())
        .args([
            "gsd",
            "security-hooks",
            "ship:pre",
            "--pick",
            ".activeHooks[capId=security].capId",
        ])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("specforge runs");
    let active: String = answers[1]["activeHooks"]
        .as_array()
        .expect("activeHooks")
        .iter()
        .filter(|h| h["capId"] == "security")
        .map(|_| "security\n")
        .collect();
    assert_eq!(String::from_utf8_lossy(&out.stdout), active, "{lines}");
}

/// `line` (the first line of `workflow` that starts with `var=`) run under `bash -c` in an empty project, with `bin`
/// first on the `PATH`: the stdout of `echo "$var"` and the stderr of the whole run.
fn run_line(workflow: &str, var: &str, bin: &std::path::Path) -> (String, String) {
    run_after("", workflow, var, bin)
}

/// [`run_line`] after `before` (shell that sets what the line reads).
fn run_after(before: &str, workflow: &str, var: &str, bin: &std::path::Path) -> (String, String) {
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
        .args(["-c", &format!("{before}\n{line}\necho \"${var}\"")])
        .output()
        .expect("bash runs");
    (
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
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
    let (json, err) = run_line("ship.md", "SHIP_PRE_HOOKS_JSON", bin.path());
    assert_eq!(json, "");
    assert!(err.contains("security-hooks failed"), "{err}");
    assert!(!err.contains("ai-hooks"), "{err}");
}

/// REVIEW-security-part1 M2 and m3: the `security-gate` line of every workflow that has one fails closed when the
/// command fails, and does not spawn it (or print an error) when no security hook is active, e.g. `@gsd/security`
/// is not loaded. Plan 05 review R9: "active" is `security-hooks`' own answer, its active entries with `capId`
/// security, not a substring of the hook line's text, which a skipped security hook also held.
#[test]
fn the_security_gate_line_fails_closed_and_stays_quiet_when_off() {
    for (workflow, hooks, point) in [
        ("ship.md", "SHIP_PRE_HOOKS_JSON", "ship:pre"),
        ("verify-work.md", "VERIFY_POST_HOOKS_JSON", "verify:post"),
        (
            "execute-phase/steps/verify-phase-goal.md",
            "VERIFY_POST_HOOKS_JSON",
            "verify:post",
        ),
    ] {
        let bin = tempfile::tempdir().expect("a bin dir");
        let (marker, asked) = (bin.path().join("spawned"), bin.path().join("asked"));
        // `security-hooks` answers the pick: one `security` line when ACTIVE is set; the gate fails
        shim(
            bin.path(),
            "specforge",
            &format!(
                "case \"$2\" in security-hooks) echo \"$@\" >> {}; [ -z \"$ACTIVE\" ] || echo security; exit 0;; esac\n\
                 echo \"$@\" >> {}; echo 'no phase' >&2; exit 1",
                asked.display(),
                marker.display()
            ),
        );
        // the hook line names a security hook that is only skipped
        let skipped = format!(
            r#"{hooks}='{{"activeHooks":[],"skippedHooks":[{{"capId":"security","kind":"gate"}}]}}'"#
        );
        let (json, _) = run_after(
            &format!("{skipped}\nexport ACTIVE=1"),
            workflow,
            "SECURITY_GATE_JSON",
            bin.path(),
        );
        let answer: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{workflow}: {e}: {json}"));
        assert_eq!(answer["block"], true, "{workflow}: {json}");
        assert!(answer["details"]["exists"].is_null(), "{workflow}: {json}");
        assert!(marker.exists(), "{workflow}: the gate was not asked");
        let question = std::fs::read_to_string(&asked).unwrap_or_default();
        assert!(
            question.contains(&format!("security-hooks {point}")),
            "{workflow}: {question}"
        );
        std::fs::remove_file(&marker).expect("the marker");
        let (json, err) = run_after(&skipped, workflow, "SECURITY_GATE_JSON", bin.path());
        assert_eq!((json.as_str(), err.as_str()), ("", ""), "{workflow}");
        assert!(
            !marker.exists(),
            "{workflow}: spawned with no active security hook"
        );
    }
}
