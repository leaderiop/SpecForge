//! `specforge gsd gate-command <build|test>`: the gate's argv, from the files the host reads for it.

use super::commands_support::{gsd, json, refused};
use crate::harness::project;

/// The words of `--pick '.argv[]'`, one a line.
fn argv(p: &crate::harness::Project, args: &[&str]) -> Vec<String> {
    let mut all = args.to_vec();
    all.extend(["--pick", ".argv[]"]);
    let run = gsd(p, "gate-command", &all);
    assert_eq!(run.code, 0, "{}", run.all());
    run.stdout.lines().map(str::to_string).collect()
}

#[test]
fn gate_command_reads_the_project_root() {
    let p = project("execute");
    p.write("package.json", r#"{"scripts": {"build": "tsc"}}"#);
    assert_eq!(argv(&p, &["build"]), ["npm", "run", "build"]);
    assert_eq!(argv(&p, &["test"]), ["npm", "test"]);
    p.write("yarn.lock", "# yarn\n");
    assert_eq!(argv(&p, &["test"]), ["yarn", "test"]);
    // A lockfile below the root is not consulted, even with no packageManager field to decide: pnpm's would win.
    p.write("sub/pnpm-lock.yaml", "lockfileVersion: 9\n");
    assert_eq!(argv(&p, &["test"]), ["yarn", "test"]);
    // The packageManager field beats the lockfile.
    p.write(
        "package.json",
        r#"{"packageManager": "bun@1.1.0", "scripts": {"build": "tsc"}}"#,
    );
    assert_eq!(argv(&p, &["test"]), ["bun", "run", "test"]);
    let answer = json(&gsd(&p, "gate-command", &["build", "--format", "json"]));
    assert_eq!(answer["source"], "package", "{answer}");
    assert_eq!(answer["scope"], "post_merge", "{answer}");
    assert_eq!(answer["argv"][0], "bun", "{answer}");
    assert!(
        answer["note"]
            .as_str()
            .is_some_and(|note| note.contains("workflow.build_command")),
        "{answer}"
    );
    let human = gsd(&p, "gate-command", &["test"]);
    assert_eq!(human.stdout.trim_end(), "running: bun run test");
}

/// A manifest that is there and cannot be read is no "no gate": the answer says which, and fails.
#[cfg(unix)]
#[test]
fn a_manifest_present_but_unread_is_not_a_skipped_gate() {
    use std::os::unix::fs::symlink;
    let outside = tempfile::tempdir().expect("a directory");
    let makefile = outside.path().join("Makefile");
    std::fs::write(&makefile, "build:\n\ttouch pwned\ntest:\n\ttouch pwned\n").expect("written");
    let package = outside.path().join("package.json");
    std::fs::write(&package, r#"{"scripts": {"build": "x", "test": "x"}}"#).expect("written");
    let check = |file: &str, make: &dyn Fn(&std::path::Path)| {
        let p = project("execute");
        make(&p.root().join(file));
        for target in ["build", "test"] {
            let run = gsd(&p, "gate-command", &[target, "--format", "json"]);
            assert_eq!(run.code, 1, "{file} {target}: {}", run.all());
            let answer = json(&run);
            assert_eq!(answer["source"], "unread", "{file} {target}: {answer}");
            assert_eq!(answer["argv"], serde_json::json!([]), "{file} {target}");
            let note = answer["note"].as_str().expect("a note");
            assert!(note.contains(file), "{file} {target}: {note}");
            assert!(
                note.contains(&format!("workflow.{target}_command")),
                "{note}"
            );
        }
    };
    let link_to = |target: std::path::PathBuf| {
        move |at: &std::path::Path| symlink(&target, at).expect("a link")
    };
    check("Makefile", &link_to(makefile.clone()));
    check("package.json", &link_to(package.clone()));
    check("Cargo.toml", &link_to(outside.path().to_path_buf()));
    check("Cargo.toml", &link_to("/nonexistent/Cargo.toml".into()));
    check("Cargo.toml", &link_to("nowhere".into()));
    check("go.mod", &|at| {
        std::fs::create_dir(at).expect("a directory")
    });
    check("pyproject.toml", &link_to("/nonexistent/x".into()));
    // A name in another case is another name, not an unread one.
    let p = project("execute");
    p.write("Justfile", "test:\n\tx\n");
    assert_eq!(argv(&p, &["test"]), ["just", "test"]);
    // Over the host's text cap: there, and not read.
    let p = project("execute");
    p.write(
        "Makefile",
        &format!("build:\n\tx\n{}", "# y\n".repeat(30_000)),
    );
    let run = gsd(&p, "gate-command", &["build", "--format", "json"]);
    assert_eq!(run.code, 1, "{}", run.all());
    assert_eq!(json(&run)["source"], "unread");
    // A setting decides before any manifest is read.
    let p = project("execute");
    symlink(&makefile, p.root().join("Makefile")).expect("a link");
    p.settings(serde_json::json!({"workflow.build_command": "make all"}));
    assert_eq!(argv(&p, &["build"]), ["bash", "-c", "make all"]);
    // A link inside the project is read, and one to a file a runner earlier in the order decides first.
    let p = project("execute");
    p.write("mk.real", "test:\n\tx\n");
    symlink("mk.real", p.root().join("Makefile")).expect("a link");
    assert_eq!(argv(&p, &["test"]), ["make", "test"]);
    symlink("/nonexistent/x", p.root().join("go.mod")).expect("a link");
    assert_eq!(argv(&p, &["test"]), ["make", "test"]);
}

#[test]
fn gate_command_answers_the_setting_the_sources_and_an_empty_gate() {
    let p = project("execute");
    // Nothing to run: an empty answer and the note, exit 0.
    let none = gsd(&p, "gate-command", &["build"]);
    assert_eq!(none.code, 0, "{}", none.all());
    assert!(
        none.stdout.contains("No build command detected"),
        "{}",
        none.all()
    );
    assert!(argv(&p, &["build"]).is_empty());
    // A value with a leading hyphen is attached with `=`, as the gates write every value.
    assert_eq!(
        argv(
            &p,
            &[
                "build",
                "--xcode-project=-x.xcodeproj",
                r#"--simulators={"devices": {"com.apple.CoreSimulator.SimRuntime.iOS-18-2": [{"udid": "U"}]}}"#,
            ]
        ),
        [
            "xcodebuild",
            "build",
            "-project",
            "-x.xcodeproj",
            "-destination",
            "id=U"
        ]
    );
    // A python source's name stays one word, and an empty arg is absent.
    p.write("pyproject.toml", "[project]\nname = \"x\"\n");
    p.write("x;touch pwned;y.py", "print(1)\n");
    assert_eq!(
        argv(&p, &["build", "--xcode-project=", "--simulators="]),
        ["python", "-m", "py_compile", "x;touch pwned;y.py"]
    );
    assert_eq!(
        argv(
            &p,
            &[
                "test",
                "--scope",
                "regression",
                "--files",
                "a b.py\n-c.py\n"
            ]
        ),
        [
            "python",
            "-m",
            "pytest",
            "-q",
            "--tb=short",
            "--",
            "a b.py",
            "-c.py"
        ]
    );
    // The user's setting is the one shell string.
    p.settings(serde_json::json!({"workflow.build_command": "make -j4 && echo ok"}));
    assert_eq!(argv(&p, &["build"]), ["bash", "-c", "make -j4 && echo ok"]);
    // A word no line can carry is refused, never split.
    p.settings(serde_json::json!({"workflow.test_command": "a\nb"}));
    refused(
        &gsd(&p, "gate-command", &["test"]),
        "invalid_input",
        &["no line can carry"],
    );
    refused(
        &gsd(&p, "gate-command", &["build", "--scope", "regression"]),
        "invalid_input",
        &["regression"],
    );
}

/// The post-merge and regression gates' argv lines, as `specforge gsd reference` serves them, run in bash in a pnpm
/// project: the arrays hold pnpm's words, not npm's.
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
        ("post-merge-gate.md", "BUILD_ARGV", "pnpm|run|build"),
        ("post-merge-gate.md", "TEST_ARGV", "pnpm|test"),
        ("regression-gate-run.md", "REG_TEST_ARGV", "pnpm|test"),
    ] {
        let served = p.run(&[
            "gsd",
            "reference",
            &format!("gsd-core/workflows/execute-phase/steps/{step}"),
        ]);
        assert_eq!(served.code, 0, "{}", served.all());
        let line = served
            .stdout
            .lines()
            .find(|l| l.starts_with(&format!("{var}=()")))
            .unwrap_or_else(|| panic!("{step} {var}"));
        let out = std::process::Command::new("bash")
            .args([
                "-c",
                &format!("{line}\n(IFS='|'; printf '%s' \"${{{var}[*]}}\")"),
            ])
            .env("PATH", &path)
            .current_dir(p.root())
            .output()
            .expect("bash");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            want,
            "{step} {var}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
