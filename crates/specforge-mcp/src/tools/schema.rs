use serde::Deserialize;
use specforge_ops::schema::SchemaRequest;

use crate::args::lenient;
use crate::target::Call;
use crate::tool::ToolOutcome;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    kind: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    include_edges: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    include_validation_rules: Option<bool>,
}

/// `specforge.schema`: the schema operation over the served project: the
/// GraphProtocolSchema a full export embeds, versioned as `specforge
/// export` versions it. `kind` keeps that kind and the edge types that can
/// start or end at it (a kind no extension declares is `invalid_input` on
/// `kind`, naming the closest); `include_edges: false` drops `edge_types`;
/// `include_validation_rules` adds the rules the loaded extensions declare.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    // An absent boolean is the request's default, as on the CLI.
    let default = SchemaRequest::default();
    let request = SchemaRequest {
        kind: args.kind.as_deref(),
        edges: args.include_edges.unwrap_or(default.edges),
        validation_rules: args
            .include_validation_rules
            .unwrap_or(default.validation_rules),
    };
    match specforge_ops::schema::schema(&call.view(), &request) {
        Ok(outcome) => {
            ToolOutcome::ok(serde_json::to_value(&outcome).expect("a schema serializes"))
        }
        Err(error) => crate::operations::op_error(error)
            .with_argument("kind")
            .into(),
    }
}
