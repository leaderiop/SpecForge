//! The rows the phase gives: one per checker per value, what inspect, MCP, the hover and a collector's
//! `check_outcomes` read, and what a judged verdict digests of them.

use std::collections::BTreeMap;

use specforge_effects::OutcomeClass;
use specforge_protocol_types::{CheckOutcome, CheckStatus};
use specforge_registry::{CheckCall, QualifiedChecker};

use super::collect::Value;
use super::{Observed, Stage};
use crate::compiled::{CheckedValue, CheckedValues, ValueOutcome};

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
    /// When its oldest journaled answer was observed (Unix milliseconds); `None` when no answer has a record.
    pub observed_at: Option<u64>,
    pub from_cache: bool,
    /// The digest of what its probes answered, free of when; `None` when it did not run.
    pub digest: Option<String>,
    /// The codes it found, as written by the checker.
    pub codes: Vec<String>,
    /// The stage that read it (ADR 0350): only an ungated row is a judged input.
    pub stage: Stage,
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
                    digest: row.digest.clone(),
                    codes: row.codes.clone(),
                })
            })
            .collect()
    }

    /// What a judged verdict digests of them (ADR 0337 §6): a value not run is `unknown` (I207), and the verdict stays
    /// fresh. Only the ungated stage's rows are read: a gated value is no judged input (ADR 0350 D4).
    pub fn outcome_classes(&self) -> CheckedValues {
        let mut values = CheckedValues::default();
        for (id, rows) in &self.0 {
            for row in rows.iter().filter(|row| row.stage == Stage::Ungated) {
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

impl ValueChecks {
    pub(super) fn add(
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
                digest: observed.digest.filter(|_| status != CheckStatus::NotRun),
                codes,
                stage: value.stage,
            });
    }

    /// A held value's row (ADR 0350): no checker read it, so it has no outcome, no digest and no code.
    pub(super) fn add_held(&mut self, value: &Value<'_>, call: &CheckCall, status: CheckStatus) {
        self.add(value, call, status, None, Vec::new(), Observed::default());
    }

    /// `other`'s rows after this one's, entity by entity: the compile's second stage after its first.
    pub(crate) fn extend(&mut self, other: ValueChecks) {
        for (id, rows) in other.0 {
            self.0.entry(id).or_default().extend(rows);
        }
    }

    pub(super) fn add_not_run(&mut self, value: &Value<'_>, call: &CheckCall) {
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
