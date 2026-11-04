//! The values phase (ADR 0336): every value whose field has checks, planned by its checkers, its findings placed on
//! the value. It reads the authored snapshot, so a computed or collected value is never checked.

mod budget;
mod cache;
mod collect;
mod dispatch;
mod local;
mod network;
mod place;

use std::collections::{BTreeMap, HashSet};

use specforge_common::{Diagnostic, Severity};
use specforge_effects::{Effects, OutcomeClass};
pub use specforge_protocol_types::CheckStatus;
use specforge_protocol_types::{
    CheckFinding, CheckItem, CheckOutcome, CheckPhase, CheckValuesInput, PlannedItem,
};
use specforge_registry::{CheckCall, Disk, EntitySnapshot, QualifiedChecker, ResolvedCheck};
use specforge_wasm::{ExtensionCalls, Limits};

use crate::Environment;
use crate::compiled::{CheckedValue, CheckedValues, ValueOutcome};
pub use budget::ValueBudget;
pub(crate) use cache::CheckCache;
use collect::Value;

/// The most values one run checks; the rest are not judged and count toward I206 as [`budget::OVER`].
const VALUE_CAP: usize = 10_000;

/// One checker's reading of one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueRow {
    /// The written path of the field or member (`links[1].href`).
    pub field: String,
    pub item: usize,
    pub checker: QualifiedChecker,
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

    /// Every row as a collector that declares `check_outcomes` receives it, by entity id then collection order.
    pub fn outcomes(&self) -> Vec<CheckOutcome> {
        self.iter()
            .flat_map(|(entity, rows)| {
                rows.iter().map(move |row| CheckOutcome {
                    entity: entity.to_string(),
                    field: row.field.clone(),
                    item: row.item as u32,
                    value: row.value.clone(),
                    checker: row.checker.to_string(),
                    status: row.status,
                    outcome: row.outcome_class,
                    observed_at: row.observed_at,
                    from_cache: row.from_cache,
                    codes: row.codes.clone(),
                })
            })
            .collect()
    }

    /// What a judged verdict digests of them (ADR 0337 §6): a value not run is `unknown` (I207), and the verdict stays
    /// fresh.
    pub fn outcome_classes(&self) -> CheckedValues {
        let mut values = CheckedValues::default();
        for (id, rows) in &self.0 {
            for row in rows {
                values.insert(
                    id,
                    CheckedValue {
                        field: row.field.clone(),
                        item: row.item,
                        checker: row.checker.to_string(),
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

impl Planned<'_> {
    fn key(&self) -> CallKey {
        CallKey::of(&self.call.extension, self.id)
    }
}

/// A call of a checker: its extension, and its id in that extension's call.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CallKey {
    extension: String,
    id: u32,
}

impl CallKey {
    fn of(extension: &str, id: u32) -> Self {
        CallKey {
            extension: extension.to_string(),
            id,
        }
    }
}

/// How the phase asks a checker: through the cache, then the extension.
struct Asking<'a> {
    env: &'a Environment,
    effects: &'a Effects,
    calls: ExtensionCalls<'a>,
    cache: &'a CheckCache,
    dir: Option<std::path::PathBuf>,
}

impl Asking<'_> {
    /// `extension`'s answers to `items` in `phase`: from the cache, the rest from the extension. No item leaves no
    /// call. A call that fails is the extension's whole answer (E028).
    fn ask(
        &self,
        extension: &str,
        phase: CheckPhase,
        items: Vec<CheckItem>,
    ) -> Result<Vec<PlannedItem>, specforge_wasm::CallError> {
        let shelf = cache::Shelf::of(self.env, self.cache, self.dir.as_deref(), extension);
        let (mut known, unknown) = shelf.split(phase, items);
        for (keys, items) in self.batches(extension, unknown) {
            let input = CheckValuesInput { phase, items };
            let answer = self.calls.check_values(extension, &input)?;
            for item in answer.items {
                if let Some(key) = keys.get(&item.id) {
                    shelf.keep(key.clone(), &item);
                    known.push(item);
                }
            }
        }
        shelf.save();
        Ok(known)
    }
}

impl Asking<'_> {
    /// `items` in calls `extension`'s guest can hold ([`call_cap`]), each item with its key. An item no call can hold
    /// is not asked: it is not judged, and counts toward I206.
    fn batches(
        &self,
        extension: &str,
        items: Vec<(String, CheckItem)>,
    ) -> Vec<(BTreeMap<u32, String>, Vec<CheckItem>)> {
        let cap = self.call_cap(extension);
        let mut batches: Vec<(BTreeMap<u32, String>, Vec<CheckItem>)> = Vec::new();
        let mut held = 0;
        for (key, item) in items {
            let size = serde_json::to_vec(&item).map_or(usize::MAX, |bytes| bytes.len());
            if size > cap {
                self.effects.count_not_run(budget::OVER, 1);
                continue;
            }
            if batches.is_empty() || held + size > cap {
                batches.push(Default::default());
                held = 0;
            }
            held += size;
            let (keys, items) = batches.last_mut().expect("just pushed");
            keys.insert(item.id, key);
            items.push(item);
        }
        batches
    }

    /// The most input one call of `extension` carries: a quarter of the memory its sandbox allows, since the guest
    /// holds the JSON, what it decodes it into and its answer at once.
    fn call_cap(&self, extension: &str) -> usize {
        let ceiling = Limits::CEILING.memory_mb;
        let declared = self
            .env
            .registries()
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == extension)
            .and_then(|declaration| declaration.handshake.sandbox_policy.as_ref()?.max_memory_mb);
        ((declared.unwrap_or(ceiling).min(ceiling) as usize) << 20) / 4
    }
}

/// What one call of a checker came to.
#[derive(Debug, Default)]
struct Settled {
    findings: Vec<CheckFinding>,
    /// What the host itself reports on the value in its place (E090, E094, E095, W174, W175).
    host: Vec<Diagnostic>,
    /// The checker has the last word: its plan needed no probe or already failed the value, or it judged the answers.
    judged: bool,
    class: Option<OutcomeClass>,
    observed: Observed,
}

/// Where the answers of a row come from: when the oldest was observed, and whether one came from the journal.
#[derive(Debug, Clone, Copy, Default)]
struct Observed {
    at: Option<u64>,
    from_cache: bool,
}

impl Settled {
    fn fails(&self) -> bool {
        place::any_error(&self.findings) || self.host.iter().any(|d| d.severity == Severity::Error)
    }
}

/// Run the phase over `entities` in `env`: plan, perform the probes through `effects` and `disk`, judge what was
/// performed, place the findings. `text_of` gives a source's text, to place a finding inside a value.
pub(crate) fn run(
    env: &Environment,
    entities: &EntitySnapshot,
    effects: &Effects,
    disk: &dyn Disk,
    cache: &CheckCache,
    text_of: &dyn Fn(&str) -> Option<std::sync::Arc<str>>,
) -> Phase {
    let registries = env.registries();
    let checks = registries.checks();
    let mut phase = Phase::default();
    let wanted = env.value_budget() != ValueBudget::NO_PHASE && !checks.is_empty();
    let Some(runtime) = env.runtime().filter(|_| wanted) else {
        return phase;
    };
    let mut values = collect::values(entities, checks, text_of);
    if values.len() > VALUE_CAP {
        let over = values.split_off(VALUE_CAP);
        effects.count_not_run(budget::OVER, over.len());
        for value in &over {
            for check in value.checks {
                for call in check.calls() {
                    phase.checks.add_not_run(value, call);
                }
            }
        }
    }
    let planned = planned(&values);
    let asking = Asking {
        env,
        effects,
        calls: ExtensionCalls::new(runtime.as_ref()),
        cache,
        dir: cache::dir_of(effects.journal()),
    };
    let answers = plans(&asking, &values, &planned, &mut phase.diagnostics);
    let settled = performed_and_judged(
        &asking,
        effects,
        disk,
        &values,
        &planned,
        &answers,
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
                &settled,
                registries,
                &mut misused,
                &mut phase,
            );
        }
    }
    phase
}

/// Which calls are dropped: those of a value's checks after one whose every branch failed in its plan (an invalid URL
/// is never fetched).
fn dropped(planned: &[Planned<'_>], answers: &BTreeMap<CallKey, PlannedItem>) -> HashSet<CallKey> {
    let failed = |one: &Planned<'_>| {
        answers
            .get(&one.key())
            .is_some_and(|plan| place::any_error(&plan.findings))
    };
    let mut dropped = HashSet::new();
    for (index, one) in planned.iter().enumerate() {
        let mut same_check = planned
            .iter()
            .filter(|other| other.value == one.value && other.check == one.check);
        if !same_check.all(failed) {
            continue;
        }
        for later in planned[index..]
            .iter()
            .filter(|later| later.value == one.value && later.check > one.check)
        {
            dropped.insert(later.key());
        }
    }
    dropped
}

/// Each answered call settled: its probes performed, and the judge call of those all answered. A failing judge (E028,
/// once) leaves its extension's values not judged.
fn performed_and_judged(
    asking: &Asking<'_>,
    effects: &Effects,
    disk: &dyn Disk,
    values: &[Value<'_>],
    planned: &[Planned<'_>],
    answers: &BTreeMap<CallKey, PlannedItem>,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<CallKey, Settled> {
    let dropped = dropped(planned, answers);
    let mut settled = BTreeMap::new();
    let mut waiting = Vec::new();
    let mut asked = Vec::new();
    for one in planned {
        let key = one.key();
        let Some(plan) = answers.get(&key) else {
            continue;
        };
        if dropped.contains(&key) {
            settled.insert(key, Settled::default());
            continue;
        }
        let fails = place::any_error(&plan.findings);
        if plan.probes.is_empty() || fails {
            settled.insert(
                key,
                Settled {
                    findings: plan.findings.clone(),
                    judged: true,
                    class: Some(OutcomeClass::Ok),
                    ..Settled::default()
                },
            );
            continue;
        }
        waiting.push(one);
        asked.push(dispatch::Asked {
            value: &values[one.value],
            call: one.call,
            probes: &plan.probes,
        });
    }
    let (readings, approvals) = dispatch::dispatch(asking.env, effects, disk, &asked);
    diagnostics.extend(approvals);
    let mut judge: BTreeMap<&str, Vec<CheckItem>> = BTreeMap::new();
    for (one, reading) in waiting.iter().zip(readings) {
        let key = one.key();
        let plan = &answers[&key];
        if !reading.answered() {
            settled.insert(
                key,
                Settled {
                    findings: plan.findings.clone(),
                    host: reading.refused,
                    class: reading.refused_class,
                    ..Settled::default()
                },
            );
            continue;
        }
        let class = reading
            .outcomes
            .iter()
            .map(|outcome| outcome.class)
            .find(|class| *class != OutcomeClass::Ok)
            .unwrap_or(OutcomeClass::Ok);
        settled.insert(
            key,
            Settled {
                findings: plan.findings.clone(),
                class: Some(class),
                observed: Observed {
                    at: reading.outcomes.iter().map(|o| o.observed_at).min(),
                    from_cache: reading.outcomes.iter().any(|o| o.from_cache),
                },
                ..Settled::default()
            },
        );
        let mut item = item_of(one, &values[one.value]);
        item.outcomes = reading.outcomes;
        judge.entry(&one.call.extension).or_default().push(item);
    }
    for (extension, items) in judge {
        match asking.ask(extension, CheckPhase::Judge, items) {
            Ok(items) => {
                for item in items {
                    if let Some(settled) = settled.get_mut(&CallKey::of(extension, item.id)) {
                        settled.findings.extend(item.findings);
                        settled.judged = true;
                    }
                }
            }
            Err(error) => diagnostics.push(error.diagnostic()),
        }
    }
    settled
}

fn item_of(one: &Planned<'_>, value: &Value<'_>) -> CheckItem {
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
    asking: &Asking<'_>,
    values: &[Value<'_>],
    planned: &[Planned<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<CallKey, PlannedItem> {
    let mut answers = BTreeMap::new();
    for declaration in asking.env.registries().declarations() {
        let extension = declaration.name();
        let items: Vec<CheckItem> = planned
            .iter()
            .filter(|one| one.call.extension == extension)
            .map(|one| item_of(one, &values[one.value]))
            .collect();
        if items.is_empty() {
            continue;
        }
        match asking.ask(extension, CheckPhase::Plan, items) {
            Ok(items) => {
                for item in items {
                    answers.insert(CallKey::of(extension, item.id), item);
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
    answers: &BTreeMap<CallKey, Settled>,
    registries: &specforge_registry::RegistryBuild,
    misused: &mut HashSet<(String, String)>,
    phase: &mut Phase,
) {
    let answer = |one: &&Planned<'_>| answers.get(&one.key());
    let passing = branches
        .iter()
        .find(|one| answer(one).is_some_and(|settled| settled.judged && !settled.fails()));
    let some_not_run = branches
        .iter()
        .any(|one| answer(one).is_none_or(|settled| !settled.judged && !settled.fails()));
    let any_of = matches!(check, ResolvedCheck::AnyOf(_));
    let one_of = any_of.then(|| {
        let names: Vec<&str> = branches
            .iter()
            .map(|one| one.call.checker.as_str())
            .collect();
        format!("one of: {}", names.join(", "))
    });
    for one in branches {
        let Some(settled) = answer(one) else {
            phase.checks.add_not_run(value, one.call);
            continue;
        };
        // What an `any_of` reports: the passing branch's findings, or every branch's when none passes.
        let reported = !any_of
            || match passing {
                Some(pass) => pass.id == one.id && pass.call.extension == one.call.extension,
                None => !some_not_run,
            };
        let status = match (any_of, passing.is_some(), some_not_run) {
            (false, _, _) if settled.fails() => CheckStatus::Failed,
            (false, _, _) if settled.judged => CheckStatus::Passed,
            (false, _, _) => CheckStatus::NotRun,
            (true, true, _) => CheckStatus::Passed,
            (true, false, true) => CheckStatus::NotRun,
            (true, false, false) => CheckStatus::Failed,
        };
        let mut kept = Vec::new();
        if reported {
            for finding in &settled.findings {
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
            for host in &settled.host {
                phase.diagnostics.push(host.clone());
                kept.push(host.code.to_string());
            }
        }
        phase.checks.add(
            value,
            one.call,
            status,
            settled.class.filter(|_| status != CheckStatus::NotRun),
            kept,
            settled.observed,
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
        observed: Observed,
    ) {
        self.0
            .entry(value.entity.to_string())
            .or_default()
            .push(ValueRow {
                field: value.field.to_string(),
                item: value.item,
                checker: call.qualified(),
                value: value.text.to_string(),
                status,
                outcome_class,
                observed_at: observed.at,
                from_cache: observed.from_cache,
                codes,
            });
    }

    fn add_not_run(&mut self, value: &Value<'_>, call: &CheckCall) {
        self.add(
            value,
            call,
            CheckStatus::NotRun,
            None,
            Vec::new(),
            Observed::default(),
        );
    }
}
