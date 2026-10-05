use serde_json::Value;

use specforge_ops::navigate::Direction;

use crate::target::Call;
use crate::tool::{ErrorCode, McpError, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    entity_id: String,
}

pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let view = call.view();
    let entity_id = args.entity_id.as_str();

    let node = match view.graph.node(entity_id) {
        Some(n) => n,
        None => {
            return McpError::new(
                ErrorCode::EntityNotFound,
                format!("Entity not found: {entity_id}"),
            )
            .with_entity(entity_id)
            .into();
        }
    };

    // References split by direction (ADR 0016): the entities that
    // reference this one, and those it refers to. `references` (both,
    // unlabeled) and `reference_count` stay as deprecated aliases.
    let nav = super::navigator(call);
    let ids = |direction| -> Vec<String> {
        let query = specforge_ops::navigate::ReferenceQuery {
            direction,
            include_declaration: false,
        };
        let occurrences = nav.references(entity_id, query).unwrap_or_default();
        let ids: std::collections::BTreeSet<String> = occurrences
            .iter()
            .map(|o| match direction {
                Direction::Outgoing => o.target.to_string(),
                _ => o.holder.to_string(),
            })
            .collect();
        ids.into_iter().collect()
    };
    let referenced_by = ids(Direction::Incoming);
    let refers_to = ids(Direction::Outgoing);
    let reference_count =
        view.graph.edges_to(entity_id).len() + view.graph.edges_from(entity_id).len();
    let references: Vec<String> = view
        .graph
        .edges_to(entity_id)
        .iter()
        .map(|e| e.source.to_string())
        .chain(
            view.graph
                .edges_from(entity_id)
                .iter()
                .map(|e| e.target.to_string()),
        )
        .collect();

    // The statement the extension declares (headline and normative): a
    // behavior's `contract`; `null` for a kind that declares none.
    let contract = specforge_emitter::context::headline_statement(node, &view.registries.fields);

    // The entity's row of the coverage view: whether its kind counts toward
    // coverage, as hover, the schema and the outline say (ADR 0004, D2-d);
    // whether the entity itself declares obligations; and the status
    // `specforge.coverage` reports for it.
    let row = match specforge_ops::coverage::row(&view, entity_id) {
        Ok(row) => row,
        Err(error) => return super::coverage::report_error_result(&error, "specforge.inspect"),
    };
    let obligations = specforge_graph::obligations(node);
    let declared = row
        .as_ref()
        .map_or(!obligations.is_empty(), |row| row.declared());
    let testable = row.as_ref().is_some_and(|row| row.testable);
    let coverage_status = specforge_ops::coverage::status_name(
        row.as_ref()
            .map_or(specforge_project::coverage::Status::Uncovered, |row| {
                row.status()
            }),
    );
    let verify_declarations: Option<Vec<String>> = declared.then(|| {
        obligations
            .iter()
            .map(|s| format!("{} {}", s.kind, s.description))
            .collect()
    });

    // The diagnostics about the entity: those its data names it in, else
    // those inside its block (ADR 0016); never by reading the message.
    let entity_diagnostics: Vec<Value> = super::reported(call)
        .iter()
        .filter(|d| specforge_ops::navigate::is_about(view.graph, d, entity_id))
        .map(|d| {
            serde_json::json!({
                "code": d.code,
                "severity": format!("{:?}", d.severity),
                "message": d.message,
                "suggestion": d.suggestion
            })
        })
        .collect();

    let result = serde_json::json!({
        "entity_id": node.id.raw,
        "kind": node.kind.raw,
        "title": node.title,
        "testable": testable,
        "declared": declared,
        "reference_count": reference_count,
        "source_span": {
            "file": node.source_span.file,
            "start_line": node.source_span.start_line,
            "start_col": node.source_span.start_col,
            "end_line": node.source_span.end_line,
            "end_col": node.source_span.end_col,
        },
        "contract": contract,
        // Every field, whatever the kind names its text: an invariant's
        // `guarantee`, a decision's `rationale`, a feature's `description`.
        "fields": specforge_emitter::field_map_to_json(&node.fields),
        "verify_declarations": verify_declarations,
        "referenced_by": referenced_by,
        "refers_to": refers_to,
        "references": references,
        "coverage_status": coverage_status,
        "diagnostics": entity_diagnostics
    });

    ToolOutcome::ok(result)
}
