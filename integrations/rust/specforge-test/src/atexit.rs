use crate::ports::RealFs;
use crate::ports::{GraphReader, ReportWriter};
use crate::{coverage, registry, report};
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();

/// Ensures the atexit handler is registered exactly once.
/// Called by TestGuard::new on first construction.
pub fn ensure_registered() {
    INIT.call_once(|| {
        // Safety: atexit is POSIX-standard. The function pointer is valid
        // for the lifetime of the process.
        unsafe {
            libc::atexit(on_exit);
        }
    });
}

/// Append one JSONL record the moment a test finishes (C11-06): results
/// survive panic=abort, SIGKILL from a CI timeout, nextest kills, and
/// segfaults — none of which run atexit. The atexit handler only finalizes.
pub fn append_jsonl(entry: &registry::TestRecordEntry) {
    let binary_name = binary_name();
    let dir = report_dir();
    let fs = RealFs;
    if fs.create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(line) = serde_json::to_string(entry) {
        let path = dir.join(format!("{binary_name}.jsonl"));
        let _ = fs.append_line(&path, &line);
    }
}

fn binary_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// The final report's name: stable across rebuilds, so each run of a test
/// target replaces its previous report instead of leaving one per build
/// hash for `specforge collect --no-run` to count again. Cargo sets
/// `CARGO_PKG_NAME` when it runs tests; the target name is the binary's
/// stem without its build hash (`tests-87edb3de886b35bf` → `tests`).
fn report_name() -> String {
    let binary = binary_name();
    match std::env::var("CARGO_PKG_NAME") {
        Ok(package) => format!("{package}--{}", strip_build_hash(&binary)),
        Err(_) => binary,
    }
}

/// `name-<16 hex digits>` → `name`; anything else unchanged.
pub fn strip_build_hash(binary: &str) -> &str {
    match binary.rsplit_once('-') {
        Some((target, hash)) if hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit()) => {
            target
        }
        _ => binary,
    }
}

extern "C" fn on_exit() {
    let entries = registry::drain();
    if entries.is_empty() {
        return;
    }

    let dir = report_dir();

    if let Err(e) = report::write_report(&dir, &report_name(), &entries) {
        eprintln!("[specforge-test] failed to write report: {e}");
    }

    // Coverage summary: load graph through the port (C11-08), stamp verify
    // kinds (C11-07), compute diff, print — including unmatched-test
    // warnings (C11-02).
    let graph_path = dir.join("graph.json");
    let fs = RealFs;
    if let Some(graph) = fs.read_graph(&graph_path) {
        let stamped = coverage::stamp_verify_kinds(entries, &graph);
        // Only the entities this binary's tests recorded: a binary exercises
        // a slice of the project, so listing every other entity as
        // uncovered is noise. Project-wide gaps are `specforge analyze
        // coverage`'s job, after `specforge collect`.
        let recorded: std::collections::HashSet<&str> =
            stamped.iter().map(|e| e.entity_id.as_str()).collect();
        let diffs: Vec<_> = coverage::compute_coverage_diff(&graph, &stamped)
            .into_iter()
            .filter(|d| recorded.contains(d.entity_id.as_str()))
            .collect();
        if let Err(e) =
            coverage::format_coverage_summary(&mut std::io::stderr(), &diffs, &graph.timestamp)
        {
            eprintln!("[specforge-test] failed to write coverage summary: {e}");
        }
    }
}

fn report_dir() -> PathBuf {
    // `specforge collect` names the report directory when it runs the tests
    // (`@specforge/cargo-test`), wherever the target directory lives.
    if let Some(dir) = std::env::var_os("SPECFORGE_REPORT") {
        return PathBuf::from(dir);
    }
    // Walk up from the current exe to find target/, then use target/specforge/
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let mut dir = exe.parent()?;
            // exe is in target/debug/deps/ — walk up to target/
            while dir.file_name().is_some() {
                if dir.file_name().is_some_and(|n| n == "target") {
                    return Some(dir.join("specforge"));
                }
                dir = dir.parent()?;
            }
            None
        })
        .unwrap_or_else(|| PathBuf::from("target/specforge"))
}
