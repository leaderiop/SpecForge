//! The conversion, pure: the old entities through the table into one list of creates, the ledger, and the computed
//! values to compare once the target is written (ADR 0360 D1-D5). The pre-checks turn what would be an E080 or an
//! E002 into a listed stop before any write.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::{BTreeMap, BTreeSet};

use super::how;
use super::ledger::{Ledger, Why};
use super::proof::Proofs;
use super::read::{Index, OldEntity};
use super::table::{self, Home, Row, VERIFY};

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
    /// The test ids the written proofs name: what a gate runs to hold them.
    pub cited: BTreeSet<String>,
    /// For each (new id, new field) a reference was to be written in, the old ids the selection leaves out that it
    /// would have held, its own references and the inverse writes of entities left out alike (each ledgered
    /// `out_of_selection`): what a required field lacks because of `--only`, not the old project.
    pub left_out: BTreeMap<(String, String), BTreeSet<String>>,
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
/// row, a value in a refused field, a word a default field does not imply, an obligation with no row, a milestone
/// that targets a day before it starts (which the target refuses, so the converted project would not check clean),
/// two creates with one id.
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
                match *home {
                    Home::Refused => stops.push(stop(
                        e,
                        field,
                        format!("refused: no kind holds it (value {value})"),
                    )),
                    Home::Default(implied) if value.as_str() != Some(implied) => {
                        let kind = table::new_kind(&e.kind).unwrap_or_default();
                        stops.push(stop(
                            e,
                            field,
                            format!("no home for {value}: a {kind} implies only \"{implied}\""),
                        ));
                    }
                    _ => {}
                }
            }
        }
        if !e.obligations.is_empty() && table::row_of(&e.kind, VERIFY).is_none() {
            stops.push(stop(e, VERIFY, format!("no row for {}.{VERIFY}", e.kind)));
        }
        if table::new_kind(&e.kind) == Some("milestone")
            && let (Some(starts), Some(targets)) = (day(e, "start_date"), day(e, "target_date"))
            && targets < starts
        {
            stops.push(stop(
                e,
                "target_date",
                format!(
                    "{targets} is before its start_date {starts}: the target refuses a milestone that ships \
                     before it starts (gsd/E537)"
                ),
            ));
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

/// The `YYYY-MM-DD` day `e` writes in `field`. Two such days compare as calendar days by their text; any other
/// spelling is the source's own date error, never one the target adds.
fn day<'e>(e: &'e OldEntity, field: &str) -> Option<&'e str> {
    let text = e.fields.get(field)?.as_str()?;
    let shaped = text.len() == 10
        && text.bytes().enumerate().all(|(at, b)| match at {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        });
    shaped.then_some(text)
}

/// The ids `e` creates, each with the entity that makes it.
fn new_ids(e: &OldEntity) -> Vec<(String, String)> {
    let mut ids = Vec::new();
    if table::new_kind(&e.kind).is_some() {
        ids.push((e.id.clone(), format!("{} '{}'", e.kind, e.id)));
    }
    if e.fields.contains_key("solution") {
        ids.push((
            solution_id(&e.id),
            format!("{} '{}' (its solution)", e.kind, e.id),
        ));
    }
    ids
}

/// The id of the decision a feature's solution becomes.
fn solution_id(id: &str) -> String {
    format!("{id}_solution")
}

/// The rating a missing occurrence or detection is filled with: the worst, so an unrated risk is never read as low.
const UNRATED: u64 = 10;

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

    /// `value` as the member `member` of the block `field`, unless the block holds it already.
    fn member(&mut self, field: &str, member: &str, value: Input) {
        let entry = self
            .fields
            .entry(field.to_string())
            .or_insert_with(|| Input::Block(BTreeMap::new()));
        if let Input::Block(block) = entry {
            block.entry(member.to_string()).or_insert(value);
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

/// The plan as it is being made.
struct Planner<'a> {
    index: &'a Index,
    proofs: &'a Proofs,
    cited: BTreeSet<String>,
    /// The old ids this run converts into an entity of their own.
    converted: BTreeSet<String>,
    drafts: BTreeMap<String, Draft>,
    order: Vec<String>,
    ledger: Ledger,
    checks: Vec<Check>,
    created: usize,
    left_out: BTreeMap<(String, String), BTreeSet<String>>,
}

/// Convert `entities` (each passed [`prechecks`]); `left` is every old entity the selection leaves out (none for a
/// whole run), read only for the inverse writes it makes onto `entities`; `index` names every old entity's kind,
/// `proofs` what an invariant's obligations are proved by.
pub fn plan(entities: &[OldEntity], left: &[OldEntity], index: &Index, proofs: &Proofs) -> Planned {
    let mut planner = Planner {
        index,
        proofs,
        cited: BTreeSet::new(),
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
        left_out: BTreeMap::new(),
    };
    for e in entities {
        planner.draft(e);
    }
    for e in entities {
        for row in table::rows_of(&e.kind) {
            let values: Vec<Value> = if row.field == VERIFY {
                e.obligations
                    .iter()
                    .map(|(kind, text)| json!({"kind": kind, "text": text}))
                    .collect()
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
    for e in left {
        planner.inverses_left_out(e);
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
        let mut fields = BTreeMap::new();
        if matches!(kind, "milestone" | "term") {
            fields.insert("name".to_string(), Input::One(title.clone()));
        }
        self.add(Draft {
            kind,
            id: e.id.clone(),
            title,
            file: e.file.clone(),
            fields,
            inverted: BTreeSet::new(),
        });
    }

    fn add(&mut self, draft: Draft) {
        self.order.push(draft.id.clone());
        self.drafts.insert(draft.id.clone(), draft);
    }

    /// `draft`, created from the entity `from`, written right after it.
    fn add_after(&mut self, from: &str, draft: Draft) {
        let at = self
            .order
            .iter()
            .position(|id| id == from)
            .map_or(self.order.len(), |at| at + 1);
        self.order.insert(at, draft.id.clone());
        self.drafts.insert(draft.id.clone(), draft);
        self.created += 1;
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
                let expected = match how::expected(compare, value) {
                    how::Expect::Value(expected) => Some(expected),
                    how::Expect::Never => None,
                    how::Expect::Nothing => return,
                };
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
            // The value the new kind implies: the pre-checks stopped on any other.
            Home::Default(_) => {}
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

    /// The new id a reference to `id` is written as, when this run converts it; otherwise ledgered and none. `into` is
    /// the new field of `e`'s own entity it is written in (none for an inverse write): one the selection leaves out
    /// is recorded there.
    fn resolve(
        &mut self,
        e: &OldEntity,
        field: &str,
        into: Option<&str>,
        id: &str,
    ) -> Option<String> {
        if self.converted.contains(id) {
            return Some(id.to_string());
        }
        let why = match self.index.kind(id) {
            Some(kind) if table::new_kind(kind).is_some() => {
                if let Some(into) = into {
                    let at = (e.id.clone(), into.to_string());
                    self.left_out.entry(at).or_default().insert(id.to_string());
                }
                Why::OutOfSelection(id.to_string())
            }
            Some(kind) => Why::TargetLater {
                target: id.to_string(),
                pending: table::pending_of(kind),
            },
            None => Why::Unconverted(format!("'{id}' names no entity of the old project")),
        };
        self.ledger.record(&e.kind, &e.id, field, json!(id), why);
        None
    }

    /// The inverse writes `e`, an entity the selection leaves out, would make onto one it converts: each item the
    /// converted entity loses is ledgered on it, `out_of_selection` naming `e`, under the new field it would have been
    /// written in, and recorded there, so a required field it empties stops the run naming the selection.
    fn inverses_left_out(&mut self, e: &OldEntity) {
        for row in table::rows_of(&e.kind) {
            let Some(value) = e.fields.get(row.field) else {
                continue;
            };
            for home in row.homes {
                let Home::To {
                    field,
                    how: table::How::InvertOnto,
                    ..
                } = *home
                else {
                    continue;
                };
                let index = self.index;
                for id in how::items(value) {
                    let Some(kind) = index.kind(&id).filter(|_| self.converted.contains(&id))
                    else {
                        continue;
                    };
                    let why = Why::OutOfSelection(e.id.clone());
                    self.ledger.record(kind, &id, field, json!(e.id), why);
                    let at = (id, field.to_string());
                    self.left_out.entry(at).or_default().insert(e.id.clone());
                }
            }
        }
    }

    /// The required values the old entity lacks, filled and ledgered.
    fn fill(&mut self, e: &OldEntity) {
        let Some(kind) = table::new_kind(&e.kind) else {
            return;
        };
        let fills: Vec<(&str, Input)> = match kind {
            "decision" => vec![("origin", Input::One("project".to_string()))],
            "requirement" if e.kind == "constraint" => vec![
                ("area", Input::One("Constraint".to_string())),
                ("tier", Input::One("v1".to_string())),
            ],
            "requirement" => vec![
                ("area", Input::One("Feature".to_string())),
                (
                    "tier",
                    Input::One(how::feature_tier(how::FEATURE_DEFAULT).to_string()),
                ),
            ],
            "risk" => vec![
                ("occurrence", how::number(UNRATED)),
                ("detection", how::number(UNRATED)),
            ],
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
            cited,
            left_out,
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
            cited,
            left_out,
        }
    }
}
