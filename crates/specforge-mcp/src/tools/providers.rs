//! `specforge.providers`: the configured providers (`specforge_ops::extension::providers`).

use crate::args::NoArgs;
use crate::reply::Answered;
use crate::target::ProjectRef;

/// `specforge.providers`'s reply: the document `specforge providers
/// --format json` prints (`McpProvidersResult`).
pub use specforge_ops::extension::ProvidersDocument as Reply;

pub(crate) fn call(project: &ProjectRef<'_>, _args: NoArgs) -> Answered<Reply> {
    // The providers specforge.json configures, as the scheme registry built
    // from the loaded extensions sees them: the listing the CLI prints.
    let listing = specforge_ops::extension::providers(&project.view());
    Ok(listing.document().into())
}
