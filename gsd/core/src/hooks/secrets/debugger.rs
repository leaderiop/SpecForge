//! Ported from gsd-core v1.19.0 hooks/gsd-secret-read-guard.js (`findDebuggerCliSpawn`), MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! The `--agent gsd-debugger` of a `claude` CLI spawn: a debugger started in a shell runs detached and returns nothing.

use std::collections::BTreeMap;

use super::scan::{MAX_NESTING_DEPTH, SHELL_INTERPRETERS, is_dash_c, resolve};
use super::shell::{Kind, Token, tokenize};

fn is_debugger(value: &str) -> bool {
    value.rsplit(':').next() == Some("gsd-debugger")
}

/// The agent name of the first `claude --agent gsd-debugger` in `command`, scanned like `find_secret_read`.
pub(super) fn find_debugger_spawn(command: &str, depth: usize) -> Option<String> {
    let (tokens, nested) = tokenize(command);
    if depth < MAX_NESTING_DEPTH {
        for sub in &nested {
            if let Some(hit) = find_debugger_spawn(sub, depth + 1) {
                return Some(hit);
            }
        }
    }
    let mut by_seg: BTreeMap<usize, Vec<&Token>> = BTreeMap::new();
    for t in tokens
        .iter()
        .filter(|t| matches!(t.kind, Kind::Word | Kind::Op))
    {
        by_seg.entry(t.seg).or_default().push(t);
    }
    for seg_tokens in by_seg.values() {
        let mut words = Vec::new();
        let mut k = 0;
        while k < seg_tokens.len() {
            let t = seg_tokens[k];
            k += 1;
            if t.kind == Kind::Op {
                if !t.dup && seg_tokens.get(k).is_some_and(|n| n.kind == Kind::Word) {
                    k += 1;
                }
            } else {
                words.push(t);
            }
        }
        let Some(cmd) = resolve(&words) else {
            continue;
        };
        let operands = &cmd.operands;
        if cmd.base == "eval" {
            if depth < MAX_NESTING_DEPTH {
                let script = operands
                    .iter()
                    .map(|w| w.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                if let Some(hit) = find_debugger_spawn(&script, depth + 1) {
                    return Some(hit);
                }
            }
            continue;
        }
        if SHELL_INTERPRETERS.contains(&cmd.base.as_str()) {
            if let Some(c) = operands.iter().position(|w| is_dash_c(&w.text))
                && let Some(script) = operands.get(c + 1)
                && depth < MAX_NESTING_DEPTH
                && let Some(hit) = find_debugger_spawn(&script.text, depth + 1)
            {
                return Some(hit);
            }
            continue;
        }
        let claude = cmd.base == "claude" || cmd.base == "claude.exe";
        if !claude {
            continue;
        }
        for (k, o) in operands.iter().enumerate() {
            if o.text == "--agent" {
                if let Some(v) = operands.get(k + 1).filter(|v| is_debugger(&v.text)) {
                    return Some(v.text.clone());
                }
            } else if let Some(v) = o.text.strip_prefix("--agent=").filter(|v| is_debugger(v)) {
                return Some(v.to_string());
            }
        }
    }
    None
}
