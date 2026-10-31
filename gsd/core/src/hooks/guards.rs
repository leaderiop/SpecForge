//! Ported from gsd-core v1.19.0 hooks/gsd-read-guard.js and hooks/gsd-write-guard.js, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! `read_guard` (D-r44-7): upstream's reminder to Read a file before writing it, for the hosts whose runtime does not
//! enforce that. `write_guard`: upstream refuses a Write that shrinks a curated `.planning/` file to under 40% of its
//! lines; here the files are the spec files, measured in bytes (the old file's lines are not a fact; U52).
//!
//! Also ports gsd-core bin/lib modules: token-scanner.cjs.

use specforge_extension_sdk::prelude::*;

use super::facts::Facts;

/// Upstream's ratio: a Write under this share of the file is refused.
const SHRINK_PERCENT: u64 = 40;
/// Upstream's floor of 40 lines, as bytes: a smaller file may be rewritten whole.
const FLOOR_BYTES: u64 = 2_048;

/// Claude Code's runtime enforces read-before-edit, so the hook is installed on the other hosts only (ADR 0322).
pub(super) fn read_guard(f: &Facts) -> HookOutput {
    if !matches!(f.class.as_str(), "write" | "edit") {
        return HookOutput::default();
    }
    match f.paths.iter().find(|p| p.exists) {
        None => HookOutput::default(),
        Some(p) => HookOutput::allow().with_context(format!(
            "READ-BEFORE-EDIT REMINDER: You are about to modify \"{}\" which already exists. \
             If you have not already used the Read tool to read this file in the current session, \
             you MUST Read it first before editing. The runtime will reject edits to files that \
             have not been read. Use the Read tool on this file path, then retry your edit.",
            Facts::base(&p.path)
        )),
    }
}

/// A Write that would empty most of a spec file.
pub(super) fn write_guard(f: &Facts) -> HookOutput {
    if f.class != "write" {
        return HookOutput::default();
    }
    for p in &f.paths {
        let (Some(old), Some(new)) = (p.bytes, p.new_bytes) else {
            continue;
        };
        if !p.in_spec_root || !p.path.ends_with(".spec") || old < FLOOR_BYTES {
            continue;
        }
        if new * 100 >= old * SHRINK_PERCENT {
            continue;
        }
        let pct = (new * 100 + old / 2) / old;
        return HookOutput::deny(format!(
            "Write guard (gsd.write_guard): this Write would shrink '{}' from {old} bytes to {new} ({pct}% of current). \
             '{}' is a curated spec file; a whole-file Write this much smaller usually means the payload was built \
             from a partial read of the file and would destroy the sections outside that window (#973: a planner \
             collapsed ROADMAP.md 292 -> 16 lines this way). To fix: use Edit for a scoped change, or Read the full \
             file and include every section in the Write. A deliberate shrink goes through `specforge delete`, \
             `specforge set` and the gsd commands, which never pass through a Write tool.",
            p.path,
            Facts::base(&p.path)
        ));
    }
    HookOutput::default()
}

pub(super) fn declare(c: &mut ContributionsBuilder) {
    c.hook("read_guard", HookEvent::PreTool, |h| {
        h.description("Reminds a host that does not enforce it to Read a file before writing it")
            .hosts(&["codex", "cursor"])
            .tools(&["write", "edit"])
            .handler(|call| Ok(read_guard(&Facts::of(call))));
    });
    c.hook("write_guard", HookEvent::PreTool, |h| {
        h.description("Refuses a Write that shrinks a spec file to under 40% of its bytes")
            .tools(&["write"])
            .paths(&["{spec_root}/**/*.spec"])
            .handler(|call| Ok(write_guard(&Facts::of(call))));
    });
}

#[cfg(test)]
mod tests;
