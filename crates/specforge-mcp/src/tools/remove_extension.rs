//! `specforge.remove_extension`: remove an extension (`specforge_ops::extension::remove`).

use serde_json::{Value, json};

use crate::args::Arguments;
use crate::mutation::{Mutated, Written};
use crate::target::ProjectRef;
use crate::tool::ToolOutcome;

/// `specforge.remove_extension`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Extension name
    name: String,
    /// Force removal
    force: bool,
    /// Preview the removal, stranded entities included, without changing any file
    dry_run: bool,
}

pub(crate) fn call(project: &ProjectRef<'_>, args: Args) -> Mutated {
    let name = args.name.clone();
    let force = args.force;
    let dry_run = args.dry_run;

    // The shared operation, over the view of the call's project: its
    // dependents and its stranded entities, the served project's or those
    // of the project `path` names.
    let request = specforge_ops::extension::RemoveRequest {
        name: &name,
        force,
        dry_run,
    };
    match specforge_ops::extension::remove(&project.view(), &request) {
        Ok(outcome) => {
            let mut result = json!({
                "removed_extension": outcome.name,
                "success": true,
                "version": outcome.version,
                "stranded": outcome
                    .stranded
                    .iter()
                    .map(|entity| json!({"entity_id": entity.entity_id, "kind": entity.kind}))
                    .collect::<Vec<_>>(),
            });
            if outcome.dry_run {
                result["dry_run"] = Value::from(true);
                return Mutated::preview(ToolOutcome::ok(result));
            }
            Mutated::wrote(
                ToolOutcome::ok(result),
                Written::files(outcome.writes)
                    .with_entities(outcome.stranded.into_iter().map(|entity| entity.entity_id)),
            )
        }
        // A removal that failed after editing specforge.json reports it.
        Err(error) => Mutated::refused_after(dry_run, error),
    }
}
