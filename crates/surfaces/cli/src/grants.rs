//! `specforge grants` (ADR 0335 D2): what the project's extensions may make the host do, and this machine's
//! approvals of it, kept in the user's consent store. `approve` and `revoke` change them; typing `approve` is the
//! consent, so nothing is asked.

use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::terminal::{Terminal, errln, outln};
use specforge_ops::analysis::grants::{self, Change, GrantState, GrantsListing, StoreListing};
use specforge_ops::consent::UserConsent;
use specforge_ops::view::ProjectView;

// `specforge grants`'s command line: the listing, or `approve` and `revoke`.
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct Args {
    #[command(subcommand)]
    action: Option<Action>,

    #[command(flatten)]
    list: ListArgs,
}

#[derive(clap::Subcommand)]
enum Action {
    /// List each capability, its grant here and the project's named roots (the default)
    List(ListArgs),
    /// Approve an extension's scopes on this machine (every scope that needs it when none is named), or a root
    Approve(ApproveArgs),
    /// Revoke an extension's approvals (every scope when none is named), a root's, or the whole project's
    Revoke(RevokeArgs),
}

#[derive(clap::Args)]
struct ListArgs {
    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// List every approval the consent store holds, for every project
    #[arg(long)]
    store: bool,

    #[command(flatten)]
    output: OutputArg,
}

#[derive(clap::Args)]
struct ApproveArgs {
    /// The extension, as `specforge grants` lists it
    #[arg(required_unless_present = "root")]
    extension: Option<String>,

    /// Its scopes, as `specforge grants` lists them
    scopes: Vec<String>,

    /// A named root of specforge.json outside the project, instead of an extension's scopes
    #[arg(long, value_name = "NAME", conflicts_with_all = ["extension", "scopes"])]
    root: Option<String>,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

#[derive(clap::Args)]
struct RevokeArgs {
    /// The extension, as `specforge grants` lists it
    #[arg(required_unless_present_any = ["root", "whole"])]
    extension: Option<String>,

    /// Its scopes
    scopes: Vec<String>,

    /// A named root of specforge.json, instead of an extension's scopes
    #[arg(long, value_name = "NAME", conflicts_with_all = ["extension", "scopes", "whole"])]
    root: Option<String>,

    /// Every approval of this project
    #[arg(long = "project", id = "whole", conflicts_with_all = ["extension", "scopes"])]
    whole: bool,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

impl Args {
    pub(crate) fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        match self.action {
            None => self.list.run(terminal, ports),
            Some(Action::List(list)) => list.run(terminal, ports),
            Some(Action::Approve(a)) => {
                let which = match (&a.root, &a.extension) {
                    (Some(root), _) => Change::Root(root),
                    (None, Some(extension)) => Change::Scopes {
                        extension,
                        scopes: &a.scopes,
                    },
                    (None, None) => unreachable!("clap requires an extension or --root"),
                };
                change(terminal, ports, &a.project, a.output.format, which, true)
            }
            Some(Action::Revoke(a)) => {
                let which = match (&a.root, &a.extension, a.whole) {
                    (_, _, true) => Change::Project,
                    (Some(root), _, false) => Change::Root(root),
                    (None, Some(extension), false) => Change::Scopes {
                        extension,
                        scopes: &a.scopes,
                    },
                    (None, None, false) => {
                        unreachable!("clap requires an extension, --root or --project")
                    }
                };
                change(terminal, ports, &a.project, a.output.format, which, false)
            }
        }
    }
}

impl ListArgs {
    fn run(self, terminal: &Terminal, ports: &specforge_effects::Ports) -> Exit {
        let format = self.output.format;
        let root = match self.project.root() {
            Ok(root) => root,
            Err(error) => return Refusal::of(format).report(terminal, &error),
        };
        let consent = UserConsent::user(root.dir());
        let store = consent.path().display().to_string();
        if self.store {
            return match grants::list_store(&consent) {
                Ok(listing) => {
                    match format {
                        OutputFormat::Json => terminal.json(&listing),
                        OutputFormat::Human => print_store(terminal, &store, &listing),
                    }
                    Exit::Passed
                }
                Err(error) => Refusal::of(format).report(terminal, &error),
            };
        }
        let project = crate::project::compile_at(&root, ports);
        match grants::list(&ProjectView::of(&project), &consent) {
            Ok(listing) => {
                match format {
                    OutputFormat::Json => terminal.json(&listing),
                    OutputFormat::Human => print_listing(terminal, &store, &listing),
                }
                Exit::Passed
            }
            Err(error) => Refusal::of(format).at(root.dir()).report(terminal, &error),
        }
    }
}

/// `approve` or `revoke` `which` in the project at `project`.
fn change(
    terminal: &Terminal,
    ports: &specforge_effects::Ports,
    project: &crate::project::ProjectArg,
    format: OutputFormat,
    which: Change<'_>,
    approve: bool,
) -> Exit {
    let root = match project.root() {
        Ok(root) => root,
        Err(error) => return Refusal::of(format).report(terminal, &error),
    };
    let consent = UserConsent::user(root.dir());
    let compiled = crate::project::compile_at(&root, ports);
    let view = ProjectView::of(&compiled);
    let done = if approve {
        grants::approve(&view, &consent, which)
    } else {
        grants::revoke(&view, &consent, which)
    };
    let done = match done {
        Ok(done) => done,
        Err(error) => return Refusal::of(format).at(root.dir()).report(terminal, &error),
    };
    match format {
        OutputFormat::Json => terminal.json(&done),
        OutputFormat::Human if done.changed.is_empty() => outln!(
            terminal,
            "Nothing to {}.",
            if approve { "approve" } else { "revoke" }
        ),
        OutputFormat::Human => {
            let verb = if approve { "Approved" } else { "Revoked" };
            for changed in &done.changed {
                outln!(terminal, "{verb} {changed}");
            }
            outln!(terminal, "in {}", consent.path().display());
        }
    }
    Exit::Passed
}

fn print_listing(terminal: &Terminal, store: &str, listing: &GrantsListing) {
    for diagnostic in &listing.diagnostics.0 {
        errln!(
            terminal,
            "warning[{}]: {}",
            diagnostic.code,
            diagnostic.message
        );
    }
    if listing.extensions.is_empty() {
        outln!(terminal, "No loaded extension declares a capability.");
    }
    let mut unapproved = Vec::new();
    for extension in &listing.extensions {
        outln!(
            terminal,
            "{} ({})",
            extension.extension,
            extension.provenance
        );
        for row in &extension.capabilities {
            let state = match (row.state, &row.approved_at) {
                (GrantState::NoneNeeded, _) => "needs no approval".to_string(),
                (GrantState::Approved, Some(at)) if !at.is_empty() => format!("approved {at}"),
                (GrantState::Approved, _) => "approved".to_string(),
                (GrantState::NotApproved, _) => "not approved".to_string(),
                (GrantState::NarrowedOut, _) => "narrowed out by specforge.json".to_string(),
            };
            let used = if row.in_use { ", in use" } else { "" };
            outln!(terminal, "  {}  {}: {state}{used}", row.name, row.scope);
            outln!(terminal, "      {}", row.why);
            if !row.narrowed.is_empty() {
                outln!(terminal, "      narrowed: {}", row.narrowed.join("; "));
            }
            if row.in_use
                && row.state == GrantState::NotApproved
                && !unapproved.contains(&extension.extension)
            {
                unapproved.push(extension.extension.clone());
            }
        }
    }
    if !listing.roots.is_empty() {
        outln!(terminal, "Roots");
        for root in &listing.roots {
            let state = match (root.inside, root.approved) {
                (true, _) => "inside the project",
                (false, true) => "outside the project, approved",
                (false, false) => "outside the project, not approved",
            };
            outln!(terminal, "  {} {}: {state}", root.name, root.target);
        }
    }
    outln!(terminal, "Approvals are kept in {store}");
    for extension in unapproved {
        outln!(
            terminal,
            "Approve what a run here uses with `{}`",
            specforge_common::remedy::command("specforge grants approve {}", &[&extension])
        );
    }
}

fn print_store(terminal: &Terminal, store: &str, listing: &StoreListing) {
    if listing.capabilities.is_empty() && listing.roots.is_empty() {
        outln!(terminal, "{store} holds no approval.");
        return;
    }
    outln!(terminal, "{store}");
    for a in &listing.capabilities {
        outln!(
            terminal,
            "  {} {} {} ({}): {}, approved {}",
            a.project,
            a.extension,
            a.scope,
            a.provenance,
            a.granted,
            a.approved_at
        );
    }
    for a in &listing.roots {
        outln!(
            terminal,
            "  {} root {} {}, approved {}",
            a.project,
            a.root,
            a.target,
            a.approved_at
        );
    }
}
