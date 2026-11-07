//! The planner's stall watch: each watch cycle is one `specforge wait` call, since each host Bash call is a new shell,
//! and its activity is the `.spec` files a planner writes.

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "common/asset_code.rs"]
mod asset_code;
#[path = "common/files.rs"]
mod files;

use asset_code::{code_of, fenced_code};

/// Every asset as `(path under assets/, its lines after the header)`.
fn every_asset() -> Vec<(String, Vec<String>)> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    files::under(&assets)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p: PathBuf| {
            let relative = p
                .strip_prefix(&assets)
                .expect("under assets")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&p).expect("an asset");
            (
                relative,
                text.split('\n').skip(1).map(String::from).collect(),
            )
        })
        .collect()
}
/// Each `(asset, function)` where an asset's code names a shell function another asset defines.
fn functions_called_across_assets() -> Vec<(String, String)> {
    let definition = Regex::new(r"^\s*([a-z_][a-z0-9_]*)\(\)\s*\{").expect("a regex");
    let assets = every_asset();
    let mut defined = Vec::new();
    for (path, lines) in &assets {
        for (_, code) in fenced_code(lines) {
            if let Some(name) = definition.captures(code) {
                defined.push((path.clone(), name[1].to_string()));
            }
        }
    }
    let mut found = Vec::new();
    for (path, lines) in &assets {
        let code = code_of(lines);
        for (home, name) in &defined {
            let word = Regex::new(&format!(r"\b{name}\b")).expect("a regex");
            if home != path && code.iter().any(|(_, text)| word.is_match(text)) {
                found.push((path.clone(), name.clone()));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// A shell function is called only in the asset that defines it: each host Bash call is a new shell, so a function
/// defined in another file's block is `command not found` (exit 127) where it is called.
///
/// Pinned as it is: the stall watch's two functions, defined in stall-detection-helpers.md, are called from three
/// other assets, where the watch never answers. The fix makes this list empty.
#[test]
fn a_shell_function_is_called_only_in_the_asset_that_defines_it() {
    let found = functions_called_across_assets();
    let at = |path: &str, name: &str| (path.to_string(), name.to_string());
    assert_eq!(
        found,
        [
            at("gsd-core/references/planning-config.md", "gsd_stall_watch"),
            at(
                "gsd-core/workflows/plan-phase.md",
                "gsd_stall_should_recover"
            ),
            at("gsd-core/workflows/plan-phase.md", "gsd_stall_watch"),
            at(
                "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
                "gsd_stall_watch"
            ),
        ]
    );
}

/// The assets the planner's stall watch is described and run in.
const STALL_WATCH_ASSETS: [&str; 3] = [
    "gsd-core/workflows/plan-phase/steps/stall-detection-helpers.md",
    "gsd-core/workflows/plan-phase.md",
    "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
];

/// The stall watch's lines: the fenced `specforge wait --changed` lines of the helpers, and each line of the watch's
/// assets that names the watch and a phase-directory glob.
fn stall_watch_lines() -> (Vec<String>, Vec<String>) {
    let assets = every_asset();
    let lines_of = |want: &str| {
        &assets
            .iter()
            .find(|(path, _)| path == want)
            .unwrap_or_else(|| panic!("no asset {want}"))
            .1
    };
    let waits = fenced_code(lines_of(STALL_WATCH_ASSETS[0]))
        .into_iter()
        .filter(|(_, code)| code.contains("specforge wait") && code.contains("--changed"))
        .map(|(_, code)| code.to_string())
        .collect();
    let watch = Regex::new(r"gsd_stall_watch|watch cycle").expect("a regex");
    let mut globbed = Vec::new();
    for asset in STALL_WATCH_ASSETS {
        for (n, line) in lines_of(asset).iter().enumerate() {
            if watch.is_match(line) && (line.contains("PHASE_DIR") || line.contains("-PLAN.md")) {
                globbed.push(format!("{asset}:{}", n + 2));
            }
        }
    }
    (waits, globbed)
}

/// The stall watch watches the files a planner writes: GSD on SpecForge writes plans as entities in `.spec` files.
///
/// Pinned as it is: no `specforge wait` line, and the watch's activity glob is `"${PHASE_DIR}"'/*-PLAN.md'`, a
/// variable nothing assigns and files nothing writes, so a working planner reads quiet. The fix flips both.
#[test]
fn the_stall_watch_watches_the_spec_files() {
    let (waits, globbed) = stall_watch_lines();
    assert!(waits.is_empty(), "{waits:?}");
    assert_eq!(
        globbed,
        [
            "gsd-core/workflows/plan-phase.md:977",
            "gsd-core/workflows/plan-phase.md:1116",
            "gsd-core/workflows/plan-phase.md:1213",
        ]
    );
}
