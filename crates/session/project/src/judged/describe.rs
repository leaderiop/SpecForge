//! What a judge reads before it answers (ADR 0290): one judged rule's inputs for one entity, with the verdict's current
//! state, from the same evaluation the checks make.

use specforge_graph::Found;
use specforge_protocol_types::Judgement;
use specforge_registry::Disk;
use specforge_registry::judged::JudgedRule;

use super::inputs::{Cited, Cx, Inputs};
use super::state::rule_of;
use crate::CompiledProject;

/// Why the inputs of a rule cannot be given for an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputsRefusal {
    /// No loaded extension declares a judged rule of that name.
    NotJudged,
    /// The rule judges no entity of the entity's kind (this one).
    NotATarget(String),
    /// The project holds no such entity.
    UnknownEntity,
}

/// A rule's inputs for an entity.
#[derive(Debug, Clone)]
pub struct Described<'p> {
    /// The entity's path.
    pub entity: String,
    pub rule: &'p JudgedRule,
    /// The rule's setting is off: nothing is read, nothing is judged (K6).
    pub gated: bool,
    /// The verdict's state at the last checks; none while gated.
    pub judgement: Option<Judgement>,
    /// The inputs now; none while gated.
    pub inputs: Option<Inputs>,
}

/// `rule` (qualified, or local when unique) judging `entity` in `project`, with `cited` as the verdict would cite it.
pub fn describe<'p>(
    project: &'p CompiledProject,
    disk: &dyn Disk,
    entity: &str,
    rule: &str,
    cited: &Cited,
) -> Result<Described<'p>, InputsRefusal> {
    let env = project.environment();
    let all: Vec<&JudgedRule> = env.registries().judged_rules().collect();
    let rule = rule_of(&all, rule).ok_or(InputsRefusal::NotJudged)?;
    let Found::One(path) = project.graph().find(entity) else {
        return Err(InputsRefusal::UnknownEntity);
    };
    let node = project
        .graph()
        .node(path.as_str())
        .ok_or(InputsRefusal::UnknownEntity)?;
    if rule.target(node.kind.raw.as_str()).is_none() {
        return Err(InputsRefusal::NotATarget(node.kind.raw.to_string()));
    }
    let gated = env.settings().gated().contains(&rule.code);
    let inputs = (!gated).then(|| {
        Cx::new(
            env,
            project.graph(),
            project.entities(),
            disk,
            project.digests(),
        )
        .inputs(node, rule, cited)
    });
    let judgement = (!gated)
        .then(|| project.judgements().get(path.as_str(), &rule.code).cloned())
        .flatten();
    Ok(Described {
        entity: path.to_string(),
        rule,
        gated,
        judgement,
        inputs,
    })
}
