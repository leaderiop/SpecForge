//! The hooks `@gsd/core` declares (r44): they equal the roster of upstream's 28 hook files, each is valid for the
//! registry (a hook the registry refuses is dropped with a W021 and runs nowhere), and the commands their text names
//! are real.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gsd_core::build;
use gsd_core::hooks::roster::{Disposition, HOOK_ROSTER};
use gsd_core::prompts::roster::ROSTER;
use specforge_extension_sdk::SettingsItem;
use specforge_extension_sdk::testing::MockHost;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The `hooks` family of the vendored inventory manifest.
fn upstream_hooks() -> Vec<String> {
    let text =
        std::fs::read_to_string(root().join("../parity/upstream/v1.18.0/INVENTORY-MANIFEST.json"))
            .expect("manifest");
    let manifest: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    manifest["families"]["hooks"]
        .as_array()
        .expect("a family")
        .iter()
        .map(|h| h.as_str().expect("a file name").to_string())
        .collect()
}

#[test]
fn the_hook_roster_matches_upstreams_hooks() {
    let upstream = upstream_hooks();
    assert_eq!(upstream.len(), 28);
    let ours: Vec<&str> = HOOK_ROSTER.iter().map(|(f, _)| *f).collect();
    let mut sorted = ours.clone();
    sorted.sort_unstable();
    assert_eq!(ours, sorted, "the rows are in the manifest's order");
    assert_eq!(
        ours,
        upstream.iter().map(String::as_str).collect::<Vec<_>>()
    );
    let count = |f: fn(&Disposition) -> bool| HOOK_ROSTER.iter().filter(|(_, d)| f(d)).count();
    assert_eq!(
        count(|d| matches!(d, Disposition::Hooks(_))),
        12,
        "ported files"
    );
    assert_eq!(
        count(|d| matches!(d, Disposition::Dropped(_))),
        11,
        "dropped files (7 not ported, and the 3 Cursor renderings no hook feeds)"
    );
    assert_eq!(
        count(|d| matches!(d, Disposition::Rendering(_))),
        5,
        "host renderings (3 Cursor, 2 Windsurf)"
    );
    for (file, d) in HOOK_ROSTER {
        if let Disposition::Dropped(why) = d {
            assert!(why.len() > 10, "{file}: a reason");
        }
    }
}

/// The ids every ported file's row names.
fn rostered() -> BTreeSet<String> {
    HOOK_ROSTER
        .iter()
        .flat_map(|(_, d)| match d {
            Disposition::Hooks(ids) => ids.to_vec(),
            _ => Vec::new(),
        })
        .map(str::to_string)
        .collect()
}

#[test]
fn every_rostered_hook_is_declared_and_every_declared_hook_is_rostered() {
    let declared: BTreeSet<String> = MockHost::new(build())
        .declaration()
        .surfaces
        .hooks
        .iter()
        .map(|h| h.id.clone())
        .collect();
    assert_eq!(declared, rostered());
}

const CLASSES: [&str; 11] = [
    "read",
    "write",
    "edit",
    "shell",
    "search",
    "web_fetch",
    "web_search",
    "ask_user",
    "agent",
    "skill",
    "todo",
];

/// The registry's checks on a hook (`registry/src/surface/contributions.rs`), applied to what is declared: a hook it
/// refuses is dropped with a W021 and wired nowhere. The black-box tests also run `check` for the W021.
#[test]
fn every_declared_hook_is_valid() {
    let d = MockHost::new(build()).declaration();
    let bools: BTreeSet<&str> = d
        .settings
        .iter()
        .filter_map(|i| match i {
            SettingsItem::Setting(s) if s.field_type == "bool" => Some(s.name.as_str()),
            _ => None,
        })
        .collect();
    let pre_post = ["pre_tool", "post_tool", "pre_commit"];
    for h in &d.surfaces.hooks {
        let id = &h.id;
        assert!(
            !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                && !id.starts_with('_'),
            "{id}: snake_case"
        );
        assert_eq!(h.export, format!("hook__{id}"));
        assert!(
            (1..=30_000).contains(&h.timeout_ms),
            "{id}: timeout {}",
            h.timeout_ms
        );
        assert!(
            h.tools.iter().all(|t| CLASSES.contains(&t.as_str())),
            "{id}: tools {:?}",
            h.tools
        );
        let event = serde_json::to_value(h.event)
            .expect("an event")
            .as_str()
            .expect("a name")
            .to_string();
        if !pre_post.contains(&event.as_str()) {
            assert!(
                h.tools.is_empty() && h.paths.is_empty(),
                "{id}: tools or paths on {event}"
            );
        }
        if event == "statusline" {
            assert!(!h.writes, "{id}: a statusline writes no plan");
        }
        if let Some(when) = &h.when {
            assert!(
                bools.contains(when.as_str()),
                "{id}: when '{when}' is no bool setting of the extension"
            );
        }
        assert!(!h.description.is_empty(), "{id}: a description");
        assert_eq!(
            format!("{:?}", h.on_failure),
            "Allow",
            "{id}: upstream's ON_CRASH is ALLOW on every hook"
        );
    }
}

/// The code of the hooks' source: each `src/hooks/**/*.rs` but the tests, comments left out.
fn hook_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs")
                && !matches!(
                    path.file_name().and_then(|n| n.to_str()),
                    Some("tests.rs" | "accounting.rs" | "corpus.rs")
                )
            {
                let text = std::fs::read_to_string(&path).expect("a source");
                // comments cite upstream's files (`hooks/gsd-read-guard.js`); only the code can name a command
                let code: Vec<&str> = text
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .collect();
                out.push((path.display().to_string(), code.join("\n")));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root().join("src/hooks"), &mut out);
    assert!(!out.is_empty());
    out
}

/// A name that follows `prefix` in `text`, each time `prefix` occurs: lowercase letters, digits and dashes.
fn named_after(text: &str, prefix: &str) -> Vec<String> {
    text.match_indices(prefix)
        .map(|(i, _)| {
            text[i + prefix.len()..]
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .collect::<String>()
        })
        .filter(|n| !n.is_empty())
        .collect()
}

/// Every `/gsd-<x>` a hook's text names is a command `@gsd/core` serves, and every `specforge gsd <cmd>` is a command
/// it declares. (`tests/cli/hooks_*.rs` run each command a hook names.)
#[test]
fn no_hook_names_an_unknown_command() {
    let d = MockHost::new(build()).declaration();
    let commands: BTreeSet<String> = d
        .surfaces
        .commands
        .iter()
        .map(|c| c.id.replace('_', "-"))
        .collect();
    assert!(commands.contains("progress-report"), "{commands:?}");
    let served: BTreeSet<&str> = ROSTER
        .iter()
        .filter(|(_, o)| o.is_core())
        .map(|(s, _)| *s)
        .collect();
    for (file, text) in hook_sources() {
        for stem in named_after(&text, "/gsd-") {
            assert!(
                served.contains(stem.as_str()),
                "{file}: /gsd-{stem} is not a command of @gsd/core"
            );
        }
        for cmd in named_after(&text, "specforge gsd ") {
            assert!(
                commands.contains(&cmd),
                "{file}: `specforge gsd {cmd}` is not a declared command"
            );
        }
    }
}

/// REVIEW-r44 M6: a `Rendering(host)` row that counts as built has a rendered line for its event on that host.
#[test]
fn a_cursor_rendering_row_has_its_line_in_the_cursor_install() {
    const GOLDEN: &str = include_str!("golden/install/cursor/hooks.json");
    for (file, d) in HOOK_ROSTER {
        if let Disposition::Rendering("cursor") = d {
            match *file {
                "gsd-cursor-pre-tool.js" => assert!(GOLDEN.contains("\"preToolUse\""), "{file}"),
                "gsd-cursor-post-tool.js" => assert!(GOLDEN.contains("\"postToolUse\""), "{file}"),
                // the session start is the community hook's, off at install: `cursor_renders_the_session_start_of_the_community_hook`
                // (cli/install_hosts.rs) installs with it on and reads the installed hooks.json
                "gsd-cursor-session-start.js" => {}
                other => panic!("{other}: a Cursor row with no rendered event"),
            }
        }
    }
}
