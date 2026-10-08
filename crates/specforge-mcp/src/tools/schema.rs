use specforge_ops::schema::SchemaRequest;

use crate::args::Arguments;
use crate::tool::ToolOutcome;
use specforge_ops::view::ProjectView;

/// `specforge.schema`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Filter schema to a specific entity kind
    kind: Option<String>,
    /// Include edge type definitions
    #[arg(default = SchemaRequest::default().edges)]
    include_edges: bool,
    /// Include the validation rules loaded extensions declare
    #[arg(default = SchemaRequest::default().validation_rules)]
    include_validation_rules: bool,
}

/// `specforge.schema`: the schema operation over the served project: the
/// GraphProtocolSchema a full export embeds, versioned as `specforge
/// export` versions it. `kind` keeps that kind and the edge types that can
/// start or end at it (a kind no extension declares is `invalid_input` on
/// `kind`, naming the closest); `include_edges: false` drops `edge_types`;
/// `include_validation_rules` adds the rules the loaded extensions declare.
pub fn call(view: ProjectView<'_>, args: Args) -> ToolOutcome {
    let request = SchemaRequest {
        kind: args.kind.as_deref(),
        edges: args.include_edges,
        validation_rules: args.include_validation_rules,
    };
    match specforge_ops::schema::schema(&view, &request) {
        Ok(outcome) => {
            ToolOutcome::ok(serde_json::to_value(&outcome).expect("a schema serializes"))
        }
        Err(error) => crate::tool::McpError::from(error)
            .with_argument("kind")
            .into(),
    }
}
