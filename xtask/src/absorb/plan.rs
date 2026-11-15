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

impl Planned {
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

    fn to(&mut self, e: &OldEntity, row: &Row, field: &str, how: table::How, value: &Value) {
        use table::How;
        match how {
            How::As => match how::as_written(value) {
                Input::Many(items) => self.list(e, row.field, field, items),
                other => {
                    if let Some(draft) = self.own(e) {
                        draft.set(field, other);
                    }
                }
            },
            How::Words => self.list(e, row.field, field, how::items(value)),
            How::Ref => {
                if let Some(id) = value.as_str()
                    && let Some(id) = self.resolve(e, row.field, id)
                    && let Some(draft) = self.own(e)
                {
                    draft.set(field, Input::One(id));
                }
            }
            How::Steps => {
                let steps: Vec<BTreeMap<String, Input>> = how::items(value)
                    .into_iter()
                    .map(|action| BTreeMap::from([("action".to_string(), Input::One(action))]))
                    .collect();
                if let (false, Some(draft)) = (steps.is_empty(), self.drafts.get_mut(&e.id)) {
                    draft.set(field, Input::Blocks(steps));
                }
            }
            How::Solution => self.solution(e, field, value),
            How::FeatureTier => {
                let status = value.as_str().unwrap_or(how::FEATURE_DEFAULT);
                self.tier(e, field, status);
            }
            How::Text => {
                if let (Some(text), Some(draft)) = (value.as_str(), self.drafts.get_mut(&e.id)) {
                    draft.join(field, text.to_string());
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
            How::Category | How::Priority => {
                let word = value.as_str().unwrap_or_default();
                let mapped = if how == How::Category {
                    how::category(word)
                } else {
                    how::priority(word)
                };
                let written = match mapped {
                    how::Mapped::Same(word) => word,
                    how::Mapped::Collapsed(word) => {
                        self.collapsed(e, row.field, value, field, word);
                        word
                    }
                    how::Mapped::Unknown(why) => {
                        return self.unconverted(e, row.field, value.clone(), &why);
                    }
                };
                if let Some(draft) = self.own(e) {
                    draft.set(field, Input::One(written.to_string()));
                }
            }
            How::Sod(scale) => match how::rating(scale, value) {
                Ok(n) => {
                    if let Some(draft) = self.own(e) {
                        draft.set(field, how::number(n));
                    }
                }
                Err(why) => self.unconverted(e, row.field, value.clone(), &why),
            },
            How::Post => {
                for (member, rated) in value.as_object().into_iter().flatten() {
                    let scale = table::Scale::ALL.into_iter().find(|s| s.member() == member);
                    match scale.map(|scale| how::rating(scale, rated)) {
                        Some(Ok(n)) => {
                            if let Some(draft) = self.own(e) {
                                draft.member(field, member, how::number(n));
                            }
                        }
                        Some(Err(why)) => self.unconverted(e, row.field, value.clone(), &why),
                        // Its rpn is compared with the computed post_rpn.
                        None if member == "rpn" => {}
                        None => self.unconverted(
                            e,
                            row.field,
                            value.clone(),
                            &format!("the {member} member has no home in the post block"),
                        ),
                    }
                }
            }
            How::PostWord(scale) => {
                let member = scale.member();
                let block = e.fields.get("post_mitigation");
                if block.is_some_and(|block| block.get(member).is_some()) {
                    let why = format!("post_mitigation's {member} is written in its place");
                    return self.unconverted(e, row.field, value.clone(), &why);
                }
                match how::rating(scale, value) {
                    Ok(n) => {
                        if let Some(draft) = self.own(e) {
                            draft.member(field, member, how::number(n));
                        }
                    }
                    Err(why) => self.unconverted(e, row.field, value.clone(), &why),
                }
            }
            How::Bound => self.metric(e, field),
            // Read with the metric; alone, it joins the text.
            How::Threshold if e.fields.contains_key("metric") => {}
            How::Threshold => self.metric(e, field),
        }
    }

    /// `items` appended to the list `field`; a scheme reference among them is ledgered (see [`how::SCHEME_REF`]).
    fn list(&mut self, e: &OldEntity, old_field: &str, field: &str, items: Vec<String>) {
        let (refs, items): (Vec<String>, Vec<String>) =
            items.into_iter().partition(|item| how::is_scheme_ref(item));
        for item in refs {
            self.unconverted(e, old_field, json!(item), how::SCHEME_REF);
        }
        if let Some(draft) = self.own(e) {
            draft.append(field, items);
        }
    }

    /// A feature's solution as a decision of its own, affecting the feature's requirement.
    fn solution(&mut self, e: &OldEntity, field: &str, value: &Value) {
        let Some(own) = self.drafts.get(&e.id) else {
            return;
        };
        let id = solution_id(&e.id);
        let draft = Draft {
            kind: "decision",
            id: id.clone(),
            title: own.title.clone(),
            file: own.file.clone(),
            fields: BTreeMap::from([
                (field.to_string(), how::as_written(value)),
                ("origin".to_string(), Input::One("project".to_string())),
                ("affects".to_string(), Input::Many(vec![e.id.clone()])),
            ]),
            inverted: BTreeSet::new(),
        };
        self.add_after(&e.id, draft);
        let why = Why::Created {
            kind: "decision".to_string(),
            id,
        };
        self.ledger
            .record(&e.kind, &e.id, "solution", value.clone(), why);
    }

    /// A feature's tier from its status; a deprecated feature with no reason is given one.
    fn tier(&mut self, e: &OldEntity, field: &str, status: &str) {
        let tier = how::feature_tier(status);
        let unexplained = tier == "out_of_scope" && !e.fields.contains_key("reason");
        let Some(draft) = self.own(e) else {
            return;
        };
        draft.set(field, Input::One(tier.to_string()));
        if unexplained {
            draft.set("reason", Input::One(status.to_string()));
            let why = Why::Filled {
                field: "requirement.reason".to_string(),
                value: json!(status),
            };
            self.ledger
                .record(&e.kind, &e.id, "reason", Value::Null, why);
        }
    }

    /// The old metric and threshold as the `field` record and lines of the entity's `text`.
    fn metric(&mut self, e: &OldEntity, field: &str) {
        let metric = e.fields.get("metric");
        let threshold = e.fields.get("threshold");
        let read = how::metric(
            metric.and_then(Value::as_str).unwrap_or_default(),
            threshold.and_then(Value::as_str),
        );
        if let Some(value) = metric {
            if let Some(why) = &read.unconverted {
                self.unconverted(e, "metric", value.clone(), why);
            }
            if let Some(direction) = read.strict {
                let path = format!("{field}.direction");
                self.collapsed(e, "metric", value, &path, direction);
            }
        }
        if let (Some(value), false) = (threshold, read.threshold_taken) {
            let why = "no one-name metric it completes; it joins the text";
            self.unconverted(e, "threshold", value.clone(), why);
        }
        if let Some(draft) = self.drafts.get_mut(&e.id) {
            if let Some(record) = read.record {
                draft.set(field, Input::Entries(record));
            }
            for text in read.text {
                draft.join("text", text);
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

    /// `value` of the old `field` written as the coarser `word` in the new `path` (a field of the entity's kind).
    fn collapsed(&mut self, e: &OldEntity, field: &str, value: &Value, path: &str, word: &str) {
        let kind = table::new_kind(&e.kind).unwrap_or_default();
        self.ledger.record(
            &e.kind,
            &e.id,
            field,
            value.clone(),
            Why::Collapsed {
                field: format!("{kind}.{path}"),
                into: json!(word),
            },
        );
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
