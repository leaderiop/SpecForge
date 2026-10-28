//! What a judge reads before it answers (ADR 0290): a judged rule's inputs for one entity, each with what it picked and
//! its digest, the verdict's current state, and the `basis` a `set_verdict` is checked against. `specforge query
//! <entity> --inputs <rule>` and `specforge.verdict_inputs` render it.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_graph::Found;
use specforge_project::judged::Cited;
use specforge_project::judged::{InputsRefusal, describe};
use specforge_protocol_types::Cite;
use specforge_registry::{Disk, PlainDisk};

use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Refusal};

/// A rule named by `set_verdict` or an inputs query is not judged.
pub const NOT_JUDGED: &str = "not_judged";
/// The rule judges no entity of the entity's kind.
pub const NOT_A_TARGET: &str = "not_a_target";
/// A cite names nothing in the project.
pub const UNKNOWN_CITE: &str = "unknown_cite";

/// The field of an inputs request a refusal can be about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputsField {
    Entity,
    Rule,
    Cited,
}

/// One judged rule on one entity.
#[derive(Debug, Clone, Copy)]
pub struct InputsRequest<'a> {
    pub entity: &'a str,
    /// The rule's qualified code, or its local code when unique.
    pub rule: &'a str,
    /// What the verdict will cite.
    pub cited: &'a [Cite],
}

/// Where the verdict stands (`McpVerdictState`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct VerdictState {
    /// `fresh`, `stale`, `missing`, `unrecorded` or `unreadable`.
    pub state: String,
    /// Fresh, recorded and its outcome passing.
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// The inputs that changed since the verdict (stale only).
    pub changed: Vec<String>,
}

/// One named input (`McpVerdictInput`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct InputRow {
    pub name: String,
    pub selector: String,
    /// `entities`, `files` or `tests`.
    pub kind: String,
    /// The entities (by path) or files (from the project root) it picked.
    pub members: Vec<String>,
    pub digest: String,
    /// It differs from what the current verdict was judged over.
    pub changed: bool,
}

/// What the verdict cites (`McpVerdictCited`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct CitedRow {
    pub entities: Vec<String>,
    pub files: Vec<String>,
    pub tests: Vec<String>,
    pub digest: String,
}

/// A judged rule's inputs for one entity (`McpVerdictInputsResult`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct JudgedInputs {
    /// The entity's path.
    pub entity: String,
    /// The rule's qualified code.
    pub rule: String,
    pub judge: String,
    pub outcomes: Vec<String>,
    pub passing: Vec<String>,
    /// The rule's setting is off: nothing is read and nothing is judged.
    pub gated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<VerdictState>,
    pub inputs: Vec<InputRow>,
    pub cited: CitedRow,
    /// Every input's name and `cited` with its digest: what `set_verdict` names as its `basis`.
    pub basis: BTreeMap<String, String>,
}

/// What `cites` cite, each reference checked against the project: an entity must be one, a file must exist; the error is the
/// reference that is not.
pub(crate) fn cited_of(view: &ProjectView<'_>, cites: &[Cite]) -> Result<Cited, String> {
    let mut cited = Cited::default();
    for cite in cites {
        match cite {
            Cite::Entity { entity } => {
                if !matches!(view.graph().find(entity), Found::One(_)) {
                    return Err(entity.clone());
                }
                cited.entities.push(entity.clone());
            }
            Cite::File { file } => {
                // inside the project: a relative path that never climbs out
                let held = Path::new(file)
                    .components()
                    .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
                    && view
                        .root()
                        .is_some_and(|root| PlainDisk.exists(&root.join(file)));
                if !held {
                    return Err(file.clone());
                }
                cited.files.push(file.clone());
            }
            Cite::Test { test } => cited.tests.push(test.clone()),
        }
    }
    Ok(cited)
}

/// The inputs `request` asks for. An entity the project lacks is E003 with the closest id; a rule no extension judges is
/// `not_judged`, a kind it does not judge `not_a_target`, a cite the project lacks `unknown_cite`.
pub fn judged_inputs(
    view: &ProjectView<'_>,
    request: &InputsRequest<'_>,
) -> Result<JudgedInputs, Refusal<InputsField>> {
    view.entity(request.entity, InputsField::Entity)?;
    let cited = cited_of(view, request.cited).map_err(|unknown| {
        Refusal::about(
            InputsField::Cited,
            OpError::new(
                OpErrorKind::InvalidInput,
                UNKNOWN_CITE,
                format!("it cites '{}', which the project does not hold", unknown),
            ),
        )
    })?;
    let described = describe(
        view.project(),
        &PlainDisk,
        request.entity,
        request.rule,
        &cited,
    )
    .map_err(|refusal| match refusal {
        InputsRefusal::NotJudged => Refusal::about(
            InputsField::Rule,
            OpError::new(
                OpErrorKind::InvalidInput,
                NOT_JUDGED,
                format!("no extension judges the rule '{}'", request.rule),
            ),
        ),
        InputsRefusal::NotATarget(kind) => Refusal::about(
            InputsField::Rule,
            OpError::new(
                OpErrorKind::InvalidInput,
                NOT_A_TARGET,
                format!("the rule '{}' does not judge a {kind}", request.rule),
            ),
        ),
        InputsRefusal::UnknownEntity => {
            crate::view::not_found(view.graph(), request.entity).about(InputsField::Entity)
        }
    })?;
    let verdict = described.judgement.as_ref().map(|j| VerdictState {
        state: j.state.clone(),
        passed: j.passed,
        outcome: j.outcome.clone(),
        at: j.at.clone(),
        changed: j.changed.clone(),
    });
    let moved: Vec<String> = verdict
        .as_ref()
        .map(|v| v.changed.clone())
        .unwrap_or_default();
    let changed = |name: &str| moved.iter().any(|c| c == name);
    let inputs = described.inputs.unwrap_or_default();
    Ok(JudgedInputs {
        entity: described.entity,
        rule: described.rule.code.clone(),
        judge: described.rule.judge.clone(),
        outcomes: described.rule.outcomes.clone(),
        passing: described.rule.passing.clone(),
        gated: described.gated,
        verdict,
        inputs: inputs
            .reports
            .iter()
            .map(|input| InputRow {
                name: input.name.clone(),
                selector: input.selector.clone(),
                kind: input.kind.to_string(),
                members: input.members.clone(),
                digest: input.digest.clone(),
                changed: changed(&input.name),
            })
            .collect(),
        cited: CitedRow {
            entities: inputs.cited.entities,
            files: inputs.cited.files,
            tests: inputs.cited.tests,
            digest: inputs.cited.digest,
        },
        basis: inputs.digests.into_iter().collect(),
    })
}
