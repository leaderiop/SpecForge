//! `specforge.rename`: rename an entity across the project (`specforge_ops::rename`).

use serde_json::{Value, json};

use crate::args::Arguments;
use crate::mutation::{Mutated, Written};
use crate::target::ProjectRef;
use crate::tool::{McpError, ToolOutcome};
use specforge_ops::OpErrorKind;

/// `specforge.rename`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Current entity ID
    entity_id: String,
    /// New entity ID
    new_name: String,
    /// Return the rename plan without changing any file
    dry_run: bool,
}

pub(crate) fn call(project: &ProjectRef<'_>, args: Args) -> Mutated {
    use specforge_ops::rename;
    let entity_id = args.entity_id.as_str();
    let new_name = args.new_name.as_str();
    let dry_run = args.dry_run;

    // Planned on the call's project as it is on disk (the target brought
    // the served project up to date, or compiled the project `path`
    // names), whose spans are relative to its spec root.
    let spec_root = project.spec_root().to_path_buf();
    let planned = rename::plan(
        &crate::tools::navigator(project.view()),
        entity_id,
        new_name,
    );
    let refused = |outcome: ToolOutcome| Mutated::refused_unless_preview(dry_run, outcome);
    let plan = match planned {
        Ok(plan) => plan,
        // The operation decided what kind of failure it is, and which
        // entity it is about; an invalid new ID is the argument's fault.
        Err(e) => {
            let argument = (e.kind == OpErrorKind::InvalidInput).then_some("new_name");
            let error = McpError::from(e);
            return refused(
                match argument {
                    Some(argument) => error.with_argument(argument),
                    None => error,
                }
                .into(),
            );
        }
    };

    let edit_json: Vec<serde_json::Value> = plan
        .edits
        .iter()
        .map(|e| {
            json!({
                "file": e.file,
                "line": e.line,
                "start_col": e.start_col,
                "end_col": e.end_col,
                "new_text": e.new_text,
            })
        })
        .collect();
    let mut result = json!({
        "old_name": entity_id,
        "new_name": new_name,
        "affected_files": plan.affected_files(),
        "edits": edit_json,
    });
    if dry_run {
        result["dry_run"] = Value::from(true);
        return Mutated::preview(ToolOutcome::ok(result));
    }
    // A failed write restores what it wrote: nothing is left written.
    let writes = match rename::apply(&plan, &spec_root) {
        Ok(writes) => writes,
        Err(e) => return refused(McpError::from(e).into()),
    };
    // The reply's `diagnostics` are what `specforge check` reports for the
    // project as it is on disk now, edits made since the last call
    // included (filled in once the target is brought up to date).
    Mutated::wrote(
        ToolOutcome::ok(result),
        Written::files(writes)
            .with_entities([new_name])
            .with_fresh_diagnostics(),
    )
}
