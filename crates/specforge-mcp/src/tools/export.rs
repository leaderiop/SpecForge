use serde::Deserialize;
use specforge_ops::export::{self, Schema};

use crate::args::{choice, lenient};
use crate::target::Call;
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
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    // The tool serves the agent formats; dot is `specforge.render`'s.
    let format = match choice(&export::AGENT_FORMAT, "format", args.format.as_deref()) {
        Ok(format) => format,
        Err(refused) => return refused,
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

    match export::export(&call.view(), &request) {
        Ok(json_str) => ToolOutcome::text(json_str),
        Err(err) => crate::tool::McpError::from(err).into(),
    }
}
