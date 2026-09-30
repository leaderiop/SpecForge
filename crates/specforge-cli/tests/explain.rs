//! `specforge explain <CODE>`: the CLI adapter over the diagnostic catalog.
//! These pin what a user sees, so the catalog can move or grow without the
//! command's output or exit codes changing by accident.

use assert_cmd::Command;

fn specforge() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

#[test]
fn explain_prints_a_catalogued_code_case_insensitively() {
    let out = specforge().args(["explain", "e001"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0), "a catalogued code exits 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    // The code is printed bold; strip the escapes before matching.
    let plain = stdout.replace("\x1b[1m", "").replace("\x1b[0m", "");
    assert!(plain.contains("E001: Parse error"), "stdout: {plain}");
    assert!(plain.contains("Owner: core"), "stdout: {plain}");
}

#[test]
fn explain_rejects_an_unknown_code_with_format_hints() {
    let out = specforge().args(["explain", "Z999"]).output().unwrap();
    assert_eq!(out.status.code(), Some(1), "an unknown code exits 1");
    assert!(
        out.stdout.is_empty(),
        "nothing on stdout for an unknown code"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("unknown diagnostic code: Z999"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("E### (error), W### (warning), I### (info)"),
        "the format hint is printed: {stderr}"
    );
    assert!(
        stderr.contains("E900-E998, W900-W998 and I900-I998 are reserved for third-party"),
        "the third-party range hint is printed: {stderr}"
    );
}
