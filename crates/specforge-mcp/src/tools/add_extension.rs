//! `specforge.add_extension`: install an extension (`specforge_ops::extension::add`).

use serde_json::json;

use crate::args::Arguments;
use crate::mutation::{MutationEvent, Replied, Written};
use crate::target::ProjectRef;
use crate::tool::ToolOutcome;

/// `specforge.add_extension`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Extension specifier
    specifier: String,
    /// Preview the install without changing any file
    dry_run: bool,
    /// Accept a registry package with no publisher signature (publisher verification skipped)
    allow_unsigned: bool,
}

/// `specforge.add_extension`: the shared add, its reply, the files it
/// wrote and `extension_added` (for an extension already there too,
/// `wasDuplicate`).
pub(crate) fn call(project: &ProjectRef<'_>, args: Args) -> Replied {
    use specforge_ops::extension::{self, AddOutcome, AddRequest, Origin, Trust};

    let allow_unsigned = args.allow_unsigned;
    let dry_run = args.dry_run;

    // The project the call installs into: the served one, or the one
    // `path` names.
    let root = project.root.to_path_buf();
    let served = project.is_served();
    // Once it is enabled, the server serves it (the next request brings the
    // project up to date); another project only has it on disk.
    let note = if served {
        "the server serves it from the next call on"
    } else {
        "installed in the project the path names; the server keeps serving its own"
    };
    let source = match extension::parse(&args.specifier) {
        Ok(source) => source,
        Err(error) => return Replied::refused_after(dry_run, error),
    };

    let registry = specforge_ops_registry::ConfiguredRegistry::for_project(&root, "add_extension");
    // The shared operation `specforge add` runs. An agent can't be asked,
    // so a publisher key change is refused rather than re-pinned.
    let request = AddRequest {
        root: &root,
        source,
        allow_unsigned,
        trust: Trust::Refuse,
        dry_run,
    };
    let added = extension::add(&request, &registry, project.runtime.as_ref());
    // What reading the registry configuration reported (E067, W140, I003), once the add asked a
    // registry, as `specforge add` shows it.
    let reported = registry.reported().to_vec();
    let added = match added {
        Ok(added) => added,
        // An install that failed after placing its module reports it.
        Err(error) => {
            return Replied::refused_after(dry_run, error).with_diagnostics(reported);
        }
    };
    let source_of = Origin::source;
    let (reply, was_duplicate) = match added.outcome {
        AddOutcome::Builtin {
            name,
            changed,
            peers_enabled,
        } => (
            json!({
                "extension": name,
                "installed": changed,
                "source": "builtin",
                "changed": changed,
                "peers_enabled": peers_enabled,
                "note": note,
            }),
            !changed,
        ),
        AddOutcome::Installed {
            name,
            version,
            sha256,
            key_id,
            origin,
        } => (
            json!({
                "extension": name,
                "installed": true,
                "version": version,
                "sha256": sha256,
                "key_id": key_id,
                "source": source_of(&origin),
                "note": note,
            }),
            false,
        ),
        // Already installed and enabled: an info response, nothing changed.
        AddOutcome::AlreadyPresent { name, version } => (
            json!({
                "extension": name,
                "installed": false,
                "already_present": true,
                "version": version,
                "message": format!("{name} {version} is already installed; specforge.json is unchanged"),
            }),
            true,
        ),
        AddOutcome::Planned {
            name,
            version,
            origin,
        } => {
            let plan = json!({
                "extension": name,
                "installed": false,
                "dry_run": true,
                "version": version,
                "source": source_of(&origin),
            });
            return Replied::preview(ToolOutcome::ok(plan).with_diagnostics(reported));
        }
    };
    let event = MutationEvent::ExtensionAdded {
        specifier: args.specifier,
        total_extensions: added.extensions_enabled,
        was_duplicate,
    };
    Replied::wrote(
        ToolOutcome::ok(reply).with_diagnostics(reported),
        Written::files(added.writes).with_event(event),
    )
}
