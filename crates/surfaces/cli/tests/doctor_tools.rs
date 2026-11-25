// `specforge doctor` names the programs a detected collector runs that this machine lacks (ADR 0389 D3), through the
// real binary and its vendored builtins, with a PATH the test chooses. Unix only: the fake tool is a shell script.

#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
use crate::support::{Cli, TestProject};
#[cfg(unix)]
use serde_json::Value;

/// A Rust crate that is also a TypeScript package, enabling packs whose collectors run cargo-nextest,
/// cargo-llvm-cov, Kani and api-extractor.
#[cfg(unix)]
fn project() -> TestProject {
    TestProject::new()
        .raw_config(
            r#"{"extensions": ["@code/core", "@rust/cargo", "@rust/source", "@rust/test", "@rust/coverage",
                "@rust/proof", "@javascript/source", "@javascript/api"]}"#,
        )
        .file(
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n")
        .file("package.json", r#"{"name": "demo", "version": "0.1.0"}"#)
        .file("tsconfig.json", "{}\n")
}

/// `<target tmp>/doctor-tools/<name>/`, holding `cargo-llvm-cov` when `with_llvm_cov`. The fake writes a marker file
/// beside itself if anything runs it. Parallel tests race safely: the script is written to a temporary name, made
/// executable, then renamed into place.
#[cfg(unix)]
fn path(name: &str, with_llvm_cov: bool) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("doctor-tools")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let tool = dir.join("cargo-llvm-cov");
    if with_llvm_cov && !tool.exists() {
        let script = format!("#!/bin/sh\ntouch '{}'\n", marker(&dir).display());
        let tmp = dir.join(format!("cargo-llvm-cov.tmp.{}", std::process::id()));
        std::fs::write(&tmp, script).unwrap();
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::rename(&tmp, &tool).unwrap();
    }
    dir
}

#[cfg(unix)]
fn marker(dir: &std::path::Path) -> PathBuf {
    dir.join("ran")
}

/// Doctor's `about: tools` findings with `path` as PATH, and its exit code.
#[cfg(unix)]
fn tools(p: &TestProject, path: &std::path::Path) -> (Vec<Value>, Option<i32>) {
    let out = Cli::on(p)
        .command(&["doctor", "--format", "json"])
        .env("PATH", path)
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    let found = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["about"] == "tools")
        .cloned()
        .collect();
    (found, out.status.code())
}

#[cfg(unix)]
fn programs(found: &[Value]) -> Vec<&str> {
    found
        .iter()
        .map(|f| f["program"].as_str().unwrap())
        .collect()
}

#[cfg(unix)]
#[specforge_test_macros::test(requirement = "FEATURE-021")]
fn doctor_names_each_missing_tool_as_a_note_and_still_exits_0() {
    let p = project();
    let (found, code) = tools(&p, &path("absent", false));
    assert_eq!(code, Some(0));
    assert_eq!(
        programs(&found),
        [
            "api-extractor",
            "cargo-kani",
            "cargo-llvm-cov",
            "cargo-nextest"
        ]
    );
    let installs = [
        (
            "api-extractor",
            "npm install --save-dev @microsoft/api-extractor typescript",
        ),
        (
            "cargo-kani",
            "cargo install --locked kani-verifier && cargo kani setup",
        ),
        (
            "cargo-llvm-cov",
            "cargo install cargo-llvm-cov cargo-nextest --locked",
        ),
        ("cargo-nextest", "cargo install cargo-nextest --locked"),
    ];
    for (finding, (program, install)) in found.iter().zip(installs) {
        assert_eq!(finding["status"], "info", "{finding}");
        assert_eq!(finding["code"], "tool_missing", "{finding}");
        assert!(
            finding["remediation"].as_str().unwrap().contains(install),
            "{program}: {finding}"
        );
        let codes: Vec<&str> = finding["codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap())
            .collect();
        assert!(!codes.is_empty(), "{finding}");
        assert!(codes.iter().all(|c| c.starts_with("code/")), "{finding}");
        if matches!(program, "cargo-llvm-cov" | "api-extractor") {
            assert!(codes.contains(&"code/W104"), "{finding}");
        }
    }
}

#[cfg(unix)]
#[specforge_test_macros::test(requirement = "FEATURE-021")]
fn doctor_finds_a_tool_on_path_without_running_it() {
    let p = project();
    let dir = path("llvm-cov", true);
    let (found, code) = tools(&p, &dir);
    assert_eq!(code, Some(0));
    assert_eq!(
        programs(&found),
        ["api-extractor", "cargo-kani", "cargo-nextest"]
    );
    assert!(!marker(&dir).exists(), "doctor ran cargo-llvm-cov");

    let human = Cli::on(&p)
        .command(&["doctor"])
        .env("PATH", &dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("Tools:\n"), "{text}");
    assert!(text.contains("`cargo-kani` is not found here"), "{text}");
    assert!(!marker(&dir).exists(), "doctor ran cargo-llvm-cov");
}
