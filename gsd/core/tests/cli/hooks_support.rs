//! What the black-box hook tests share (D-r44-16): install into a temp project with a `HOME` of the test's own, read
//! the command string the install wrote for a hook, and run that string through `sh -c` as the host does.
//!
//! The host event shapes copy the kernel's recorded samples (`crates/surfaces/cli/src/hosts_tests/hook.rs`); the Codex
//! and Cursor ones are drafts, as the kernel's are.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::harness::{Project, Run};

pub const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

/// A temp `HOME` and a `bin/` with a `specforge` symlink: what a host's shell finds.
pub struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("a temp home");
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).expect("bin");
        let binary = std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN");
        std::os::unix::fs::symlink(binary, bin.join("specforge")).expect("a symlink");
        Home { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    fn bin(&self) -> PathBuf {
        self.dir.path().join("bin")
    }
}

/// `specforge install <host>` in the project, `HOME` this one, exit 0 asserted.
pub fn install_in(p: &Project, host: &str, home: &Home) -> Run {
    let h = home.path().display().to_string();
    let run = p.run_with(&["install", host, "--format", "json"], &[("HOME", &h)]);
    assert_eq!(run.code, 0, "install {host}: {}", run.all());
    run
}

/// Every string of `v` that is a hook command line (`specforge hook <route> …`).
fn commands_of(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) if s.contains("specforge hook ") => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| commands_of(x, out)),
        Value::Object(o) => o.values().for_each(|x| commands_of(x, out)),
        _ => {}
    }
}

fn settings_file(host: &str) -> &'static str {
    match host {
        "claude-code" => ".claude/settings.json",
        "codex" => ".codex/hooks.json",
        "cursor" => ".cursor/hooks.json",
        other => panic!("no host {other}"),
    }
}

/// Every hook command line the install wrote for `host`, in file order.
pub fn all_commands(p: &Project, host: &str) -> Vec<String> {
    let file = settings_file(host);
    let json: Value = serde_json::from_str(&p.read(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
    let mut out = Vec::new();
    commands_of(&json, &mut out);
    out
}

/// The command line the install wrote for the hook `route` (`gsd.write_guard`); fails naming the file when absent.
pub fn installed_command(p: &Project, host: &str, route: &str) -> String {
    let want = format!("specforge hook {route} ");
    let found: Vec<String> = all_commands(p, host)
        .into_iter()
        .filter(|c| c.contains(&want))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "{} holds {} lines for {route}: {found:?}",
        settings_file(host),
        found.len()
    );
    found.into_iter().next().unwrap_or_default()
}

/// Whether the install wrote a line for `route` on `host`.
pub fn has_command(p: &Project, host: &str, route: &str) -> bool {
    let want = format!("specforge hook {route} ");
    all_commands(p, host).iter().any(|c| c.contains(&want))
}

/// Runs the installed `command` through `sh -c`, as the host does: the project root as the directory, `HOME` the
/// test's, `specforge` first on `PATH`, `stdin` the event.
pub fn run_installed(p: &Project, command: &str, stdin: &str, home: &Home) -> Run {
    use std::io::Write;
    let path = format!(
        "{}:{}",
        home.bin().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(p.root())
        .env("HOME", home.path())
        .env("SPECFORGE_WASMTIME_CACHE", super::harness::compile_cache())
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run sh");
    child
        .stdin
        .take()
        .expect("a piped stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait for sh");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A host's event for a tool use (`phase` `pre` or `post`), as its hook protocol writes it.
pub fn tool_event(
    p: &Project,
    host: &str,
    phase: &str,
    tool: &str,
    input: &Value,
    response: Option<&Value>,
) -> String {
    let cwd = p.root().display().to_string();
    let mut v = if host == "cursor" {
        json!({"hook_event_name": format!("{phase}ToolUse"), "conversation_id": "s1", "tool_name": tool,
               "tool_input": input, "workspace_roots": [cwd]})
    } else {
        let name = if phase == "pre" {
            "PreToolUse"
        } else {
            "PostToolUse"
        };
        json!({"hook_event_name": name, "session_id": "s1", "cwd": cwd, "tool_name": tool, "tool_input": input})
    };
    if let Some(r) = response {
        v[if host == "cursor" {
            "tool_output"
        } else {
            "tool_response"
        }] = r.clone();
    }
    v.to_string()
}

/// A host's own name for a tool class.
pub fn tool_name(host: &str, class: &str) -> &'static str {
    match (host, class) {
        ("cursor", "shell") => "Shell",
        ("cursor", "agent") => "Task",
        ("codex", "edit") => "apply_patch",
        (_, "read") => "Read",
        (_, "write") => "Write",
        (_, "edit") => "Edit",
        (_, "shell") => "Bash",
        (_, "search") => "Grep",
        (_, "agent") => "Agent",
        (_, "web_fetch") => "WebFetch",
        (_, other) => panic!("no tool {other}"),
    }
}

/// The deny or advice of an answer in a host's own words: `(denied, reason, context)`.
pub fn verdict(host: &str, run: &Run) -> (bool, String, String) {
    assert_eq!(run.code, 0, "{}", run.all());
    if run.stdout.trim().is_empty() {
        return (false, String::new(), String::new());
    }
    let v: Value =
        serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.stdout));
    let text = |x: &Value| x.as_str().unwrap_or_default().to_string();
    if host == "cursor" {
        return (
            v["permission"] == "deny" || v["continue"] == false,
            text(&v["agent_message"]) + &text(&v["user_message"]),
            text(&v["additional_context"]),
        );
    }
    let s = &v["hookSpecificOutput"];
    (
        s["permissionDecision"] == "deny",
        text(&s["permissionDecisionReason"]),
        text(&s["additionalContext"]),
    )
}

/// Every file of the project but git's and the wasmtime cache, with the sha256 of its bytes.
pub fn tree(p: &Project) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            if entry.file_name() == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).expect("under root");
                let bytes = std::fs::read(&path).expect("read");
                out.insert(
                    rel.display().to_string(),
                    format!("{:x}", Sha256::digest(bytes)),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(p.root(), p.root(), &mut out);
    out
}

/// The paths whose bytes differ between two trees (added, removed or changed).
pub fn changed(before: &BTreeMap<String, String>, after: &BTreeMap<String, String>) -> Vec<String> {
    let mut out: Vec<String> = after
        .iter()
        .filter(|(k, v)| before.get(*k) != Some(*v))
        .map(|(k, _)| k.clone())
        .chain(before.keys().filter(|k| !after.contains_key(*k)).cloned())
        .collect();
    out.sort();
    out
}

/// Runs the hook `route` installed on `host` with the event, asserting that nothing but the extension's state
/// under `.specforge/state/gsd__core/` changed in the project (the black-box invariant: no r44 hook answers a plan).
pub fn run_hook(p: &Project, host: &str, route: &str, event: &str, home: &Home) -> Run {
    let command = installed_command(p, host, route);
    let before = tree(p);
    let run = run_installed(p, &command, event, home);
    let after = tree(p);
    let moved = changed(&before, &after);
    assert!(
        moved
            .iter()
            .all(|f| f.starts_with(".specforge/state/gsd__core/")),
        "{route} on {host} changed {moved:?}"
    );
    run
}

/// Whether `bash -n` accepts the hook line.
pub fn parses(command: &str) -> bool {
    Command::new("bash")
        .args(["-n", "-c", command])
        .status()
        .is_ok_and(|s| s.success())
}
