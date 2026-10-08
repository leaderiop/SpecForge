//! `specforge.extensions`: the installed extensions (`specforge_ops::extension::list`).

use serde_json::{Value, json};

use crate::args::NoArgs;
use crate::target::ProjectRef;
use crate::tool::ToolOutcome;

pub(crate) fn call(project: &ProjectRef<'_>, _args: NoArgs) -> ToolOutcome {
    // The shared listing, over the project view: what the project
    // compiled, its lock and the kinds its graph uses.
    let listing = specforge_ops::extension::list(&project.view());
    let extensions: Vec<Value> = listing.extensions.iter().map(|e| e.to_json()).collect();
    let lock_entries: Vec<Value> = listing
        .locked
        .iter()
        .map(|e| json!({ "name": e.name, "version": e.version }))
        .collect();
    ToolOutcome::ok(json!({
        "extensions": extensions,
        "lock_file_entries": lock_entries,
        "entity_kinds_in_graph": listing.kinds_in_graph,
    }))
}
