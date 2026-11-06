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
    // A python source's name stays one word, and an empty arg is absent.
    p.write("pyproject.toml", "[project]\nname = \"x\"\n");
    p.write("x;touch pwned;y.py", "print(1)\n");
    assert_eq!(
        argv(&p, &["build", "--xcode-project", "", "--simulators", ""]),
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
