//! The verdict command of a judged table (ADR 0377 D5): writes a judge's verdict on one of the table's rules, and
//! nothing else. A prompt cannot write, and an agent calls a command: this is the command. The judge is the rule's
//! own, never an argument, so a caller cannot name another one; the host checks it again, and checks that the command
//! is of the rule's extension. That is not an identity check: any agent may call the command, and `model` is free
//! text. The verdict is as trustworthy as the agent that was told to judge (parity with upstream, where any agent can
//! write VERIFICATION.md).

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::{Cite, Verdict};

use super::{Judged, get};
use crate::spelling::find;

/// Every refusal of the command: `invalid_input`, exit 1.
const INVALID: &str = "invalid_input";

/// One extension's verdict command over its judged table.
pub struct Recorder {
    /// The command's id: `record_verdict`.
    pub command: &'static str,
    /// The namespace of the table's codes: `gsd`, so a rule is `gsd/E501`.
    pub namespace: &'static str,
    /// The extension, as a refusal names it: `@gsd/core`.
    pub extension: &'static str,
    pub table: &'static [Judged],
    /// Per-rule checks of a verdict, each run on every call once its arguments parse.
    pub checks: &'static [Check],
}

/// A verdict the command was asked to record, as its checks see it.
pub struct Proposed<'a> {
    /// The entity's path: the one entity the caller's spelling names, read as the host reads it
    /// ([`crate::spelling::find`]). The verdict is recorded on this path, so the checks judge the entity it lands on.
    pub entity: &'a str,
    /// The entity's kind, one the rule targets.
    pub kind: &'a str,
    pub rule: &'static Judged,
    pub outcome: &'a str,
    pub values: &'a BTreeMap<String, Value>,
    pub cited: &'a [Cite],
    /// The command's graph, for a check that reads what a citation names.
    pub graph: &'a CommandGraph,
}

/// A check of a [`Proposed`] verdict: its `Err` is the refusal.
pub type Check = fn(&Proposed<'_>) -> Result<(), String>;

/// The optional JSON arg `name` as `T`, `default` when it is not given.
fn json<T>(
    call: &CommandCall<'_>,
    name: &str,
    default: T,
    parse: impl FnOnce(&str) -> serde_json::Result<T>,
) -> Result<T, String> {
    match call.str(name) {
        None => Ok(default),
        Some(text) => parse(text).map_err(|e| format!("{name}: not the JSON it takes: {e}")),
    }
}

/// The entity `spelling` names, of a kind `rule` judges: refused when it names none, several, or one of another kind.
/// Failing closed here is what keeps a per-rule check from passing a spelling it could not read.
fn judged_entity<'a>(
    call: &CommandCall<'a>,
    namespace: &str,
    rule: &Judged,
    spelling: &str,
) -> Result<&'a GraphNode, String> {
    let node = find(call.graph(), spelling)
        .one(spelling)
        .map_err(|why| format!("entity: {why}"))?;
    if rule.targets.iter().any(|t| t.kind == node.kind) {
        return Ok(node);
    }
    let kinds: Vec<&str> = rule.targets.iter().map(|t| t.kind).collect();
    Err(format!(
        "entity: '{}' is a {}, which {namespace}/{} does not judge (it judges: {})",
        node.id,
        node.kind,
        rule.code,
        kinds.join(", ")
    ))
}

fn record(call: &CommandCall<'_>, recorder: &Recorder) -> Result<CommandOutput, String> {
    let (ns, extension) = (recorder.namespace, recorder.extension);
    let arg = |name: &str| {
        call.str(name)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| format!("{name} is required"))
    };
    let (entity, code, outcome, model) =
        (arg("entity")?, arg("rule")?, arg("outcome")?, arg("model")?);
    let Some(rule) = get(recorder.table, code) else {
        return Err(format!("rule: '{code}' is no judged rule of {extension}"));
    };
    if !rule.outcomes.contains(&outcome) {
        return Err(format!(
            "outcome: '{outcome}' is no outcome of {ns}/{code} ({})",
            rule.outcomes.join(", ")
        ));
    }
    let basis: BTreeMap<String, String> = serde_json::from_str(arg("basis")?)
        .map_err(|e| format!("basis is not the JSON object `specforge query <entity> --inputs {ns}/{code}` printed: {e}"))?;
    let findings: Vec<Value> = json(call, "findings", Vec::new(), |t| serde_json::from_str(t))?;
    let cited: Vec<Cite> = json(call, "cited", Vec::new(), |t| serde_json::from_str(t))?;
    let values: BTreeMap<String, Value> =
        json(call, "values", BTreeMap::new(), |t| serde_json::from_str(t))?;
    let node = judged_entity(call, ns, rule, entity)?;
    let proposed = Proposed {
        entity: &node.id,
        kind: &node.kind,
        rule,
        outcome,
        values: &values,
        cited: &cited,
        graph: call.graph(),
    };
    for check in recorder.checks {
        check(&proposed)?;
    }
    let mut plan = call.plan();
    plan.set_verdict(Verdict {
        entity: node.id.clone(),
        rule: format!("{ns}/{code}"),
        outcome: outcome.to_string(),
        judge: rule.judge.to_string(),
        model: model.to_string(),
        findings,
        cited,
        values,
        basis,
    });
    Ok(
        CommandOutput::ok(format!("verdict {ns}/{code} {outcome} on {}\n", node.id))
            .with_plan(plan.finish()),
    )
}

/// The `values` arg's description: the first rule of the table that binds a field from them names an example.
pub(super) fn values_description(recorder: &Recorder) -> String {
    let example = recorder.table.iter().find_map(|j| {
        let field = j.values.first()?;
        let name = field.rsplit('.').next().unwrap_or(field);
        Some(format!(
            ", such as the {name} of {}/{}",
            recorder.namespace, j.code
        ))
    });
    format!(
        "JSON object of the field values the rule binds (default {{}}){}",
        example.unwrap_or_default()
    )
}

/// Declares `recorder`'s command.
pub fn declare_record(c: &mut ContributionsBuilder, recorder: &'static Recorder) {
    let ns = recorder.namespace;
    let codes: Vec<&str> = recorder.table.iter().map(|j| j.code).collect();
    c.command(recorder.command, |cmd| {
        cmd.description(&format!("Record a judge's verdict on a judged {ns}/ rule: read the inputs with `specforge query <entity> --inputs {ns}/<code>`, judge, then record the outcome with that `basis`"))
            .writes()
            .arg("entity", |a| {
                a.required().description("the entity judged (its path, or a short form that is not ambiguous)");
            })
            .arg("rule", |a| {
                a.required().one_of(&codes).description("the judged rule, as its local code");
            })
            .arg("outcome", |a| {
                a.required().description("the verdict's outcome, one of the rule's");
            })
            .arg("model", |a| {
                a.required().description("the model that judged");
            })
            .arg("basis", |a| {
                a.required().description(&format!("the JSON object `basis` of `specforge query <entity> --inputs {ns}/<code> --format json`"));
            })
            .arg("findings", |a| {
                a.description("JSON list of findings (default []): objects with severity and message");
            })
            .arg("cited", |a| {
                a.description("JSON list of what the verdict cites (default []): {\"entity\": ..}, {\"file\": ..} or {\"test\": ..}");
            })
            .arg("values", |a| {
                a.description(&values_description(recorder));
            })
            .handler(move |call| match record(call, recorder) {
                Ok(out) => out,
                Err(why) => call.fail(&CommandError::new(INVALID, why), 1),
            });
    });
}
