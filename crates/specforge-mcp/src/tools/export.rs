use serde::Deserialize;
use specforge_ops::export::{self, Format, Schema};

use crate::args::lenient;
use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    format: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    scope: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    max_tokens: Option<u64>,
    #[serde(default, deserialize_with = "lenient")]
    with_schema: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    no_schema: Option<bool>,
}

/// `specforge.export`: the export `specforge export` writes, through the
/// same function and schema policy (ADR 0004 D3-a). `with_schema` and
/// `no_schema` are the CLI's `--with-schema` and `--no-schema`.
pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let format = args.format.as_deref().unwrap_or("graph");
    // The tool serves the agent formats; dot is `specforge.render`'s.
    let format = match format.parse::<Format>() {
        Ok(Format::Dot) | Err(_) => {
            return ToolOutcome::invalid_params(format!("Unknown format: {}", format));
        }
        Ok(format) => format,
    };
    let schema = match (args.no_schema == Some(true), args.with_schema == Some(true)) {
        (true, _) => Schema::Without,
        (_, true) => Schema::With,
        _ => Schema::Default,
    };
    let request = export::Request {
        format: Some(format),
        scope: args.scope.as_deref(),
        max_tokens: args.max_tokens.map(|v| v as usize),
        schema,
        ..export::Request::default()
    };

    match crate::operations::export_graph(state, &request) {
        Ok(json_str) => ToolOutcome::text(json_str),
        Err(err) => ToolOutcome::invalid_params(err.message),
    }
}
