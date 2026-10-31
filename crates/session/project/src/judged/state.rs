//! The state of each judged rule on each entity (ADR 0290), and what the rule reports for it.

use serde_json::Value;
use specforge_common::{Diagnostic, SourceSpan, codes};
use specforge_graph::Computed;
use specforge_graph::field_form::write;
use specforge_parser::{FieldValue, VerdictMember};
use specforge_protocol_types::{Judgement, JudgementState};
use specforge_registry::judged::JudgedRule;
use specforge_registry::{Disk, EntitySnapshot};

use super::inputs::{Cited, Cx, NONE};
use super::{DigestMemo, Judgements};
use crate::Environment;
use crate::coverage::ReportRead;
use crate::harness::{Harness, LEDGER_FILE, Ledger, VERDICT_KEY, digest};

/// The rule `written` names: its qualified code, or its local code when one rule has it.
pub(super) fn rule_of<'a>(rules: &[&'a JudgedRule], written: &str) -> Option<&'a JudgedRule> {
    rules
        .iter()
        .copied()
        .find(|r| r.code == written)
        .or_else(|| {
            let local = format!("/{written}");
            let mut named = rules.iter().copied().filter(|r| r.code.ends_with(&local));
            named.next().filter(|_| named.next().is_none())
        })
}

/// Why `verdict` cannot be read as a verdict of `rule` on a `kind`, when it cannot.
fn unreadable(rule: &JudgedRule, kind: &str, verdict: &VerdictMember) -> Option<String> {
    if rule.target(kind).is_none() {
        return Some(format!("rule '{}' does not judge a {kind}", rule.code));
    }
    if !rule.outcomes.contains(&verdict.outcome) {
        return Some(format!(
            "'{}' is not an outcome of '{}' ({})",
            verdict.outcome,
            rule.code,
            rule.outcomes.join(", ")
        ));
    }
    for member in ["judge", "model", "at"] {
        if !matches!(verdict.body.get(member), Some(FieldValue::String(s)) if !s.is_empty()) {
            return Some(format!("'{member}' is missing or not a string"));
        }
    }
    let Some(FieldValue::Block(inputs)) = verdict.body.get("inputs") else {
        return Some("'inputs' is missing or not a block".into());
    };
    if inputs
        .entries()
        .iter()
        .any(|e| !matches!(e.value, FieldValue::String(_)))
    {
        return Some("'inputs' holds a digest that is not a string".into());
    }
    if let Some(FieldValue::Block(values)) = verdict.body.get("values") {
        // shortcut: a bound field's name is checked here, its value against the field's form by `set_verdict`.
        let bound: Vec<&str> = rule
            .values
            .iter()
            .filter_map(|v| v.strip_prefix(&format!("{kind}.")))
            .collect();
        if let Some(entry) = values
            .entries()
            .iter()
            .find(|e| !bound.contains(&e.key.as_str()))
        {
            return Some(format!(
                "'values' names '{}', which '{}' does not bind on a {kind}",
                entry.key, rule.code
            ));
        }
    } else if verdict.body.entry("values").is_some() {
        return Some("'values' is not a block".into());
    }
    None
}

fn recorded_inputs(verdict: &VerdictMember) -> Vec<(String, String)> {
    match verdict.body.get("inputs") {
        Some(FieldValue::Block(block)) => block
            .entries()
            .iter()
            .filter_map(|e| match &e.value {
                FieldValue::String(digest) => Some((e.key.to_string(), digest.clone())),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn text(verdict: &VerdictMember, key: &str) -> Option<String> {
    match verdict.body.get(key) {
        Some(FieldValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn finding(rule: &JudgedRule, id: &str, kind: &str, why: &str, span: &SourceSpan) -> Diagnostic {
    let message = rule.template.replace("{id}", id).replace("{kind}", kind);
    Diagnostic::from_extension(
        &rule.extension,
        rule.code.clone(),
        rule.severity,
        format!("{message}: {why}"),
    )
    .with_span(span.clone())
}

/// What an evaluation found.
#[derive(Default)]
pub struct Evaluated {
    pub judgements: Judgements,
    pub diagnostics: Vec<Diagnostic>,
    /// The `V` fields the fresh verdicts give: their outcome or `values` (ADR 0290).
    pub values: Computed,
}

/// The `V` fields of `kind` that the fresh `verdict` of `rule` gives `id`: the outcome for an outcome field, a member
/// of `values` for a bound one, admitted by its field's form.
fn given(
    env: &Environment,
    rule: &JudgedRule,
    node: &specforge_graph::Node,
    verdict: &VerdictMember,
    out: &mut Evaluated,
) {
    let (id, kind) = (node.id.raw.as_str(), node.kind.raw.as_str());
    let own = |list: &[String]| -> Vec<String> {
        list.iter()
            .filter_map(|v| v.strip_prefix(&format!("{kind}.")).map(str::to_string))
            .collect()
    };
    for field in own(&rule.outcome_fields) {
        if let Some((_, form)) = env.registries().fields().get_path(kind, &field)
            && let Ok(admitted) = form.admit_json(&Value::String(verdict.outcome.clone()))
        {
            out.values.insert(id, &field, admitted);
        }
    }
    let Some(FieldValue::Block(values)) = verdict.body.get("values") else {
        return;
    };
    let bound = own(&rule.values);
    for entry in values.entries() {
        let field = entry.key.as_str();
        if !bound.iter().any(|b| b == field) {
            continue;
        }
        let Some((_, form)) = env.registries().fields().get_path(kind, field) else {
            continue;
        };
        match form.admit_json(write(&entry.value).as_value()) {
            Ok(admitted) => out.values.insert(id, field, admitted),
            Err(refusal) => out.diagnostics.push(
                Diagnostic::new(
                    codes::W168,
                    format!(
                        "verdict of {kind} '{id}': 'values' gives '{field}' a value its form refuses{}",
                        refusal
                            .suggestion
                            .map_or_else(String::new, |s| format!(": {s}"))
                    ),
                )
                .with_span(verdict.span.clone()),
            ),
        }
    }
}

/// Every judged rule's state on every entity of `graph`, and the diagnostics: a rule's own for each entity whose
/// verdict does not hold, W168 for a verdict that cannot be read, I205 once when test inputs were not judged, E045
/// when the report is unusable. `ledger` is the bytes of `specforge-harness`. A rule gated by its setting reads nothing.
pub fn evaluate(
    env: &Environment,
    graph: &specforge_graph::Graph,
    entities: &EntitySnapshot,
    disk: &dyn Disk,
    memo: &DigestMemo,
    ledger: &[u8],
) -> Evaluated {
    let registries = env.registries();
    let rules: Vec<&JudgedRule> = registries
        .judged_rules()
        .filter(|rule| !env.settings().gated().contains(&rule.code))
        .collect();
    let mut out = Evaluated::default();
    if rules.is_empty() {
        return out;
    }
    let all: Vec<&JudgedRule> = registries.judged_rules().collect();
    let ledger = Ledger::parse(&String::from_utf8_lossy(ledger)).ledger;
    let harness = Harness::of(registries);
    let cx = Cx::new(env, graph, entities, disk, memo);
    let forms = registries.field_forms();
    let mut unjudged = 0usize;
    for node in graph.nodes() {
        let (id, kind) = (node.id.raw.as_str(), node.kind.raw.as_str());
        let mut read: Vec<(&JudgedRule, &VerdictMember)> = Vec::new();
        let mut broken: Vec<&JudgedRule> = Vec::new();
        for verdict in &node.verdicts {
            let Some(rule) = rule_of(&all, &verdict.rule) else {
                out.diagnostics.push(
                    Diagnostic::new(
                        codes::W168,
                        format!(
                            "verdict of {kind} '{id}': '{}' is not a judged rule",
                            verdict.rule
                        ),
                    )
                    .with_span(verdict.span.clone()),
                );
                continue;
            };
            if env.settings().gated().contains(&rule.code) {
                continue;
            }
            let why = if read.iter().any(|(r, _)| r.code == rule.code)
                || broken.iter().any(|r| r.code == rule.code)
            {
                Some(format!("a second verdict of '{}'", rule.code))
            } else {
                unreadable(rule, kind, verdict)
            };
            match why {
                Some(why) => {
                    out.diagnostics.push(
                        Diagnostic::new(codes::W168, format!("verdict of {kind} '{id}': {why}"))
                            .with_span(verdict.span.clone()),
                    );
                    if !broken.iter().any(|r| r.code == rule.code)
                        && !read.iter().any(|(r, _)| r.code == rule.code)
                    {
                        broken.push(rule);
                    }
                }
                None => read.push((rule, verdict)),
            }
        }
        for rule in &rules {
            let Some(target) = rule.target(kind) else {
                continue;
            };
            if target
                .unless_set
                .iter()
                .any(|f| node.fields.entries().iter().any(|e| e.key.as_str() == f))
            {
                continue;
            }
            let mut judgement = Judgement {
                rule: rule.code.clone(),
                ..Judgement::default()
            };
            let verdict = read
                .iter()
                .find(|(r, _)| r.code == rule.code)
                .map(|(_, v)| *v);
            let (state, why, span) = match verdict {
                None if broken.iter().any(|r| r.code == rule.code) => (
                    JudgementState::Unreadable,
                    "its verdict cannot be read".to_string(),
                    node.source_span.clone(),
                ),
                None => (
                    JudgementState::Missing,
                    "no verdict".to_string(),
                    node.source_span.clone(),
                ),
                Some(verdict) => {
                    judgement.outcome = Some(verdict.outcome.clone());
                    judgement.at = text(verdict, "at");
                    let key = format!("{VERDICT_KEY}{}", verdict.rule);
                    let recorded = harness
                        .value(graph, forms, id, &key)
                        .is_some_and(|value| ledger.records(id, &key, &digest(id, &key, &value)));
                    if !recorded {
                        (
                            JudgementState::Unrecorded,
                            format!("its verdict is not recorded in {LEDGER_FILE}"),
                            verdict.span.clone(),
                        )
                    } else {
                        let now = cx.inputs(node, rule, &Cited::of(verdict));
                        if now.tests_unjudged {
                            unjudged += 1;
                        }
                        judgement.files = now.files.clone();
                        let before = recorded_inputs(verdict);
                        let changed: Vec<String> = now
                            .digests
                            .iter()
                            .filter(|(name, digest)| {
                                let then = before
                                    .iter()
                                    .find(|(n, _)| n == name)
                                    .map(|(_, d)| d.as_str());
                                // `none` now is not judged; a recorded `none` is stale once the digest is real
                                digest != NONE && then != Some(digest.as_str())
                            })
                            .map(|(name, _)| name.clone())
                            .collect();
                        let at = judgement.at.clone().unwrap_or_default();
                        if !changed.is_empty() {
                            let why = format!(
                                "the verdict of {at} is stale: {} changed",
                                changed.join(", ")
                            );
                            judgement.changed = changed;
                            (JudgementState::Stale, why, verdict.span.clone())
                        } else {
                            judgement.passed = rule.passes(&verdict.outcome);
                            // `findings` is a list of blocks (a list of values reads the same).
                            let n = match verdict.body.get("findings") {
                                Some(FieldValue::List(items)) => items.len(),
                                Some(FieldValue::Blocks(items)) => items.len(),
                                _ => 0,
                            };
                            let why = format!(
                                "the verdict of {at} is '{}' ({n} finding{})",
                                verdict.outcome,
                                if n == 1 { "" } else { "s" }
                            );
                            (JudgementState::Fresh, why, verdict.span.clone())
                        }
                    }
                }
            };
            if state == JudgementState::Fresh
                && let Some(verdict) = verdict
            {
                given(env, rule, node, verdict, &mut out);
            }
            judgement.state = state.as_str().to_string();
            if !judgement.passed {
                let finding = finding(rule, id, kind, &why, &span);
                out.judgements.found(finding.clone());
                out.diagnostics.push(finding);
            }
            out.judgements.insert(id, judgement);
        }
    }
    if unjudged > 0 {
        match cx.report() {
            ReportRead::Unusable(why) => out.diagnostics.push(Diagnostic::new(codes::E045, why.clone())),
            _ => out.diagnostics.push(Diagnostic::new(
                codes::I205,
                format!(
                    "test inputs of {unjudged} judged verdicts were not judged: there is no specforge-report.json, run `specforge collect`"
                ),
            )),
        }
    }
    out
}
