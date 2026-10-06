//! `specforge.inspect`: one entity's facts (`specforge_ops::inspect`), as
//! JSON. The tool renders the read view; it reads no edge, coverage row or
//! attribution itself (ADR 0015, section "Inspect").

use serde_json::{Value, json};
use specforge_ops::inspect::{EntityCoverage, EntityFacts, obligation_text};

use crate::target::Call;
use crate::tool::ToolOutcome;

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    entity_id: String,
}

pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let view = call.view();
    let Ok(facts) = specforge_ops::inspect::inspect(&view, &args.entity_id) else {
        return crate::tool::entity_not_found(&args.entity_id).into();
    };
    // A recorded report that cannot be read fails the call (ADR 0004 D2-e).
    let coverage = match &facts.coverage {
        Ok(coverage) => coverage,
        Err(error) => return super::coverage::report_error_result(error),
    };
    ToolOutcome::ok(result_json(&facts, coverage))
}

/// `McpInspectResult`: the one presenter of an entity's facts as JSON.
fn result_json(facts: &EntityFacts, coverage: &EntityCoverage) -> Value {
    let node = facts.node;
    let refs = &facts.references;
    // Deprecated aliases (ADR 0016): both directions, unlabeled, in edge
    // order, one per reference.
    let references: Vec<&str> = refs
        .incoming
        .iter()
        .chain(&refs.outgoing)
        .map(|r| r.peer.as_str())
        .collect();
    let declared = coverage.declared();
    json!({
        "entity_id": node.id.raw,
        "kind": node.kind.raw,
        "title": node.title,
        // Its kind's testability, the standing the hover shows (ADR 0004
        // D2-d); whether it declares obligations; its coverage status.
        "testable": facts.standing.testable,
        "declared": declared,
        "reference_count": references.len(),
        "source_span": super::span_json(&node.source_span),
        // The statement the extension declares (headline and normative):
        // a behavior's `contract`; `null` for a kind that declares none.
        "contract": facts.headline,
        // Every field, whatever the kind names its text: an invariant's
        // `guarantee`, a decision's `rationale`, a feature's `description`.
        "fields": specforge_emitter::field_map_to_json(&node.fields),
        "verify_declarations": declared
            .then(|| facts.obligations.iter().map(obligation_text).collect::<Vec<_>>()),
        "referenced_by": refs.referenced_by(),
        "refers_to": refs.refers_to(),
        "references": references,
        "coverage_status": specforge_ops::coverage::STATUS.name_of(coverage.status()),
        // The diagnostics about the entity: those its data names it in,
        // else those inside its block (ADR 0016); never by its message.
        "diagnostics": facts.diagnostics.iter().map(|d| json!({
            "code": d.code,
            "severity": format!("{:?}", d.severity),
            "message": d.message,
            "suggestion": d.suggestion
        })).collect::<Vec<_>>(),
    })
}
