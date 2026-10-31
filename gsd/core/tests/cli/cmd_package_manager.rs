//! `specforge gsd resolve-package-manager <script>` (#5346): the package manager's command, from the files the host
//! reads for it.

use super::commands_support::{gsd, json, refused};
use crate::harness::project;

#[test]
fn resolve_package_manager_reads_the_project_root() {
    let p = project("execute");
    let run = gsd(&p, "resolve-package-manager", &["build"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        run.stdout.trim_end(),
        "npm run build",
        "nothing else is printed"
    );
    p.write("yarn.lock", "# yarn\n");
    assert_eq!(
        gsd(&p, "resolve-package-manager", &["test"])
            .stdout
            .trim_end(),
        "yarn test"
    );
    // A lockfile below the root is not consulted, even with no packageManager field to decide: pnpm's would win.
    p.write("sub/pnpm-lock.yaml", "lockfileVersion: 9\n");
    assert_eq!(
        gsd(&p, "resolve-package-manager", &["test"])
            .stdout
            .trim_end(),
        "yarn test"
    );
    // The packageManager field beats the lockfile.
    p.write("package.json", r#"{"packageManager": "bun@1.1.0"}"#);
    assert_eq!(
        gsd(&p, "resolve-package-manager", &["test"])
            .stdout
            .trim_end(),
        "bun run test"
    );
    let answer = json(&gsd(
        &p,
        "resolve-package-manager",
        &["build", "--format", "json"],
    ));
    assert_eq!(answer["command"], "bun run build", "{answer}");
    refused(
        &gsd(&p, "resolve-package-manager", &["a;rm"]),
        "invalid_input",
        &["not a script name"],
    );
}

/// P04 T5: the post-merge and regression gates' package-script lines, as `specforge gsd reference` serves them, run in
/// bash in a pnpm project: they print pnpm's command, not npm's.
#[test]
fn the_build_and_test_gates_run_the_projects_package_manager() {
    let p = project("execute");
    p.write(
        "package.json",
        r#"{"scripts": {"build": "tsc", "test": "vitest"}}"#,
    );
    p.write("pnpm-lock.yaml", "lockfileVersion: 9\n");
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    for (step, var, want) in [
        ("post-merge-gate.md", "BUILD_CMD", "pnpm run build"),
        ("post-merge-gate.md", "TEST_CMD", "pnpm test"),
        ("regression-gate-run.md", "REG_TEST_CMD", "pnpm test"),
    ] {
        let served = p.run(&[
            "gsd",
            "reference",
            &format!("gsd-core/workflows/execute-phase/steps/{step}"),
        ]);
        assert_eq!(served.code, 0, "{}", served.all());
        let lines: Vec<&str> = served
            .stdout
            .lines()
            .map(str::trim)
            .filter(|l| {
                l.starts_with(&format!("{var}=$(")) || l.starts_with(&format!("[ -n \"${var}\" ]"))
            })
            .collect();
        assert_eq!(lines.len(), 2, "{step} {var}: {lines:?}");
        let out = std::process::Command::new("bash")
            .args(["-c", &format!("{}\necho \"${var}\"", lines.join("\n"))])
            .env("PATH", &path)
            .current_dir(p.root())
            .output()
            .expect("bash");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            want,
            "{step} {var}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
