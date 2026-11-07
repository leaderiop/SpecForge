//! The planner's stall watch, its cycles run as `stall-detection-helpers.md` holds them under bash: a planner that
//! writes where its mode writes is active, and a silent one is quiet.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{Shell, asset, q};

const HELPERS: &str = "gsd-core/workflows/plan-phase/steps/stall-detection-helpers.md";
/// The interval as a cycle spells it: minutes, filled from the settings.
const INTERVAL: &str = "\"{{setting:planner.stall_detect_interval_minutes|5}}m\"";
/// What the outline planner writes (chunked mode, §8.5.1).
const OUTLINE: &str = ".specforge/tmp/gsd-outline-*.md";

/// The helpers' cycle that watches `glob`, its interval `interval`, run with `HOME` at `home`.
fn cycle(glob: &str, interval: &str, home: &Path) -> String {
    let text = asset(HELPERS);
    let line = text
        .lines()
        .find(|l| l.contains("specforge wait") && l.contains(&format!("--changed '{glob}'")))
        .unwrap_or_else(|| panic!("no watch cycle watches {glob}"));
    assert!(line.contains(INTERVAL), "{line}");
    format!(
        "export HOME={}; {}",
        q(&home.to_string_lossy()),
        line.replace(INTERVAL, &format!("\"{interval}\""))
    )
}

/// A project (`specforge.json`, `spec/`) and a home beside it.
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a project");
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name": "stall", "version": "0.1.0", "spec_root": "spec"}"#,
    )
    .expect("specforge.json");
    std::fs::create_dir_all(dir.path().join("spec")).expect("spec/");
    std::fs::create_dir_all(dir.path().join("home")).expect("home/");
    dir
}

/// A planner appending a line to `file` every 300 ms, `times` times.
fn writing(file: PathBuf, times: usize) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for n in 0..times {
            std::thread::sleep(Duration::from_millis(300));
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file)
                .and_then(|mut f| writeln!(f, "| 01-0{n} | row |"))
                .expect("a row");
        }
    })
}

/// The outline planner writes only its outline, a temp file: its cycle watches it, so a slow outline is active and a
/// silent one quiet, where the `.spec` cycle sees neither.
#[test]
fn an_outline_planner_writing_slowly_is_never_stalled_and_a_silent_one_is() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let dir = project();
    let home = dir.path().join("home");
    let (made, err, code) = sh.run_in(
        dir.path(),
        &format!(
            "export HOME={}; specforge files temp --name gsd-outline-v1-01.md --pick .path",
            q(&home.to_string_lossy())
        ),
    );
    assert_eq!(code, 0, "{err}");
    let outline = PathBuf::from(made.trim());
    let run = |glob: &str| {
        let writer = writing(outline.clone(), 5);
        let ran = sh.run_in(dir.path(), &cycle(glob, "2s", &home));
        writer.join().expect("the planner");
        ran
    };
    let (out, err, code) = run(OUTLINE);
    assert_eq!((out.trim(), code), ("changed", 0), "{err}");
    // The `.spec` cycle never sees the outline: an outline planner it watched would be stalled.
    let (out, err, code) = run("**/*.spec");
    assert_eq!((out.trim(), code), ("deadline", 124), "{err}");
    let (out, err, code) = sh.run_in(dir.path(), &cycle(OUTLINE, "1s", &home));
    assert_eq!((out.trim(), code), ("deadline", 124), "{err}");
}

/// A cycle reads its glob from the project's root, so a planner's `.spec` write is activity whichever directory the
/// orchestrator's shell stands in.
#[test]
fn a_cycle_sees_a_spec_write_from_a_subdirectory() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let dir = project();
    let home = dir.path().join("home");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("src/");
    let writer = writing(dir.path().join("spec/plan.spec"), 5);
    let (out, err, code) = sh.run_in(&src, &cycle("**/*.spec", "2s", &home));
    writer.join().expect("the planner");
    assert_eq!((out.trim(), code), ("changed", 0), "{err}");
}
