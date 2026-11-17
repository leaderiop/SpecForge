//! The conversion, pure: the old entities through the table into one list of creates, the ledger, and the computed
//! values to compare once the target is written (ADR 0360 D1-D5). The pre-checks turn what would be an E080 or an
//! E002 into a listed stop before any write.
//!
//! The cross-entity links and the id map come first (ADR 0372 D3, D4): every new id and path is assigned before any
//! draft, every draft is keyed by its new path, and a reference is written through the map. The plan carries the map,
//! and the ledger records it as its `ids` section.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::{BTreeMap, BTreeSet};

use super::how;
use super::ids::{self, Ids};
use super::ledger::{Ledger, Why};
use super::links::Links;
use super::read::OldEntity;
use super::table::{self, Home, Row, VERIFY};

#[path = "plan/roadmap.rs"]
mod roadmap;
#[path = "plan/to.rs"]
mod to;

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

/// A computed value to read back once the target is written: the old (kind, id, field) and value, the new entity's
/// path and field, and the value the old one means (`None`: no new value equals it, so it is always ledgered).
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
    /// The ledger, its `ids` section the map's.
    pub ledger: Ledger,
    pub checks: Vec<Check>,
    /// The id map: each converted entity's new id and path.
    pub ids: Ids,
    /// Old entities that became an entity of their own.
    pub converted: usize,
    /// Entities the converter created that the old project did not hold.
    pub created: usize,
}

impl Planned {
    /// The files the creates write, relative to the project's root, each once.
    pub fn files(&self) -> Vec<std::path::PathBuf> {
        let files: BTreeSet<&str> = self
            .creates
            .iter()
            .filter_map(|edit| match edit {
                Edit::Create { file, .. } => Some(file.as_str()),
                _ => None,
            })
            .collect();
        files.into_iter().map(std::path::PathBuf::from).collect()
    }

    /// Ledger each computed value `computed(new id, field)` gives that differs from what its old value means.
    pub fn compare(&mut self, computed: impl Fn(&str, &str) -> Value) {
        for check in &self.checks {
            let new = computed(&check.new_id, check.new_field);
            if check.expected.as_ref() != Some(&new) {
                self.ledger.record(
                    &check.kind,
                    &check.id,
                    &check.field,
                    check.value.clone(),
                    Why::Computed {
                        field: format!("{}.{}", check.new_kind, check.new_field),
                        new,
                    },
                );
            }
        }
    }
}

/// Every reason the run cannot convert `entities`, all at once: an entity of a kind no row converts, a field with no
/// row, a value in a refused field, an obligation with no row, a milestone two releases list, a release with no
/// version, two entities given one new id (the links' and the id map's stops).
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
            if row.homes.contains(&Home::Refused) {
                stops.push(stop(
                    e,
                    field,
                    format!("refused: no kind holds it (value {value})"),
                ));
            }
        }
        if !e.obligations.is_empty() && table::row_of(&e.kind, VERIFY).is_none() {
            stops.push(stop(e, VERIFY, format!("no row for {}.{VERIFY}", e.kind)));
        }
    }
    let (links, linked) = Links::of(entities);
    stops.extend(linked);
    stops.extend(ids::assign(entities, &links).1);
    stops.sort();
    if stops.is_empty() { Ok(()) } else { Err(stops) }
}

/// One entity being written: its create's parts, its id the name within its scope (its scope field among its fields).
#[derive(Debug, Clone)]
struct Draft {
    kind: &'static str,
    id: String,
    title: String,
    file: String,
    fields: BTreeMap<String, Input>,
}

impl Draft {
    fn set(&mut self, field: &str, value: Input) {
        self.fields.insert(field.to_string(), value);
    }

    /// `text` after the text `field` holds, a blank line between.
    fn join(&mut self, field: &str, text: String) {
        match self.fields.get_mut(field) {
            Some(Input::One(held)) if !held.is_empty() => {
                held.push_str("\n\n");
                held.push_str(&text);
            }
            _ => self.set(field, Input::One(text)),
        }
    }

    /// `items` after the list `field` holds, each once; no items write nothing.
    fn append(&mut self, field: &str, items: Vec<String>) {
        if items.is_empty() {
            return;
        }
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

/// The plan as it is being made: drafts keyed by their new path.
#[derive(Default)]
struct Planner {
    links: Links,
    ids: Ids,
    /// Each old id of the run and the old kinds that hold it, so a reference the map does not hold says why.
    known: BTreeMap<String, Vec<(String, bool)>>,
    drafts: BTreeMap<String, Draft>,
    order: Vec<String>,
    ledger: Ledger,
    checks: Vec<Check>,
    created: usize,
}

/// Convert `entities` (each passed [`prechecks`]). The links and the id map come first; a reference is written
/// through the map, and one to an entity the run does not convert is ledgered `dropped`, never stubbed.
pub fn plan(entities: &[OldEntity]) -> Planned {
    let links = Links::of(entities).0;
    let ids = ids::assign(entities, &links).0;
    let wholes: Vec<Option<&'static str>> =
        entities.iter().map(|e| dropped_whole(e, &links)).collect();
    let mut known: BTreeMap<String, Vec<(String, bool)>> = BTreeMap::new();
    for (e, whole) in entities.iter().zip(&wholes) {
        known
            .entry(e.id.clone())
            .or_default()
            .push((e.kind.clone(), whole.is_some()));
    }
    let mut planner = Planner {
        links,
        ids,
        known,
        ..Planner::default()
    };
    for (e, whole) in entities.iter().zip(&wholes) {
        match whole {
            Some(reason) => planner.drop_whole(e, reason),
            None => planner.draft(e),
        }
    }
    for (e, _) in entities.iter().zip(&wholes).filter(|(_, w)| w.is_none()) {
        for row in table::rows_of(&e.kind) {
            let values: Vec<Value> = if row.field == VERIFY {
                obligations(e)
            } else {
                e.fields.get(row.field).cloned().into_iter().collect()
            };
            for value in &values {
                for home in row.homes {
                    planner.home(e, row, home, value);
                }
            }
        }
        planner.fill(e);
    }
    planner.finish()
}

/// Why `e` converts to nothing, if it does (ADR 0370 D8, ADR 0372 D4): every row of its kind drops its value, it is
/// a decision that no longer holds, or it is a milestone no converted release lists.
fn dropped_whole(e: &OldEntity, links: &Links) -> Option<&'static str> {
    if e.kind == super::links::MILESTONE && links.release_of(&e.id).is_none() {
        return Some(table::NO_RELEASE);
    }
    let status = e.fields.get("status").and_then(Value::as_str);
    table::converts_to_nothing(&e.kind, status)
}

/// The `@gsd/core` kind of the entity `e` converts to under `links`: none for an entity that converts to nothing,
/// and for a module, whose rows create no entity.
pub fn converts_to(e: &OldEntity, links: &Links) -> Option<&'static str> {
    dropped_whole(e, links).map_or_else(|| table::new_kind(&e.kind), |_| None)
}

/// `e`'s obligations, each as the ledger writes it.
fn obligations(e: &OldEntity) -> Vec<Value> {
    e.obligations
        .iter()
        .map(|(kind, text)| json!({"kind": kind, "text": text}))
        .collect()
}

impl Planner {
    /// `e`, which converts to nothing: its title, each of its values and each obligation ledgered `dropped` with
    /// `reason`, so an entity with no value still leaves one entry.
    fn drop_whole(&mut self, e: &OldEntity, reason: &str) {
        let title = ("title", e.title.clone().map_or(Value::Null, Value::String));
        let values = e
            .fields
            .iter()
            .map(|(field, value)| (field.as_str(), value.clone()));
        let obligations = obligations(e).into_iter().map(|value| (VERIFY, value));
        for (field, value) in std::iter::once(title).chain(values).chain(obligations) {
            let why = Why::Dropped(reason.to_string());
            self.ledger.record(&e.kind, &e.id, field, value, why);
        }
    }

    /// The entity `e` converts to, empty but for its header, its name and its scope field, keyed by its new path.
    fn draft(&mut self, e: &OldEntity) {
        let Some(new) = self.ids.get(&e.kind, &e.id).cloned() else {
            return;
        };
        let kind = new.kind;
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
        let mut fields = BTreeMap::new();
        match kind {
            "milestone" => {
                fields.insert("name".to_string(), Input::One(title.clone()));
            }
            "phase" => {
                fields.insert("name".to_string(), Input::One(how::phase_name(&title)));
            }
            _ => {}
        }
        if let Some((field, scope)) = &new.scope {
            fields.insert((*field).to_string(), Input::One(scope.clone()));
        }
        self.ledger.rename(&e.kind, &e.id, new.path());
        self.add(
            new.path(),
            Draft {
                kind,
                id: new.name,
                title,
                file: e.file.clone(),
                fields,
            },
        );
    }

    fn add(&mut self, path: String, draft: Draft) {
        self.order.push(path.clone());
        self.drafts.insert(path, draft);
    }

    /// `draft`, created from the entity `from`, written right after it; its path is its id.
    fn add_after(&mut self, from: &OldEntity, draft: Draft) {
        let from = self.ids.path(&from.kind, &from.id);
        let at = self
            .order
            .iter()
            .position(|path| Some(path) == from.as_ref())
            .map_or(self.order.len(), |at| at + 1);
        self.order.insert(at, draft.id.clone());
        self.drafts.insert(draft.id.clone(), draft);
        self.created += 1;
    }

    /// The draft `e` converts to.
    fn own(&mut self, e: &OldEntity) -> Option<&mut Draft> {
        let path = self.ids.path(&e.kind, &e.id)?;
        self.drafts.get_mut(&path)
    }

    fn home(&mut self, e: &OldEntity, row: &Row, home: &Home, value: &Value) {
        match *home {
            Home::To { field, how, .. } => self.to(e, row, field, how, value),
            Home::Computed {
                kind,
                field,
                compare,
            } => {
                let expected = match how::expected(compare, value) {
                    how::Expect::Value(expected) => Some(expected),
                    how::Expect::Never => None,
                };
                let Some(new_id) = self.ids.path(&e.kind, &e.id) else {
                    return;
                };
                self.checks.push(Check {
                    kind: e.kind.clone(),
                    id: e.id.clone(),
                    field: row.field.to_string(),
                    value: value.clone(),
                    new_id,
                    new_kind: kind,
                    new_field: field,
                    expected,
                });
            }
            Home::Later(pending) => {
                self.ledger.record(
                    &e.kind,
                    &e.id,
                    row.field,
                    value.clone(),
                    Why::Later(pending),
                );
            }
            // A refused value stopped the run before planning.
            Home::Refused => {}
            Home::Dropped(reason) => {
                self.ledger.record(
                    &e.kind,
                    &e.id,
                    row.field,
                    value.clone(),
                    Why::Dropped(reason.to_string()),
                );
            }
        }
    }

    fn unconverted(&mut self, e: &OldEntity, field: &str, value: Value, why: &str) {
        self.ledger.record(
            &e.kind,
            &e.id,
            field,
            value,
            Why::Unconverted(why.to_string()),
        );
    }

    /// The required values the old entity lacks, filled and ledgered. A constraint's tier is `v2` when the run writes
    /// a phase that does not list it (ADR 0372 D7).
    fn fill(&mut self, e: &OldEntity) {
        let Some(kind) = self.ids.get(&e.kind, &e.id).map(|new| new.kind) else {
            return;
        };
        let fills: Vec<(&str, Input)> = match kind {
            "decision" => vec![("origin", Input::One("project".to_string()))],
            "requirement" if e.kind == "constraint" => vec![
                ("category", Input::One("Constraint".to_string())),
                ("tier", Input::One(self.unlisted_tier(e, "v1").to_string())),
            ],
            "requirement" => vec![
                ("category", Input::One("Feature".to_string())),
                (
                    "tier",
                    Input::One(how::feature_tier(how::FEATURE_DEFAULT).to_string()),
                ),
            ],
            "phase" => self.phase_fills(e),
            _ => Vec::new(),
        };
        for (field, value) in fills {
            let Some(draft) = self.own(e) else {
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
            ids,
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
            .filter_map(|path| drafts.remove(path))
            .map(|draft| Edit::Create {
                kind: draft.kind.to_string(),
                id: draft.id,
                title: draft.title,
                file: draft.file,
                fields: draft.fields,
                verify: Vec::new(),
            })
            .collect();
        Planned {
            creates,
            ledger,
            checks,
            ids,
            converted,
            created,
        }
    }
}
