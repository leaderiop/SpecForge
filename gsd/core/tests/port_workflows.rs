//! The ported workflows, references, templates and command files against the vendored upstream files (gsd-core
//! v1.19.0): the vendored tree lists every asset, nothing waits for a re-port, and the hook stubs ask the capability
//! extensions. `port::the_port_lock_maps_every_asset` maps each span by span (ADR 0332).

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "common/asset_code.rs"]
mod asset_code;
#[path = "common/command_word.rs"]
mod command_word;
#[path = "common/files.rs"]
mod files;
#[path = "common/ledger.rs"]
mod ledger;
#[path = "common/pending.rs"]
mod pending;
#[path = "common/port_map.rs"]
mod port_map;
#[path = "common/specforge_only.rs"]
mod specforge_only;

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

/// Every path waiting for its re-port is a file of the snapshot (a misspelt path would skip nothing), and none is an
/// asset's: an asset re-ports by span (ADR 0332 D6).
#[test]
fn every_pending_path_is_vendored() {
    let assets = port_map::assets();
    for path in pending::PENDING_RESYNC {
        let plain = vendored_root().join(path);
        assert!(
            plain.is_file() || vendored_root().join(format!("{path}.txt")).is_file(),
            "{path} is not in the snapshot"
        );
        assert!(
            !assets.iter().any(|a| port_map::upstream_path(a) == *path),
            "{path} is an asset: its spans resync in port.lock, not here"
        );
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
        let mine = port_map::asset_body(&path);
        let (theirs_body, _) = port_map::upstream_body(&path);
        let spans = port_map::map(&path);
        for (n, theirs) in theirs_body.iter().enumerate() {
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
            // the own span over upstream's line holds the call, once for each identical upstream line it covers
            let (own, up) = spans
                .iter()
                .find_map(|s| match s {
                    port_map::Span::Own { asset, up, .. } if up.contains(&n) => {
                        Some((asset.clone(), up.clone()))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{path}:{n}: upstream's hook line is kept as it was"));
            let sites = theirs_body[up].iter().filter(|l| *l == theirs).count();
            let calls = mine[own].iter().filter(|l| **l == want).count();
            assert!(calls >= sites, "{path}:{n}: the call is gone");
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
