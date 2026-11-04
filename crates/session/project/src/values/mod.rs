//! The values phase (ADR 0336): every value whose field has checks, planned by its checkers, its findings placed on
//! the value. It reads the authored snapshot, so a computed or collected value is never checked.

mod collect;
mod place;

use std::collections::{BTreeMap, HashSet};

use specforge_common::Diagnostic;
use specforge_effects::{Effects, OutcomeClass};
use specforge_protocol_types::{
    CheckFinding, CheckItem, CheckPhase, CheckValuesInput, ExtensionDeclaration, PlannedItem,
};
use specforge_registry::{CheckCall, EntitySnapshot, ResolvedCheck};
use specforge_wasm::ExtensionCalls;

use crate::Environment;
use crate::compiled::{CheckedValue, CheckedValues, ValueOutcome};
use collect::Value;

/// The most values one run checks; the rest are not judged and count toward I206 as `budget`.
const VALUE_CAP: usize = 10_000;

/// What became of one check on one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    /// Not performed or not answered: not judged, and nothing goes red for it.
    NotRun,
}

/// One checker's reading of one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueRow {
    /// The written path of the field or member (`links[1].href`).
    pub field: String,
    pub item: usize,
    /// `<extension>:<checker>`.
    pub checker: String,
    pub value: String,
    pub status: CheckStatus,
    /// The class of what its probes answered: `Ok` for a checker with none; `None` when it did not run.
    pub outcome_class: Option<OutcomeClass>,
    /// When its oldest probe answer was observed (Unix milliseconds); `None` for a pure checker.
    pub observed_at: Option<u64>,
    pub from_cache: bool,
    /// The codes it found, as written by the checker.
    pub codes: Vec<String>,
}

/// Every entity's checked values, by entity id, each in collection order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValueChecks(BTreeMap<String, Vec<ValueRow>>);

impl ValueChecks {
    /// `id`'s rows; empty when none of its values has a check.
    pub fn of(&self, id: &str) -> &[ValueRow] {
        self.0.get(id).map_or(&[], Vec::as_slice)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &[ValueRow])> {
        self.0
            .iter()
            .map(|(id, rows)| (id.as_str(), rows.as_slice()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// What a judged verdict digests of them (ADR 0337 §6): a value not run is `unknown` (I207), and the verdict stays
    /// fresh.
    pub(crate) fn checked_values(&self) -> CheckedValues {
        let mut values = CheckedValues::default();
        for (id, rows) in &self.0 {
            for row in rows {
                values.insert(
                    id,
                    CheckedValue {
                        field: row.field.clone(),
                        item: row.item,
                        checker: row.checker.clone(),
                        outcome: row
                            .outcome_class
                            .map_or(ValueOutcome::Unknown, ValueOutcome::Class),
                    },
                );
            }
        }
        values
    }
}

/// What one run of the phase gave.
#[derive(Debug, Default)]
pub(crate) struct Phase {
    pub diagnostics: Vec<Diagnostic>,
    pub checks: ValueChecks,
}

/// One call of a checker on one value.
struct Planned<'a> {
    value: usize,
    /// Which of the value's checks it belongs to.
    check: usize,
    call: &'a CheckCall,
    /// Its id in its extension's call.
    id: u32,
}

/// Run the phase over `entities` in `env`. `text_of` gives a source's text, to place a finding inside a value.
pub(crate) fn run(
    env: &Environment,
    entities: &EntitySnapshot,
    effects: &Effects,
    text_of: &dyn Fn(&str) -> Option<std::sync::Arc<str>>,
) -> Phase {
    let registries = env.registries();
    let checks = registries.checks();
    let mut phase = Phase::default();
    let Some(runtime) = env.runtime().filter(|_| !checks.is_empty()) else {
        return phase;
    };
    let mut values = collect::values(entities, checks, text_of);
    if values.len() > VALUE_CAP {
        let over = values.split_off(VALUE_CAP);
        effects.count_not_run("budget", over.len());
        for value in &over {
            for check in value.checks {
                for call in check.calls() {
                    phase.checks.add_not_run(value, call);
                }
            }
        }
    }
    let planned = planned(&values);
    let calls = ExtensionCalls::new(runtime.as_ref());
    let answers = plans(
        registries.declarations(),
        &values,
        &planned,
        &calls,
        &mut phase.diagnostics,
    );
    let mut misused = HashSet::new();
    let mut by_value: Vec<Vec<&Planned>> = Vec::new();
    by_value.resize_with(values.len(), Vec::new);
    for one in &planned {
        by_value[one.value].push(one);
    }
    for (index, value) in values.iter().enumerate() {
        for (position, check) in value.checks.iter().enumerate() {
            let branches: Vec<&Planned> = by_value[index]
                .iter()
                .filter(|one| one.check == position)
                .copied()
                .collect();
            settle(
                value,
                check,
                &branches,
                &answers,
                registries,
                &mut misused,
                &mut phase,
            );
        }
    }
    phase
}

/// One [`Planned`] per call of each check of each value, ids counted per extension.
fn planned<'a>(values: &'a [Value<'a>]) -> Vec<Planned<'a>> {
    let mut next: BTreeMap<&str, u32> = BTreeMap::new();
    let mut planned = Vec::new();
    for (index, value) in values.iter().enumerate() {
        for (position, check) in value.checks.iter().enumerate() {
            for call in check.calls() {
                let id = next.entry(&call.extension).or_default();
                planned.push(Planned {
                    value: index,
                    check: position,
                    call,
                    id: *id,
                });
                *id += 1;
            }
        }
    }
    planned
}

/// Each extension's answers, by extension and id. A failing extension (E028, once) has none, so its values are not
/// judged; the others still run.
fn plans(
    declarations: &[ExtensionDeclaration],
    values: &[Value<'_>],
    planned: &[Planned<'_>],
    calls: &ExtensionCalls<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<(String, u32), PlannedItem> {
    let mut answers = BTreeMap::new();
    for declaration in declarations {
        let extension = declaration.name();
        let items: Vec<CheckItem> = planned
            .iter()
            .filter(|one| one.call.extension == extension)
            .map(|one| {
                let value = &values[one.value];
                CheckItem {
                    id: one.id,
                    checker: one.call.checker.clone(),
                    entity: value.entity.to_string(),
                    field: value.field.to_string(),
                    item: value.item as u32,
                    text: value.text.to_string(),
                    args: one.call.args.clone(),
                    outcomes: Vec::new(),
                }
            })
            .collect();
        if items.is_empty() {
            continue;
        }
        let input = CheckValuesInput {
            phase: CheckPhase::Plan,
            items,
        };
        match calls.check_values(extension, &input) {
            Ok(plan) => {
                for item in plan.items {
                    answers.insert((extension.to_string(), item.id), item);
                }
            }
            Err(error) => diagnostics.push(error.diagnostic()),
        }
    }
    answers
}

/// One check of one value settled: its findings placed, a row per call. An `any_of` passes when one branch yields no
/// error-level finding; a branch not run could have been that one, so nothing goes red for it.
fn settle(
    value: &Value<'_>,
    check: &ResolvedCheck,
    branches: &[&Planned<'_>],
    answers: &BTreeMap<(String, u32), PlannedItem>,
    registries: &specforge_registry::RegistryBuild,
    misused: &mut HashSet<(String, String)>,
    phase: &mut Phase,
) {
    let answer = |one: &&Planned<'_>| answers.get(&(one.call.extension.clone(), one.id));
    let has_error = |findings: &[CheckFinding]| {
        findings
            .iter()
            .any(|finding| place::is_error(&finding.code))
    };
    let passing = branches
        .iter()
        .find(|one| answer(one).is_some_and(|planned| !has_error(&planned.findings)));
    let some_not_run = branches.iter().any(|one| answer(one).is_none());
    let any_of = matches!(check, ResolvedCheck::AnyOf(_));
    let one_of = any_of.then(|| {
        let names: Vec<&str> = branches
            .iter()
            .map(|one| one.call.checker.as_str())
            .collect();
        format!("one of: {}", names.join(", "))
    });
    for one in branches {
        let Some(planned) = answer(one) else {
            phase.checks.add_not_run(value, one.call);
            continue;
        };
        // What an `any_of` reports: the passing branch's findings, or every branch's when none passes.
        let reported = match passing {
            Some(pass) if any_of => pass.id == one.id,
            _ => passing.is_some() || !some_not_run,
        };
        let status = if !any_of {
            if has_error(&planned.findings) {
                CheckStatus::Failed
            } else {
                CheckStatus::Passed
            }
        } else if passing.is_some() {
            CheckStatus::Passed
        } else if some_not_run {
            CheckStatus::NotRun
        } else {
            CheckStatus::Failed
        };
        let mut kept = Vec::new();
        if reported {
            for finding in &planned.findings {
                phase.diagnostics.extend(place::misuse(
                    registries,
                    one.call,
                    &finding.code,
                    misused,
                ));
                let placed =
                    place::finding(registries, one.call, finding, value, one_of.as_deref());
                phase.diagnostics.push(placed);
                kept.push(finding.code.clone());
            }
        }
        phase.checks.add(
            value,
            one.call,
            status,
            (status != CheckStatus::NotRun).then_some(OutcomeClass::Ok),
            kept,
        );
    }
}

impl ValueChecks {
    fn add(
        &mut self,
        value: &Value<'_>,
        call: &CheckCall,
        status: CheckStatus,
        outcome_class: Option<OutcomeClass>,
        codes: Vec<String>,
    ) {
        self.0
            .entry(value.entity.to_string())
            .or_default()
            .push(ValueRow {
                field: value.field.to_string(),
                item: value.item,
                checker: format!("{}:{}", call.extension, call.checker),
                value: value.text.to_string(),
                status,
                outcome_class,
                observed_at: None,
                from_cache: false,
                codes,
            });
    }

    fn add_not_run(&mut self, value: &Value<'_>, call: &CheckCall) {
        self.add(value, call, CheckStatus::NotRun, None, Vec::new());
    }
}
