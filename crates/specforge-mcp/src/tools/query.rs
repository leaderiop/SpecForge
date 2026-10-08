use serde_json::Value;
use specforge_emitter::{EmitOptions, emit};
use specforge_ops::export::Format;

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.query`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to query
    entity_id: String,
    /// Number of hops
    #[arg(default = 1)]
    depth: usize,
    /// Filter by entity kinds
    kinds: Vec<String>,
    /// Output detail level
    #[arg(choice = specforge_ops::export::AGENT_FORMAT)]
    format: Format,
    /// Include coverage metadata in the response
    include_coverage: bool,
}

pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let view = call.view();
    let entity_id = args.entity_id.as_str();
    let depth = args.depth;
    let include_coverage = args.include_coverage;

    let kinds: Vec<&str> = args.kinds.iter().map(String::as_str).collect();
    let unknown_kinds = super::unknown_kind_diagnostics(&view, &kinds);

    let query_result = {
        let options = EmitOptions {
            format: args.format.emit_format(),
            scope: Some(entity_id),
            depth: Some(depth),
            kind_filter: kinds,
            field_registry: Some(&view.registries().fields),
            ..EmitOptions::default()
        };
        emit(view.graph(), &options)
    };

    match query_result {
        Ok(json_str) => {
            let mut result: Value = serde_json::from_str(&json_str).unwrap_or(Value::Null);

            if include_coverage
                && let Some(nodes) = result.get_mut("nodes").and_then(|n| n.as_array_mut())
            {
                // The failure `specforge.coverage` reports.
                let coverage = match view.coverage() {
                    Ok(coverage) => coverage,
                    Err(error) => {
                        return crate::tool::McpError::from(error).into();
                    }
                };
                for node in nodes.iter_mut() {
                    let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let Some(verdict) = coverage.verdict(node_id) else {
                        continue;
                    };
                    let status = specforge_ops::coverage::STATUS.name_of(verdict.status());
                    node.as_object_mut()
                        .unwrap()
                        .insert("coverage_status".into(), Value::from(status));
                }
            }

            ToolOutcome::ok(result).with_diagnostics(unknown_kinds)
        }
        Err(err) => super::emitter_error(err, view.graph(), entity_id),
    }
}
