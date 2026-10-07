use specforge_ops::export::{self, Format, Schema};

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.export`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Export format
    #[arg(choice = specforge_ops::export::AGENT_FORMAT)]
    format: Format,
    /// Scope to entity subgraph
    scope: Option<String>,
    /// Optional token budget; truncates the export to the most central entities that fit
    max_tokens: Option<usize>,
    /// Embed the Graph Protocol schema in a context, brief or budgeted graph export (a full graph export embeds it already); under max_tokens it counts toward the budget
    with_schema: bool,
    /// Leave the schema out of a graph export (Graph Protocol 1.0)
    no_schema: bool,
}

/// `specforge.export`: the export `specforge export` writes, through the
/// same function and schema policy (ADR 0004 D3-a). `with_schema` and
/// `no_schema` are the CLI's `--with-schema` and `--no-schema`.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    // The tool serves the agent formats; dot is `specforge.render`'s.
    let schema = match (args.no_schema, args.with_schema) {
        (true, _) => Schema::Without,
        (_, true) => Schema::With,
        _ => Schema::Default,
    };
    let request = export::Request {
        format: Some(args.format),
        scope: args.scope.as_deref(),
        max_tokens: args.max_tokens,
        schema,
        ..export::Request::default()
    };

    match export::export(&call.view(), &request) {
        Ok(json_str) => ToolOutcome::text(json_str),
        Err(err) => crate::tool::McpError::from(err).into(),
    }
}
