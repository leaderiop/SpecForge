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

extern "C" fn on_exit() {
    let entries = registry::drain();
    if entries.is_empty() {
        return;
    }

    let binary_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unknown".to_string());

    let dir = report_dir();

    if let Err(e) = report::write_report(&dir, &binary_name, entries.clone()) {
        eprintln!("[specforge-test] failed to write report: {e}");
    }

    // Coverage summary: load graph, compute diff, print
    let graph_path = dir.join("graph.json");
    if let Some(graph) = coverage::load_graph(&graph_path) {
        let diffs = coverage::compute_coverage_diff(&graph, &entries);
        let _ = coverage::format_coverage_summary(&mut std::io::stderr(), &diffs, &graph.timestamp);
    }
}

fn report_dir() -> PathBuf {
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
