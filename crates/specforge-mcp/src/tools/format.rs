//! `specforge.format`: format the spec files (`specforge_ops::format`).

use std::path::PathBuf;

use serde_json::{Value, json};

use specforge_common::project_root_of;

use crate::args::Arguments;
use crate::mutation::{Mutated, Written};
use crate::target::ProjectRef;
use crate::tool::{ErrorCode, McpError, ToolOutcome};
use specforge_ops::OpErrorKind;

/// `specforge.format`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Files or directories to format, relative to the project root (defaults to every spec file)
    paths: Vec<String>,
    /// Check only, don't modify
    check: bool,
    /// Return a before/after diff for each file that would change, without modifying it
    diff: bool,
    /// Write formatted output (defaults to false in check or diff mode)
    write: Option<bool>,
}

impl Args {
    /// Whether the call writes or only reports: `specforge format`'s one
    /// reading of check, diff and write.
    pub(crate) fn mode(&self) -> specforge_ops::format::Mode {
        specforge_ops::format::Mode::of_flags(self.check, self.diff, self.write)
    }
}

pub(crate) fn call(project: &ProjectRef<'_>, args: Args) -> Mutated {
    use specforge_ops::format::{self, Request};

    let diff = args.diff;
    // The one reading of check, diff and write: a run that does not write
    // is a preview.
    let mode = args.mode();
    let preview = !mode.writes();

    // The project the call formats: the served one, or the one `path`
    // names (a directory that is no project is its own root, formatted with
    // the defaults, as `specforge format` formats it); its config decides
    // what is formatted.
    let project_root = project_root_of(project.root);

    // The run `specforge format` makes. Relative paths name files under
    // the project root.
    let explicit: Vec<PathBuf> = args.paths.iter().map(|p| project_root.join(p)).collect();
    let outcome = format::run(&Request {
        root: &project_root,
        paths: &explicit,
        mode,
    });

    let shown = |path: &std::path::Path| path.display().to_string();
    let changed_files: Vec<String> = outcome.changes.iter().map(|c| shown(&c.path)).collect();
    let mut result = json!({
        "changed_files": changed_files,
        "total_checked": outcome.checked,
        "ok": outcome.ok(),
        "all_clean": outcome.clean(),
        "check_only": !mode.writes(),
        "diagnostics": specforge_common::diagnostics_json(&outcome.diagnostics),
    });
    if diff {
        let diffs: Vec<Value> = outcome
            .changes
            .iter()
            .map(|c| {
                let file_path = shown(&c.path);
                let stats = specforge_formatter::unified_diff(&file_path, &c.before, &c.after);
                json!({
                    "file_path": file_path,
                    "before": c.before,
                    "after": c.after,
                    "insertions": stats.insertions,
                    "deletions": stats.deletions,
                })
            })
            .collect();
        result["diffs"] = Value::from(diffs);
    }
    let written = |reply: ToolOutcome| match preview {
        true => Mutated::preview(reply),
        false => Mutated::wrote(reply, Written::files(outcome.writes())),
    };
    if outcome.succeeded() {
        return written(ToolOutcome::ok(result));
    }

    // Every other file was still formatted, and what was written is
    // reported; the call failed for these (read or write), with the kind
    // the files share (permission denied for locked files, not found for
    // missing ones), else an internal failure.
    let reasons: Vec<String> = outcome.failures.iter().map(ToString::to_string).collect();
    result["message"] = Value::from(reasons.join("; "));
    result["failed_files"] = Value::from(
        outcome
            .failures
            .iter()
            .map(|f| shown(f.path()))
            .collect::<Vec<_>>(),
    );
    result["failures"] = Value::from(
        outcome
            .failures
            .iter()
            .map(|f| {
                json!({
                    "file": shown(f.path()),
                    "operation": f.verb(),
                    "code": ErrorCode::from(f.kind()).as_str(),
                    "message": f.to_string(),
                })
            })
            .collect::<Vec<_>>(),
    );
    let message = result["message"].as_str().unwrap_or_default().to_string();
    let code = ErrorCode::from(outcome.failure_kind().unwrap_or(OpErrorKind::Internal));
    written(McpError::new(code, message).with_data(result).into())
}
