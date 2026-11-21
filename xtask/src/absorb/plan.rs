//! The conversion, pure: the old entities through the table into one list of creates, the ledger, and the computed
//! values to compare once the target is written (ADR 0360 D1-D5). The pre-checks turn what would be an E080 or an
//! E002 into a listed stop before any write. Each create is an entity the old spec held: no entity is created that
//! it did not (ADR 0372 D6).
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
use super::table::{self, Fate, Home, Row, VERIFY};

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

/// Why a decision not in force is dropped whole (ADR 0372 D5): its status is not `accepted`, or it names a successor.
pub const NOT_IN_FORCE: &str = "a decision not in force (its status is not accepted, or it names a successor) no \
                                longer holds: it converts to nothing, and its ADR under docs/adr/ is its record \
                                (ADR 0372 D5)";

/// Why a phase's title loses its leading `Phase N:`: the label GSD prints computes the number from the id.
pub const NUMBERED: &str = "a number of the old milestones' numbering: GSD labels a phase `Phase <the last segment \
                            of its id>: <its title>`, so the title holds the name only";

/// The ledger's field for one `method` member of an entity dropped whole.
const METHOD: &str = "method";
/// The ledger's field for an entity's field annotations, dropped whole: one entry, each field to its annotations.
const ANNOTATIONS: &str = "annotations";

/// What the conversion plans.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    /// One create per converted entity, in the old order.
    pub creates: Vec<Edit>,
    /// The ledger, its `ids` section the map's.
    pub ledger: Ledger,
    pub checks: Vec<Check>,
    /// The id map: each converted entity's new id and path.
    pub ids: Ids,
    /// Old entities that became an entity of their own: one per create.
    pub converted: usize,
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

/// Every reason the run cannot convert `entities`, all at once (ADR 0372 D2): an entity of a kind with no fate; on an
/// entity of a converting kind, a field with no row, an obligation with no row, a method or a field annotation (no
/// `@gsd/core` field takes one); a milestone two releases list, a release with no version, two entities given one new
/// id (the links' and the id map's stops). An entity of a kind dropped whole needs no row: its fate's reason is every
/// value's.
pub fn prechecks(entities: &[OldEntity]) -> Result<(), Vec<Stop>> {
    let mut stops = Vec::new();
    let stop = |e: &OldEntity, field: &str, why: String| Stop {
        kind: e.kind.clone(),
        id: e.id.clone(),
        field: field.to_string(),
        why,
    };
    for e in entities {
        match table::fate(&e.kind) {
            None => {
                stops.push(stop(
                    e,
                    "",
                    format!("no fate for kind {} (ADR 0372 D1)", e.kind),
                ));
                continue;
            }
            Some(Fate::Dropped(_)) => continue,
            Some(Fate::To(_)) => {}
        }
        for field in e.fields.keys() {
            if table::row_of(&e.kind, field).is_none() {
                stops.push(stop(e, field, format!("no row for {}.{field}", e.kind)));
            }
        }
        if !e.obligations.is_empty() && table::row_of(&e.kind, VERIFY).is_none() {
            stops.push(stop(e, VERIFY, format!("no row for {}.{VERIFY}", e.kind)));
        }
        if !e.methods.is_empty() {
            stops.push(stop(
                e,
                METHOD,
                format!("a {} holds no method: no @gsd/core field takes one", e.kind),
            ));
        }
        for field in e.annotations.keys() {
            stops.push(stop(
                e,
                field,
                format!(
                    "a field annotation of a {} has no home in @gsd/core",
                    e.kind
                ),
            ));
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

    /// `text` after the text `field` holds, a blank line between; a blank `text` adds nothing.
    fn join(&mut self, field: &str, text: String) {
        if text.trim().is_empty() {
            return;
        }
        match self.fields.get_mut(field) {
            Some(Input::One(held)) if !held.is_empty() => {
                held.push_str("\n\n");
                held.push_str(&text);
            }
            _ => self.set(field, Input::One(text)),
        }
    }

    /// `bound` on the last line of the text `field` holds, after one space; `bound` alone when it holds none; a blank
    /// `bound` adds nothing.
    fn bound(&mut self, field: &str, bound: &str) {
        if bound.trim().is_empty() {
            return;
        }
        match self.fields.get_mut(field) {
            Some(Input::One(held)) if !held.trim().is_empty() => {
                held.truncate(held.trim_end().len());
                held.push(' ');
                held.push_str(bound);
            }
            _ => self.set(field, Input::One(bound.to_string())),
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
    /// Each requirement a written phase lists, by its new path, and the phase that keeps it: the first in source
    /// order that lists it (ADR 0372 D7).
    listings: BTreeMap<String, String>,
    drafts: BTreeMap<String, Draft>,
    order: Vec<String>,
    ledger: Ledger,
    checks: Vec<Check>,
}

/// Convert `entities` (each passed [`prechecks`]). The links, the id map and the phases' listings come first; a
/// reference is written through the map, and one to an entity the run does not convert is ledgered `dropped`, never
/// stubbed.
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
    planner.listings_of(entities);
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

/// Why `e` is dropped whole, if it is (ADR 0372 D1, D4, D5): its kind's fate drops it, it is a decision not in force
/// (its status is not `accepted`, or it writes `superseded_by`), or it is a milestone no converted release lists. None
/// for a kind with no fate, which stops the run.
fn dropped_whole(e: &OldEntity, links: &Links) -> Option<&'static str> {
    match table::fate(&e.kind)? {
        Fate::Dropped(reason) => Some(reason),
        Fate::To(_) if e.kind == super::links::MILESTONE && links.release_of(&e.id).is_none() => {
            Some(table::NO_RELEASE)
        }
        Fate::To(_)
            if e.kind == "decision"
                && (e.fields.get("status").and_then(Value::as_str) != Some("accepted")
                    || e.fields.contains_key("superseded_by")) =>
        {
            Some(NOT_IN_FORCE)
        }
        Fate::To(_) => None,
    }
}

/// The `@gsd/core` kind of the entity `e` converts to under `links`, read from the fate table: none for an entity
/// dropped whole, and for one of a kind with no fate.
pub fn converts_to(e: &OldEntity, links: &Links) -> Option<&'static str> {
    match dropped_whole(e, links) {
        Some(_) => None,
        None => table::fate(&e.kind)?.new_kind(),
    }
}

/// `e`'s obligations, each as the ledger writes it.
fn obligations(e: &OldEntity) -> Vec<Value> {
    e.obligations
        .iter()
        .map(|(kind, text)| json!({"kind": kind, "text": text}))
        .collect()
}

impl Planner {
    /// `e`, dropped whole: its title, each of its values, each obligation, each method and its field annotations (one
    /// entry, each field to its annotations) ledgered `dropped` with `reason`, so an entity with no value still leaves
    /// one entry (ADR 0372 D2).
    fn drop_whole(&mut self, e: &OldEntity, reason: &str) {
        let title = ("title", e.title.clone().map_or(Value::Null, Value::String));
        let values = e
            .fields
            .iter()
            .map(|(field, value)| (field.as_str(), value.clone()));
        let obligations = obligations(e).into_iter().map(|value| (VERIFY, value));
        let methods = e.methods.iter().map(|method| (METHOD, method.clone()));
        let annotations = (!e.annotations.is_empty()).then(|| {
            let held = e.annotations.clone().into_iter().collect();
            (ANNOTATIONS, Value::Object(held))
        });
        let all = std::iter::once(title)
            .chain(values)
            .chain(obligations)
            .chain(methods)
            .chain(annotations);
        for (field, value) in all {
            let why = Why::Dropped(reason.to_string());
            self.ledger.record(&e.kind, &e.id, field, value, why);
        }
    }

    /// The entity `e` converts to, empty but for its header, its name and its scope field, keyed by its new path. A
    /// phase is titled by its name: GSD labels it `Phase <its id's last segment>: <title>`, so an old title's
    /// `Phase N:` would print its number twice, and a number of another numbering: that prefix is ledgered `dropped`.
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
        let title = match kind {
            "phase" => {
                let name = how::phase_name(&title);
                let number = title[..title.len() - name.len()].trim_end();
                if !number.is_empty() {
                    let why = Why::Dropped(NUMBERED.to_string());
                    self.ledger
                        .record(&e.kind, &e.id, "title", json!(number), why);
                }
                name
            }
            _ => title,
        };
        let mut fields = BTreeMap::new();
        if matches!(kind, "milestone" | "phase") {
            fields.insert("name".to_string(), Input::One(title.clone()));
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

    /// The required values the old entity lacks, filled and ledgered. A requirement's category is the one rule's
    /// ([`ids::category`], ADR 0372 D3), which a constraint's own category word has already written; a constraint's
    /// and an invariant's tier is `v2` when the run writes a phase that does not list it (ADR 0372 D7), else `v1`.
    fn fill(&mut self, e: &OldEntity) {
        let Some(kind) = self.ids.get(&e.kind, &e.id).map(|new| new.kind) else {
            return;
        };
        let requirement = |tier: &str| {
            vec![
                ("category", Input::One(ids::category(e))),
                ("tier", Input::One(tier.to_string())),
            ]
        };
        let fills: Vec<(&str, Input)> = match (kind, e.kind.as_str()) {
            ("decision", _) => vec![("origin", Input::One("project".to_string()))],
            ("requirement", "feature") => requirement(how::feature_tier(how::FEATURE_DEFAULT)),
            ("requirement", _) => requirement(self.unlisted_tier(e, "v1")),
            ("phase", _) => self.phase_fills(e),
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
            ..
        } = self;
        let converted = order.len();
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
        }
    }
}
