//! The `specforge` doctor finds on PATH, the one the hooks and the MCP server `specforge install` writes run,
//! chosen by the test: a shell script that prints a version line. Unix only: the fakes are shell scripts.

#[cfg(unix)]
use crate::support::{Cli, TestProject};
#[cfg(unix)]
use std::path::PathBuf;

/// A directory holding only a `specforge` that prints `line` for `--version`, written once under
/// `<target tmp>/path-binary/<name>/` (to a temporary name, then renamed: parallel tests race safely).
#[cfg(unix)]
fn on_path(name: &str, line: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("path-binary")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let program = dir.join("specforge");
    if !program.exists() {
        let tmp = dir.join(format!("specforge.tmp.{}", std::process::id()));
        std::fs::write(&tmp, format!("#!/bin/sh\necho \"{line}\"\n")).unwrap();
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::rename(&tmp, &program).unwrap();
    }
    dir
}

#[cfg(unix)]
#[test]
fn doctor_warns_of_an_older_specforge_on_path() {
    let dir = TestProject::new();
    let path = on_path("older", "specforge 0.2.0 (protocol 1.6.0)");
    let program = path.join("specforge");

    let run = Cli::on(&dir).env("PATH", &path).run(&["doctor"]);

    assert_eq!(
        run.code, 0,
        "a warning does not fail doctor: {}",
        run.stdout
    );
    let section = &run.stdout[run.stdout.find("specforge on PATH:\n").expect(&run.stdout)..];
    assert!(
        section.contains(&format!(
            "[WARN] [W169] the specforge on PATH ({}) speaks extension protocol 1.6.0, older than this \
             specforge's {}",
            program.display(),
            specforge_protocol_types::PROTOCOL_VERSION
        )),
        "{section}"
    );
    assert!(
        section.contains("the hooks and the MCP server"),
        "{section}"
    );

    let json = Cli::on(&dir).env("PATH", &path).document(&["doctor"]).json;
    assert_eq!(json["specforge_on_path"]["code"], "W169", "{json}");
    assert_eq!(json["specforge_on_path"]["path"], program.to_str().unwrap());
}

#[cfg(unix)]
#[test]
fn doctor_says_nothing_of_a_specforge_on_path_of_this_protocol() {
    let dir = TestProject::new();
    let line = format!(
        "specforge 0.3.0 (9229143f5, protocol {})",
        specforge_protocol_types::PROTOCOL_VERSION
    );
    let path = on_path("current", &line);

    let run = Cli::on(&dir).env("PATH", &path).run(&["doctor"]);

    assert!(!run.stdout.contains("specforge on PATH:"), "{}", run.stdout);
}
