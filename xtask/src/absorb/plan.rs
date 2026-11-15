//! The conversion, pure: the old entities through the table into one list of creates, the ledger, and the computed
//! values to compare once the target is written (ADR 0360 D1-D5). The pre-checks turn what would be an E080 or an
//! E002 into a listed stop before any write.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::{BTreeMap, BTreeSet};

use super::how;
use super::ledger::{Ledger, Why};
use super::read::{Index, OldEntity};
use super::table::{self, Home, Row, VERIFY};

/// Why the run stops before any write: an old entity, its field (empty: the entity), and why.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stop {
    pub kind: String,
    pub id: String,
    pub field: String,
    pub why: String,
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.kind.is_empty(), self.field.is_empty()) {
            (true, _) => write!(f, "'{}': {}", self.id, self.why),
            (false, true) => write!(f, "{} '{}': {}", self.kind, self.id, self.why),
            (false, false) => write!(
                f,
                "{} '{}' {}: {}",
                self.kind, self.id, self.field, self.why
            ),
        }
    }
}

/// A computed value to read back once the target is written: the old (kind, id, field) and value, the new entity
/// and field, and the value the old one means (`None`: no new value equals it, so it is always ledgered).
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub kind: String,
    pub id: String,
    pub field: String,
    pub value: Value,
    pub new_id: String,
    pub new_kind: &'static str,
    pub new_field: &'static str,
    pub expected: Option<Value>,
}

/// What the conversion plans.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    /// One create per converted entity, in the old order, each created entity after the one it comes from.
    pub creates: Vec<Edit>,
    pub ledger: Ledger,
    pub checks: Vec<Check>,
    /// Old entities that became an entity of their own.
    pub converted: usize,
    /// Entities the converter created that the old project did not hold.
    pub created: usize,
}

/// Every reason the run cannot convert `entities`, all at once: an entity of a kind no row converts, a field with no
/// row, a value in a refused field, a word a default field does not imply, an obligation with no row, two creates
/// with one id.
pub fn prechecks(entities: &[OldEntity]) -> Result<(), Vec<Stop>> {
    let mut stops = Vec::new();
    let stop = |e: &OldEntity, field: &str, why: String| Stop {
        kind: e.kind.clone(),
        id: e.id.clone(),
        field: field.to_string(),
        why,
    };
    for e in entities {
        if table::rows_of(&e.kind).next().is_none() {
            stops.push(stop(e, "", format!("no row converts a {} yet", e.kind)));
            continue;
        }
        for (field, value) in &e.fields {
            let Some(row) = table::row_of(&e.kind, field) else {
                stops.push(stop(e, field, format!("no row for {}.{field}", e.kind)));
                continue;
            };
            for home in row.homes {
                if *home == Home::Refused {
                    stops.push(stop(
                        e,
                        field,
                        format!("refused: no kind holds it (value {value})"),
                    ));
                }
            }
        }
        if !e.obligations.is_empty() && table::row_of(&e.kind, VERIFY).is_none() {
            stops.push(stop(e, VERIFY, format!("no row for {}.{VERIFY}", e.kind)));
        }
    }
    let mut by_id: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in entities {
        for (new_id, maker) in new_ids(e) {
            by_id.entry(new_id).or_default().push(maker);
        }
    }
    for (new_id, makers) in by_id.into_iter().filter(|(_, makers)| makers.len() > 1) {
        stops.push(Stop {
            kind: String::new(),
            id: new_id,
            field: String::new(),
            why: format!("two creates with one id, by {}", makers.join(" and ")),
        });
    }
    stops.sort();
    if stops.is_empty() { Ok(()) } else { Err(stops) }
}

/// The ids `e` creates, each with the entity that makes it.
fn new_ids(e: &OldEntity) -> Vec<(String, String)> {
    let mut ids = Vec::new();
    if table::new_kind(&e.kind).is_some() {
        ids.push((e.id.clone(), format!("{} '{}'", e.kind, e.id)));
    }
    ids
}

/// One entity being written: its create's parts.
#[derive(Debug, Clone)]
struct Draft {
    kind: &'static str,
    id: String,
    title: String,
    file: String,
    fields: BTreeMap<String, Input>,
    /// The fields other entities wrote by inversion: sorted by id once every entity is planned.
    inverted: BTreeSet<String>,
}

impl Draft {
    fn set(&mut self, field: &str, value: Input) {
        self.fields.insert(field.to_string(), value);
    }

    /// `items` after the list `field` holds, each once.
    fn append(&mut self, field: &str, items: Vec<String>) {
        let entry = self
            .fields
            .entry(field.to_string())
            .or_insert_with(|| Input::Many(Vec::new()));
        if let Input::Many(list) = entry {
            for item in items {
                if !list.contains(&item) {
                    list.push(item);
                }
            }
        }
    }
}

/// The plan as it is being made.
struct Planner<'a> {
    index: &'a Index,
    /// The old ids this run converts into an entity of their own.
    converted: BTreeSet<String>,
    drafts: BTreeMap<String, Draft>,
    order: Vec<String>,
    ledger: Ledger,
    checks: Vec<Check>,
    created: usize,
}

/// Convert `entities` (each passed [`prechecks`]); `index` names every old entity's kind.
pub fn plan(entities: &[OldEntity], index: &Index) -> Planned {
    let mut planner = Planner {
        index,
        converted: entities
            .iter()
            .filter(|e| table::new_kind(&e.kind).is_some())
            .map(|e| e.id.clone())
            .collect(),
        drafts: BTreeMap::new(),
        order: Vec::new(),
        ledger: Ledger::default(),
        checks: Vec::new(),
        created: 0,
    };
    for e in entities {
        planner.draft(e);
    }
    for e in entities {
        for row in table::rows_of(&e.kind) {
            if let Some(value) = e.fields.get(row.field) {
                for home in row.homes {
                    planner.home(e, row, home, value);
                }
            }
        }
        planner.fill(e);
    }
    planner.finish()
}

impl Planner<'_> {
    /// The entity `e` converts to, empty but for its header.
    fn draft(&mut self, e: &OldEntity) {
        let Some(kind) = table::new_kind(&e.kind) else {
            return;
        };
        let title = match &e.title {
            Some(title) if !title.trim().is_empty() => title.clone(),
            _ => {
                self.ledger.record(
                    &e.kind,
                    &e.id,
                    "title",
                    Value::Null,
                    Why::Filled {
                        field: format!("{kind}.title"),
                        value: json!(e.id),
                    },
                );
                e.id.clone()
            }
        };
        self.add(Draft {
            kind,
            id: e.id.clone(),
            title,
            file: e.file.clone(),
            fields: BTreeMap::new(),
            inverted: BTreeSet::new(),
        });
    }

    fn add(&mut self, draft: Draft) {
        self.order.push(draft.id.clone());
        self.drafts.insert(draft.id.clone(), draft);
    }

    fn own(&mut self, e: &OldEntity) -> Option<&mut Draft> {
        self.drafts.get_mut(&e.id)
    }

    fn home(&mut self, e: &OldEntity, row: &Row, home: &Home, value: &Value) {
        match *home {
            Home::To { field, how, .. } => self.to(e, row, field, how, value),
            Home::Computed {
                kind,
                field,
                compare,
            } => {
                let expected = how::expected(compare, value);
                self.checks.push(Check {
                    kind: e.kind.clone(),
                    id: e.id.clone(),
                    field: row.field.to_string(),
                    value: value.clone(),
                    new_id: e.id.clone(),
                    new_kind: kind,
                    new_field: field,
                    expected,
                });
            }
            // A refused value stopped the run before planning.
            Home::Refused => {}
        }
    }

    fn to(&mut self, e: &OldEntity, row: &Row, field: &str, how: table::How, value: &Value) {
        use table::How;
        match how {
            How::As => {
                let input = how::as_written(value);
                if let Some(draft) = self.own(e) {
                    match input {
                        Input::Many(items) => draft.append(field, items),
                        other => draft.set(field, other),
                    }
                }
            }
            How::Refs => {
                let ids: Vec<String> = how::items(value)
                    .into_iter()
                    .filter_map(|id| self.resolve(e, row.field, &id))
                    .collect();
                if let Some(draft) = self.own(e) {
                    draft.append(field, ids);
                }
            }
            How::InvertOnto => {
                for id in how::items(value) {
                    if let Some(target) = self.resolve(e, row.field, &id)
                        && let Some(draft) = self.drafts.get_mut(&target)
                    {
                        draft.append(field, vec![e.id.clone()]);
                        draft.inverted.insert(field.to_string());
                    }
                }
            }
            How::Outcome | How::Settled | How::Options => {
                if let Some(input) = how::word(how, value)
                    && let Some(draft) = self.own(e)
                {
                    draft.set(field, input);
                }
            }
        }
    }

    /// The new id a reference to `id` is written as, when this run converts it; otherwise ledgered and none.
    fn resolve(&mut self, e: &OldEntity, field: &str, id: &str) -> Option<String> {
        if self.converted.contains(id) {
            return Some(id.to_string());
        }
        let why = match self.index.kind(id) {
            Some(kind) if table::new_kind(kind).is_some() => Why::OutOfSelection(id.to_string()),
            Some(kind) => Why::TargetLater {
                target: id.to_string(),
                pending: table::pending_of(kind),
            },
            None => Why::Unconverted(format!("'{id}' names no entity of the old project")),
        };
        self.ledger.record(&e.kind, &e.id, field, json!(id), why);
        None
    }

    /// The required values the old entity lacks, filled and ledgered.
    fn fill(&mut self, e: &OldEntity) {
        let Some(kind) = table::new_kind(&e.kind) else {
            return;
        };
        let fills: Vec<(&str, Input)> = match kind {
            "decision" => vec![("origin", Input::One("project".to_string()))],
            _ => Vec::new(),
        };
        for (field, value) in fills {
            let Some(draft) = self.drafts.get_mut(&e.id) else {
                return;
            };
            if draft.fields.contains_key(field) {
                continue;
            }
            let written = how::input_json(&value);
            draft.set(field, value);
            self.ledger.record(
                &e.kind,
                &e.id,
                field,
                Value::Null,
                Why::Filled {
                    field: format!("{kind}.{field}"),
                    value: written,
                },
            );
        }
    }

    fn finish(self) -> Planned {
        let Planner {
            mut drafts,
            order,
            ledger,
            checks,
            created,
            ..
        } = self;
        let converted = order.len() - created;
        let creates = order
            .iter()
            .filter_map(|id| drafts.remove(id))
            .map(|mut draft| {
                for field in &draft.inverted {
                    if let Some(Input::Many(list)) = draft.fields.get_mut(field) {
                        list.sort();
                    }
                }
                Edit::Create {
                    kind: draft.kind.to_string(),
                    id: draft.id,
                    title: draft.title,
                    file: draft.file,
                    fields: draft.fields,
                    verify: Vec::new(),
                }
            })
            .collect();
        Planned {
            creates,
            ledger,
            checks,
            converted,
            created,
        }
    }
}
