//! `set_verdict` (ADR 0289, D6): a judged rule's verdict, checked through the judged-rules port. The host has no
//! judged rule until a host that declares them ([`NoJudgedRules`]): every `set_verdict` is then `not_judged`. A host
//! that has them implements [`JudgedRules`]; the checks, their order and their refusals are here.
//!
//! In order: the rule is judged (`not_judged`); the entity's kind is a target, the outcome declared, the origin a
//! command of the rule's extension, the judge the rule's, every value bound and every cited reference existing
//! (`verdict_refused`, with a `reason`); the digests the verdict was judged over are the digests now
//! (`stale_revision`, with `inputs`). Then the host fills `inputs` and `at` and plans the write.

use super::{NOT_JUDGED, Origin, STALE_REVISION};
use crate::OpError;
use crate::OpErrorKind;
use crate::Refusal;
use crate::authoring::edit::{EditField, Site};
use crate::view::ProjectView;
use serde_json::{Value, json};
use specforge_protocol_types::Cite;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A verdict the host refuses: the data names the operation and the `reason`.
pub const VERDICT_REFUSED: &str = "verdict_refused";

/// A judged rule, as the port describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgedRule {
    /// The extension that declares it: only a command of it may set its verdict.
    pub extension: String,
    /// The judge's name and the extension's version, written `<judge>@<version>`.
    pub judge: String,
    pub version: String,
    /// The kinds it judges.
    pub targets: Vec<String>,
    /// The outcomes it declares.
    pub outcomes: Vec<String>,
    /// The fields of the entity it binds, whose values a verdict may carry.
    pub values: Vec<String>,
}

/// A cite that names nothing in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCite(pub String);

/// The verdict member the host writes: what the judge answered, the digests it was judged over, and when.
#[derive(Debug, Clone, PartialEq)]
pub struct VerdictMember {
    pub rule: String,
    pub outcome: String,
    /// `<judge>@<version>`.
    pub judge: String,
    pub model: String,
    pub findings: Vec<Value>,
    pub cited: Vec<Cite>,
    pub values: BTreeMap<String, Value>,
    /// The digest of each named input, as the host computed them.
    pub inputs: BTreeMap<String, String>,
    /// The host's date, `YYYY-MM-DD`.
    pub at: String,
}

/// The judged rules of a host, as the apply step reads them (ADR 0289).
pub trait JudgedRules {
    /// The judged rule `name`, or none.
    fn rule(&self, name: &str) -> Option<JudgedRule>;
    /// Each named input's digest for `entity`, `cited` included as `cited`; an unknown cite is an error.
    fn digests(
        &self,
        view: &ProjectView<'_>,
        entity: &str,
        rule: &str,
        cited: &[Cite],
    ) -> Result<BTreeMap<String, String>, UnknownCite>;
    /// Whether the field `field` of an entity of kind `kind` takes `value`, which a verdict gives it; the reason when it
    /// does not. A host that checks no form takes every value.
    fn admits(
        &self,
        _view: &ProjectView<'_>,
        _kind: &str,
        _field: &str,
        _value: &Value,
    ) -> Result<(), String> {
        Ok(())
    }
    /// `text` (the entity's file) with the entity's verdict for the rule replaced by `verdict`, or added.
    fn write(
        &self,
        site: &Site<'_>,
        text: &str,
        verdict: &VerdictMember,
    ) -> Result<String, Refusal<EditField>>;
}

/// The host before judged rules: no rule is judged.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoJudgedRules;

impl JudgedRules for NoJudgedRules {
    fn rule(&self, _: &str) -> Option<JudgedRule> {
        None
    }
    fn digests(
        &self,
        _: &ProjectView<'_>,
        _: &str,
        _: &str,
        _: &[Cite],
    ) -> Result<BTreeMap<String, String>, UnknownCite> {
        Ok(BTreeMap::new())
    }
    fn write(
        &self,
        _: &Site<'_>,
        text: &str,
        _: &VerdictMember,
    ) -> Result<String, Refusal<EditField>> {
        Ok(text.to_string())
    }
}

/// The host's clock: one seam for the date a verdict and a command carry.
pub trait Clock {
    /// Today, UTC, `YYYY-MM-DD`.
    fn today(&self) -> String;
}

/// The system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> String {
        crate::extension::command::today()
    }
}

/// The judged rules a plan is applied with, shared by the steps that write a verdict.
#[derive(Clone)]
pub struct Rules(pub Arc<dyn JudgedRules>);

impl std::fmt::Debug for Rules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Rules")
    }
}

impl PartialEq for Rules {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// A `set_verdict` operation as the plan spelled it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerdictOp {
    pub number: usize,
    pub entity: String,
    pub rule: String,
    pub outcome: String,
    pub judge: String,
    pub model: String,
    pub findings: Vec<Value>,
    pub cited: Vec<Cite>,
    pub values: BTreeMap<String, Value>,
    pub basis: BTreeMap<String, String>,
}

/// A verdict the host checked and will write.
#[derive(Debug, Clone, PartialEq)]
pub struct VerdictStep {
    /// The operation's number in the plan.
    pub number: usize,
    pub entity: String,
    pub member: VerdictMember,
    pub rules: Rules,
}

fn refused(op: &VerdictOp, reason: &str, message: String) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        VERDICT_REFUSED,
        format!("operation {}: {message}", op.number),
    )
    .with_data(json!({"op": op.number, "reason": reason}))
}

/// Check `op` as D6 orders it, asked by `origin`, and give the step that writes it.
pub(crate) fn check(
    view: &ProjectView<'_>,
    rules: &Arc<dyn JudgedRules>,
    clock: &dyn Clock,
    origin: &Origin,
    op: &VerdictOp,
) -> Result<VerdictStep, OpError> {
    let Some(rule) = rules.rule(&op.rule) else {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            NOT_JUDGED,
            format!(
                "operation {}: set_verdict names the rule '{}', which no extension judges",
                op.number, op.rule
            ),
        )
        .with_data(json!({"op": op.number})));
    };
    let kind = match view.graph().find(&op.entity) {
        specforge_graph::Found::One(path) => view
            .graph()
            .node(path.as_str())
            .map(|node| node.kind.raw.to_string()),
        _ => None,
    };
    if !kind.as_ref().is_some_and(|k| rule.targets.contains(k)) {
        return Err(refused(
            op,
            "kind",
            format!(
                "'{}' is not an entity the rule '{}' judges",
                op.entity, op.rule
            ),
        ));
    }
    if !rule.outcomes.contains(&op.outcome) {
        return Err(refused(
            op,
            "outcome",
            format!("'{}' is no outcome of the rule '{}'", op.outcome, op.rule),
        ));
    }
    if !matches!(origin, Origin::Command { extension } if *extension == rule.extension) {
        return Err(refused(
            op,
            "origin",
            format!(
                "only a command of {} writes a verdict for the rule '{}'",
                rule.extension, op.rule
            ),
        ));
    }
    if op.judge != rule.judge {
        return Err(refused(
            op,
            "judge",
            format!("'{}' is not the judge of the rule '{}'", op.judge, op.rule),
        ));
    }
    if let Some(value) = op.values.keys().find(|v| !rule.values.contains(v)) {
        return Err(refused(
            op,
            "value",
            format!("the rule '{}' binds no value '{value}'", op.rule),
        ));
    }
    for (field, value) in &op.values {
        if let Err(why) = rules.admits(view, kind.as_deref().unwrap_or_default(), field, value) {
            return Err(refused(
                op,
                "value",
                format!("the value of '{field}' is refused by its field: {why}"),
            ));
        }
    }
    let current = rules
        .digests(view, &op.entity, &op.rule, &op.cited)
        .map_err(|UnknownCite(cite)| {
            refused(
                op,
                "cite",
                format!("it cites '{cite}', which the project does not hold"),
            )
        })?;
    stale(op.number, &op.basis, &current)?;
    Ok(VerdictStep {
        number: op.number,
        entity: op.entity.clone(),
        member: VerdictMember {
            rule: op.rule.clone(),
            outcome: op.outcome.clone(),
            judge: format!("{}@{}", rule.judge, rule.version),
            model: op.model.clone(),
            findings: op.findings.clone(),
            cited: op.cited.clone(),
            values: op.values.clone(),
            inputs: current,
            at: clock.today(),
        },
        rules: Rules(Arc::clone(rules)),
    })
}

/// `stale_revision` when an input's digest is not the one the verdict was judged over.
fn stale(
    number: usize,
    basis: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
) -> Result<(), OpError> {
    let stale: Vec<Value> = current
        .iter()
        .filter(|(name, digest)| basis.get(*name) != Some(*digest))
        .map(|(name, digest)| json!({"name": name, "basis": basis.get(name), "current": digest}))
        .collect();
    if stale.is_empty() {
        return Ok(());
    }
    Err(OpError::new(
        OpErrorKind::Conflict,
        STALE_REVISION,
        format!(
            "operation {number}: the verdict was judged over inputs that have changed; nothing was written"
        ),
    )
    .with_data(json!({"op": number, "inputs": stale})))
}

/// Under the write lock: the digests of each verdict's inputs over the disk as it is now are the ones it was judged
/// over (the compile's), else `stale_revision`. A verdict has no file revision, so this is its only check against a
/// writer that landed since the compile.
pub(crate) fn against_disk(
    project: &specforge_project::CompiledProject,
    verdicts: &[&VerdictStep],
) -> Result<(), OpError> {
    if verdicts.is_empty() {
        return Ok(());
    }
    let disk = project.edited_over_disk(&Default::default(), None);
    let view = ProjectView::of(&disk);
    for step in verdicts {
        let current = step
            .rules
            .0
            .digests(&view, &step.entity, &step.member.rule, &step.member.cited)
            .map_err(|UnknownCite(cite)| {
                OpError::new(
                    OpErrorKind::InvalidInput,
                    VERDICT_REFUSED,
                    format!(
                        "operation {}: it cites '{cite}', which the project no longer holds",
                        step.number
                    ),
                )
                .with_data(json!({"op": step.number, "reason": "cite"}))
            })?;
        stale(step.number, &step.member.inputs, &current)?;
    }
    Ok(())
}
