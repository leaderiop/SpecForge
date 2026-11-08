//! The ported workflows, references, templates and command files against the vendored upstream files (gsd-core
//! v1.19.0, D-r42-4): line for line, a line may differ only where upstream's names `.planning`, `gsd-tools`,
//! `gsd_run` or `gsd-sdk`, or where a link left the tree (`REWRITTEN`).

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "common/port.rs"]
mod rule;
use rule::check_lines;

#[path = "common/pending.rs"]
mod pending;

#[path = "common/jq_rewrites.rs"]
mod jq_rewrites;

#[path = "common/quick_rewrites.rs"]
mod quick_rewrites;

#[path = "common/git_rewrites.rs"]
mod git_rewrites;

#[path = "common/review_rewrites.rs"]
mod review_rewrites;

#[path = "common/relic_rewrites.rs"]
mod relic_rewrites;

#[path = "common/args_rewrites.rs"]
mod args_rewrites;

#[path = "common/files_rewrites.rs"]
mod files_rewrites;

#[path = "common/answer_rewrites.rs"]
mod answer_rewrites;

#[path = "common/wait_rewrites.rs"]
mod wait_rewrites;

#[path = "common/prose_rewrites.rs"]
mod prose_rewrites;

#[path = "common/thread_rewrites.rs"]
mod thread_rewrites;

#[path = "common/workflow_rewrites.rs"]
mod workflow_rewrites;
use workflow_rewrites::{REWRITTEN, REWRITTEN_SPANS};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn vendored_root() -> PathBuf {
    PathBuf::from(gsd_kit::snapshot!())
}

/// The files under `dir` as paths relative to `base`, sorted.
fn files(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(base, &path, out);
        } else {
            let relative = path.strip_prefix(base).expect("under base");
            out.push(relative.to_string_lossy().into_owned());
        }
    }
    out.sort();
}

/// The embedded files that are not agent bodies.
pub fn assets() -> Vec<String> {
    let base = root().join("assets");
    let mut all = Vec::new();
    files(&base, &base, &mut all);
    all.retain(|p| !p.starts_with("agents/"));
    all
}

/// The vendored copy of an asset: upstream's own name, or `<name>.txt` for a file whose links leave the tree.
fn vendored(path: &str) -> PathBuf {
    let plain = vendored_root().join(path);
    if plain.is_file() {
        plain
    } else {
        vendored_root().join(format!("{path}.txt"))
    }
}

/// The body of an upstream file: a command file loses its frontmatter, which the slash binding carries.
fn body(path: &str) -> Vec<String> {
    let file = vendored(path);
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let mut lines: Vec<String> = text.split('\n').map(String::from).collect();
    if path.starts_with("commands/") {
        assert_eq!(lines[0], "---", "{path} has no frontmatter");
        let end = lines[1..]
            .iter()
            .position(|l| l == "---")
            .expect("a closing ---")
            + 1;
        lines.drain(..=end);
    }
    lines
}

#[test]
fn workflow_assets_diff_only_in_ported_passages() {
    let rx = Regex::new(r"\.planning|gsd-tools|gsd_run|gsd-sdk").expect("a regex");
    let mut checked = 0;
    for path in assets().into_iter().filter(|p| p.ends_with(".md")) {
        let asset = std::fs::read_to_string(root().join("assets").join(&path)).expect("an asset");
        let theirs = body(&path);
        relic_rewrites::assert_blanked(&path, &asset);
        let check = || {
            check_lines(&path, &path, &asset, &theirs, &rx, &|n| {
                REWRITTEN.contains(&(path.as_str(), n))
                    || jq_rewrites::LINES.contains(&(path.as_str(), n))
                    || review_rewrites::LINES.contains(&(path.as_str(), n))
                    || relic_rewrites::LINES.contains(&(path.as_str(), n))
                    || rule::in_spans(
                        REWRITTEN_SPANS
                            .iter()
                            .chain(jq_rewrites::SPANS)
                            .chain(quick_rewrites::SPANS)
                            .chain(git_rewrites::SPANS)
                            .chain(relic_rewrites::SPANS)
                            .chain(args_rewrites::SPANS)
                            .chain(files_rewrites::SPANS)
                            .chain(answer_rewrites::SPANS)
                            .chain(wait_rewrites::SPANS)
                            .chain(prose_rewrites::SPANS)
                            .chain(thread_rewrites::SPANS),
                        &path,
                        n,
                    )
                    || relic_rewrites::blanked(&path, n, &theirs[n])
            });
        };
        if pending::is_pending(&path) {
            pending::assert_still_pending(&path, check);
            continue;
        }
        check();
        checked += 1;
    }
    assert!(checked > 0, "no workflow asset was checked");
}

/// Every path waiting for its re-port is a file of the snapshot (a misspelt path would skip nothing).
#[test]
fn every_pending_path_is_vendored() {
    for path in pending::PENDING_RESYNC {
        assert!(vendored(path).is_file(), "{path} is not in the snapshot");
    }
}

/// The re-port of the tracked release is done when nothing waits. Ignored only between a snapshot move and its last
/// re-port, never on main.
#[test]
fn pending_resync_is_empty() {
    assert_eq!(pending::PENDING_RESYNC, &[] as &[&str]);
}

/// The files `@gsd/security` embeds that upstream's `gsd-core/` and `commands/` trees hold (its agents and capability
/// file are not read from those trees).
fn security_assets() -> Vec<String> {
    let base = root().join("../security/assets");
    let mut all = Vec::new();
    files(&base, &base, &mut all);
    all.retain(|p| p.starts_with("gsd-core/") || p.starts_with("commands/"));
    all
}

/// Vendored for its metadata alone: the prompt renders text of ours (D-r43-16), so the file is not an asset.
const VENDORED_ONLY: &[&str] = &["commands/gsd/update.md"];

#[test]
fn the_vendored_tree_lists_every_asset() {
    let source = std::fs::read_to_string(vendored_root().join("SOURCE.md")).expect("SOURCE.md");
    let mut vendored_files = Vec::new();
    for dir in ["gsd-core", "commands"] {
        let base = vendored_root();
        files(&base, &base.join(dir), &mut vendored_files);
    }
    let named: Vec<String> = vendored_files
        .iter()
        .map(|p| p.strip_suffix(".txt").unwrap_or(p).to_string())
        .collect();
    let security = security_assets();
    let assets = assets();
    for path in assets.iter().chain(&security) {
        assert!(named.contains(path), "{path} has no vendored copy");
        assert!(
            source.contains(path.as_str()),
            "SOURCE.md does not list {path}"
        );
    }
    for path in &named {
        assert!(
            assets.contains(path)
                || security.contains(path)
                || VENDORED_ONLY.contains(&path.as_str()),
            "{path} is vendored but not an asset of @gsd/core or @gsd/security"
        );
    }
}

/// Every upstream `X=$(gsd_run loop render-hooks <point> --raw)` line (or its `${X:-$(...)}` twin) is, here, the line
/// that asks each capability extension for its hooks; no stub is left. The `${X:-$(...)}` form asks too, not
/// `${X:-[]}`. A capability that fails is warned about, not hidden (exit 2 is "not loaded" and stays silent).
/// Upstream's `--after-fingerprint "$PHASE_DIR"` stays, with the phase's id for the directory. The line holds one
/// answer per extension, merged by no jq.
#[test]
fn every_hook_stub_asks_the_capability_extensions() {
    const NOTE: &str = "   # one compact JSON answer per line, one per capability extension: its activeHooks / skippedHooks are those of every line, in order";
    let twin = Regex::new(
        r"^(\s*)(\w+)=(\$\{\w+:-)?\$\(gsd_run loop render-hooks (\S+?)(?: --after-fingerprint (\S+))? --raw\)\}?$",
    )
    .expect("a regex");
    let mut asked = 0;
    let mut fingerprinted = Vec::new();
    for path in assets().into_iter().filter(|p| p.ends_with(".md")) {
        let asset = std::fs::read_to_string(root().join("assets").join(&path)).expect("an asset");
        assert!(
            !asset.contains("no hooks to render"),
            "{path}: a stub is left"
        );
        let mine: Vec<&str> = asset.split('\n').skip(1).collect();
        // a file waiting for its re-port has moved its lines: it must still hold each call, somewhere
        let pending = pending::is_pending(&path);
        for (n, theirs) in body(&path).iter().enumerate() {
            let Some(c) = twin.captures(theirs) else {
                continue;
            };
            let flag = match c.get(5).map(|m| m.as_str()) {
                None => String::new(),
                Some(dir) => {
                    fingerprinted.push(path.clone());
                    // the phase's id stands for its directory; autonomous.md's shell blocks name it `${PHASE_NUM}`
                    let phase = if path.ends_with("autonomous.md") {
                        "$MILESTONE/${PHASE_NUM}"
                    } else {
                        "$PHASE_ID"
                    };
                    format!(
                        " --after-fingerprint {}",
                        dir.replace("${PHASE_DIR}", phase)
                            .replace("$PHASE_DIR", phase)
                    )
                }
            };
            let call = format!(
                "$(for c in ai codebase ops security ui; do specforge gsd \"$c-hooks\" {}{flag} --pick . 2>/dev/null || [ $? -eq 2 ] || echo \"⚠ $c-hooks failed: its hooks are left out\" >&2; done)",
                &c[4]
            );
            let want = match c.get(3) {
                Some(open) => format!("{}{}={}{call}}}{NOTE}", &c[1], &c[2], open.as_str()),
                None => format!("{}{}={call}{NOTE}", &c[1], &c[2]),
            };
            if pending {
                // upstream's identical lines (one hook point asked twice) each need their own call
                let sites = body(&path).iter().filter(|l| *l == theirs).count();
                let calls = mine.iter().filter(|l| **l == want).count();
                assert!(calls >= sites, "{path}:{n}: the call is gone");
            } else {
                assert_eq!(mine[n], want, "{path}:{n}");
            }
            asked += 1;
        }
    }
    assert_eq!(asked, 24);
    // upstream passes the flag at exactly these two sites (verify-phase-goal.md omits it on purpose)
    fingerprinted.sort();
    assert_eq!(
        fingerprinted,
        [
            "gsd-core/workflows/autonomous.md",
            "gsd-core/workflows/verify-work.md"
        ]
    );
}

/// An empty or unparseable `SECURITY_GATE_JSON` (the command failed) is a block in every
/// workflow that reads it, and ship.md's step 1a no longer says the named branches cannot reach a failed check.
#[test]
fn a_security_gate_that_cannot_be_read_blocks() {
    let read = |path: &str| {
        std::fs::read_to_string(root().join("assets/gsd-core/workflows").join(path))
            .expect("a workflow")
    };
    let ship = read("ship.md");
    assert!(ship.contains("`SECURITY_SHIP_GATE_READ_FAILED`"));
    assert!(!ship.contains("cannot reach this state — their failure modes"));
    for path in ["verify-work.md", "execute-phase/steps/verify-phase-goal.md"] {
        let text = read(path);
        assert!(
            text.contains("`SECURITY_GATE_JSON` is empty") && text.contains("treat it as blocking"),
            "{path}: no rule for an unreadable gate"
        );
    }
}

/// The threat-ID duplicates come from `threat-ids`, not from `INIT` (a `specforge query`).
#[test]
fn the_threat_id_gate_names_threat_ids_not_init() {
    let read = |path: &str| {
        std::fs::read_to_string(root().join("assets/gsd-core/workflows").join(path))
            .expect("a workflow")
    };
    let execute = read("execute-phase.md");
    let parse = execute
        .lines()
        .find(|l| l.starts_with("Parse JSON for:"))
        .expect("the INIT field list");
    assert!(!parse.contains("threat_id_duplicate_count"), "{parse}");
    let gate = read("execute-phase/steps/threat-id-gate.md");
    assert!(!gate.contains("re-running init"));
    // a loaded @gsd/security that fails is not "no duplicates" (exit 2 is "not loaded")
    for text in [execute, read("plan-phase.md")] {
        assert!(text.contains(r#""error":"threat-ids failed""#));
        assert!(text.contains("never read it as no duplicates"));
    }
}
