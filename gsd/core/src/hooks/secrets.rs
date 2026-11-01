//! Ported from gsd-core v1.19.0 hooks/gsd-secret-read-guard.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `secret_read_guard` (D-r44-8): a Read, a Grep or a shell command that reads `.env`, `.env.<suffix>` or `.secrets`
//! is refused, the templates (`.env.example`, `.sample`, `.template`, `.dist`) excepted. The shell command is read
//! from the tool's own `command` (upstream's tokenizer, ported), because the host's words lose where one command
//! ends and the next begins. Starting `gsd-debugger` as a CLI process is refused too.
//!
//! gsd-core modules this file ports: secrets.cjs. With gsd-core v1.19.0's #5627 (wrapper options, linear walk),
//! #5046 (bundled short flags) and #5513 (heredoc readings).

mod debugger;
mod git;
mod names;
mod scan;
mod shell;
#[cfg(test)]
mod tests;

use specforge_extension_sdk::prelude::*;

use super::facts::Facts;

const MAX_COMMAND_LENGTH: usize = 1024 * 1024;
const PATTERN: &str = ".env, .env.<suffix> (except .env.example/.sample/.template/.dist), .secrets";

fn deny(code: &str, tool: &str, target: &str) -> HookOutput {
    HookOutput::deny(match code {
        "command-too-large" => format!(
            "Secret read guard (gsd.secret_read_guard): this Bash command is over {MAX_COMMAND_LENGTH} characters and \
             cannot be checked for secret-file reads. Split it into smaller commands."
        ),
        "command-too-nested" => format!(
            "Secret read guard (gsd.secret_read_guard): this Bash command nests command substitutions or heredocs \
             deeper than {} levels and is denied rather than read, because what it runs cannot be checked for \
             secret-file reads. Split it into smaller commands.",
            shell::MAX_SPAN_DEPTH
        ),
        "command-too-complex" => format!(
            "Secret read guard (gsd.secret_read_guard): this Bash command takes more than {} tokens to read \
             across its nested scripts and heredoc bodies and is denied rather than read, because what it runs \
             cannot be checked for secret-file reads. Split it into smaller commands.",
            shell::MAX_SCAN_TOKENS
        ),
        "glob-too-complex" => format!(
            "Secret read guard (gsd.secret_read_guard): the Grep glob '{target}' expands to more than 64 \
             alternatives and cannot be checked for secret-file matches. Use a narrower glob."
        ),
        "debugger-cli-spawn" => format!(
            "Debugger spawn guard (gsd.secret_read_guard): this Bash command launches the debugger as a CLI process \
             ('--agent {target}'). A shell-spawned debugger runs detached, returns no structured result, and the \
             debug checkpoint never advances. Spawn it with the Agent tool instead: \
             Agent(subagent_type=\"gsd-debugger\", run_in_background=false, ...) and wait for its structured return."
        ),
        _ => format!(
            "Secret read guard (gsd.secret_read_guard): {tool} would read '{target}', which matches a protected \
             secret-file pattern ({PATTERN}). Secret values must not be read into the conversation. If you need a \
             specific value, ask the user for it; if you need the variable NAMES, read the non-secret template \
             (.env.example) instead."
        ),
    })
}

/// The command a shell tool runs: a string, or Codex's argv (`["bash", "-lc", script]`).
fn command_of(f: &Facts) -> String {
    match f.input.get("command") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(a)) if f.host == "codex" => {
            let words: Vec<&str> = a.iter().filter_map(|w| w.as_str()).collect();
            match words.as_slice() {
                [_, flag, script] if flag.ends_with('c') && flag.starts_with('-') => {
                    (*script).to_string()
                }
                _ => words.join(" "),
            }
        }
        _ => String::new(),
    }
}

pub(super) fn secret_read_guard(f: &Facts) -> HookOutput {
    let tool = f.tool.as_str();
    match f.class.as_str() {
        "read" => {
            let path = f.input_str("file_path").unwrap_or_default();
            if names::names_secret(path) {
                return deny("secret-read", tool, path);
            }
        }
        "search" => {
            let path = f
                .input_str("path")
                .or_else(|| f.input_str("file_path"))
                .unwrap_or_default();
            if names::names_secret(path) {
                return deny("secret-read", tool, path);
            }
            let glob = f.input_str("glob").unwrap_or_default();
            if !glob.is_empty()
                && let Some(code) = names::classify_glob(glob)
            {
                return deny(code, tool, glob);
            }
        }
        "shell" => {
            let command = command_of(f);
            if command.is_empty() {
                return HookOutput::default();
            }
            if command.len() > MAX_COMMAND_LENGTH {
                return deny("command-too-large", tool, "");
            }
            let (hit, stop) = shell::judged(|| {
                scan::find_secret_read(&command, 0)
                    .or_else(|| scan::find_secret_read_under_other_readings(&command))
            });
            // Judged before the read: a stopped scan did not read the rest, so no hit is no evidence.
            match stop {
                Some(shell::Stop::Nested) => return deny("command-too-nested", tool, ""),
                Some(shell::Stop::Complex) => return deny("command-too-complex", tool, ""),
                None => {}
            }
            if let Some(hit) = hit {
                return deny("secret-read", tool, &hit);
            }
            if let Some(spawn) = debugger::find_debugger_spawn(&command, 0) {
                return deny("debugger-cli-spawn", tool, &spawn);
            }
        }
        _ => {}
    }
    HookOutput::default()
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("secret_read_guard", HookEvent::PreTool, |h| {
        h.description("Refuses a read of .env, .env.<suffix> or .secrets by a Read, a Grep or a shell command")
            .tools(&["read", "search", "shell"])
            // a guard that cannot judge a command refuses it (F7); upstream fails open (#3911)
            .on_failure_deny()
            .handler(|call| Ok(secret_read_guard(&Facts::of(call))));
    });
}
