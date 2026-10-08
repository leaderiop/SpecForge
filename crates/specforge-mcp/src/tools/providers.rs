//! `specforge.providers`: the configured providers (`specforge_ops::extension::providers`).

use crate::args::NoArgs;
use crate::target::ProjectRef;
use crate::tool::ToolOutcome;

pub(crate) fn call(project: &ProjectRef<'_>, _args: NoArgs) -> ToolOutcome {
    // The providers specforge.json configures, as the scheme registry built
    // from the loaded extensions sees them: the listing the CLI prints.
    let listing = specforge_ops::extension::providers(&project.view());
    ToolOutcome::ok(listing.to_json())
}
