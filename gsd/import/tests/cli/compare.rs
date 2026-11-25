//! The comparator of an imported project with its hand-written twin (ADR 0391 D13): both as the graph export gives their
//! nodes, keyed by (kind, id path), each value folded the way the twins shorten what `.planning/` says, and what no
//! import can write left out: the computed and context fields (core's from its kinds golden,
//! `gsd/core/tests/golden/kinds.json`; `@gsd/security`'s from `gsd_security::kinds::computed()`), every verdict, and
//! every `context_pack`. What still differs is either listed, with its reason and the import's value, or a failure; a
//! listed difference that no longer occurs, or whose import value is not the listed one, is a failure too.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

/// One entity of either side: its kind and its graph id path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub kind: String,
    pub id: String,
}

/// The `field` of a [`Difference`] when the entity is in the import and not in the twin.
pub const IMPORT_ONLY: &str = "(import only)";
/// The `field` of a [`Difference`] when the entity is in the twin and not in the import.
pub const TWIN_ONLY: &str = "(twin only)";

/// The kinds whose title a `.planning/` line names, so the import's can equal the twin's. Every other kind's title is
/// display the twin authored (13-RESEARCH, "Twin fields with NO .planning source").
const TITLED: [&str; 6] = [
    "phase",
    "plan",
    "task",
    "uat_test",
    "deliverable",
    "artifact",
];

/// The imported key `(kind, import id)` that stands for the twin's `twin id` on a fixture: the twin invents an id where
/// `.planning/` has none (an out-of-scope row is `OOS-<NN>` in the import, ADR 0391 D4).
pub const ID_PAIRS: [(&str, &str, &str, &str); 2] = [
    ("taskboard", "requirement", "OOS-01", "SHARE-01"),
    ("ledger", "requirement", "OOS-01", "CURR-01"),
];

/// What a listed difference's import value is, folded as [`differences`] folds it: a listed field is checked on both
/// sides, never exempt from a value check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// Exactly this JSON value.
    Is(&'static str),
    /// No value: the field is absent or empty in the import.
    Absent,
}

impl Expect {
    /// Whether the folded import value `import` is this one.
    pub fn admits(self, import: &Value) -> bool {
        match self {
            Expect::Is(json) => {
                serde_json::from_str::<Value>(json).is_ok_and(|expected| expected == *import)
            }
            Expect::Absent => import.is_null(),
        }
    }
}

/// One way the two sides differ: on the entity (twin's key once paired) and the field (`title`, a field name,
/// [`IMPORT_ONLY`] or [`TWIN_ONLY`]), with each side's folded value (`null` when absent or empty).
#[derive(Debug, Clone, PartialEq)]
pub struct Difference {
    pub kind: String,
    pub id: String,
    pub field: String,
    pub twin: Value,
    pub import: Value,
}

/// What [`compare`] leaves out: `kind.field` and `kind.field.member` of every computed, context or verdict writer.
pub struct Excluded(BTreeSet<String>);

impl Excluded {
    /// Core's from the kinds golden and `@gsd/security`'s computed fields.
    pub fn gsd() -> Self {
        let golden: Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../core/tests/golden/kinds.json"
            ))
            .expect("the kinds golden"),
        )
        .expect("the golden is JSON");
        let mut out = Self::of(&golden["entities"]);
        out.0.extend(gsd_security::kinds::computed());
        out
    }

    /// The excluded fields and members of a `describe entities` answer.
    pub fn of(entities: &Value) -> Self {
        let excluded = |f: &Value| {
            matches!(
                f["writer"].as_str(),
                Some("computed" | "context" | "verdict")
            )
        };
        let mut out = BTreeSet::new();
        for kind in entities["items"].as_array().into_iter().flatten() {
            let name = kind["name"].as_str().unwrap_or_default();
            for field in kind["fields"].as_array().into_iter().flatten() {
                let field_name = field["name"].as_str().unwrap_or_default();
                if excluded(field) {
                    out.insert(format!("{name}.{field_name}"));
                }
                for member in field["members"].as_array().into_iter().flatten() {
                    if excluded(member) {
                        let member_name = member["name"].as_str().unwrap_or_default();
                        out.insert(format!("{name}.{field_name}.{member_name}"));
                    }
                }
            }
        }
        Excluded(out)
    }

    fn field(&self, kind: &str, field: &str) -> bool {
        field == "context_pack" || self.0.contains(&format!("{kind}.{field}"))
    }

    fn member(&self, kind: &str, field: &str, member: &str) -> bool {
        self.0.contains(&format!("{kind}.{field}.{member}"))
    }
}

/// `s` as the twins shorten it: every backtick removed, trimmed, one trailing `.` dropped, the first character
/// lowercased.
pub fn fold(s: &str) -> String {
    let unticked: String = s.chars().filter(|c| *c != '`').collect();
    let trimmed = unticked.trim();
    let trimmed = trimmed.strip_suffix('.').unwrap_or(trimmed);
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `value` folded throughout, the excluded members of `kind.field` dropped, and an empty string, list or block, or
/// `null`, as `null`.
fn folded(value: &Value, kind: &str, field: &str, excluded: &Excluded) -> Value {
    let out = match value {
        Value::String(s) => Value::String(fold(s)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| folded(item, kind, field, excluded))
                .filter(|item| !item.is_null())
                .collect(),
        ),
        Value::Object(members) => Value::Object(
            members
                .iter()
                .filter(|(member, _)| !excluded.member(kind, field, member))
                .map(|(member, v)| (member.clone(), folded(v, kind, field, excluded)))
                .filter(|(_, v)| !v.is_null())
                .collect::<Map<String, Value>>(),
        ),
        other => other.clone(),
    };
    let empty = match &out {
        Value::String(s) => s.is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(members) => members.is_empty(),
        Value::Null => true,
        _ => false,
    };
    if empty { Value::Null } else { out }
}

/// A side's nodes by key, each with its comparable values: `title` for a [`TITLED`] kind, then every field not
/// excluded, folded. `pairs` maps an imported (kind, id) to the twin's id.
fn sided(
    nodes: &[Value],
    pairs: &[(&str, &str, &str)],
    excluded: &Excluded,
) -> BTreeMap<Key, BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for node in nodes {
        let kind = node["kind"].as_str().unwrap_or_default().to_string();
        let id = node["id"].as_str().unwrap_or_default();
        let id = pairs
            .iter()
            .find(|(k, from, _)| *k == kind && *from == id)
            .map_or(id, |(_, _, to)| to)
            .to_string();
        let mut values = BTreeMap::new();
        if TITLED.contains(&kind.as_str()) {
            values.insert(
                "title".to_string(),
                folded(&node["title"], &kind, "title", excluded),
            );
        }
        for (field, value) in node["fields"].as_object().into_iter().flatten() {
            if !excluded.field(&kind, field) {
                values.insert(field.clone(), folded(value, &kind, field, excluded));
            }
        }
        out.insert(Key { kind, id }, values);
    }
    out
}

/// Every difference of `import` from `twin` (graph nodes), the imported keys `pairs` names (kind, import id, twin id)
/// mapped first.
pub fn differences(
    import: &[Value],
    twin: &[Value],
    pairs: &[(&str, &str, &str)],
    excluded: &Excluded,
) -> Vec<Difference> {
    let import = sided(import, pairs, excluded);
    let twin = sided(twin, &[], excluded);
    let mut out = Vec::new();
    let keys: BTreeSet<&Key> = import.keys().chain(twin.keys()).collect();
    for key in keys {
        let one = |field: &str, twin: Value, import: Value| Difference {
            kind: key.kind.clone(),
            id: key.id.clone(),
            field: field.to_string(),
            twin,
            import,
        };
        match (twin.get(key), import.get(key)) {
            (None, Some(_)) => out.push(one(IMPORT_ONLY, Value::Null, Value::Bool(true))),
            (Some(_), None) => out.push(one(TWIN_ONLY, Value::Bool(true), Value::Null)),
            (Some(t), Some(i)) => {
                let fields: BTreeSet<&String> = t.keys().chain(i.keys()).collect();
                for field in fields {
                    let tv = t.get(field).cloned().unwrap_or(Value::Null);
                    let iv = i.get(field).cloned().unwrap_or(Value::Null);
                    if tv != iv {
                        out.push(one(field, tv, iv));
                    }
                }
            }
            (None, None) => {}
        }
    }
    out
}

/// What [`compare`] found wrong: the differences no row lists, the rows listing a difference that no longer occurs, and
/// the listed differences whose import value is not the row's. All empty is a pass.
#[derive(Debug, Default, PartialEq)]
pub struct Compared {
    pub unlisted: Vec<Difference>,
    pub gone: Vec<(String, String, String)>,
    pub wrong: Vec<Difference>,
}

/// `import` against `twin` (graph nodes), with `pairs` (kind, import id, twin id) and `listed`, the (kind, id, field)
/// of each known difference and its import value.
pub fn compare(
    import: &[Value],
    twin: &[Value],
    pairs: &[(&str, &str, &str)],
    listed: &[(&str, &str, &str, Expect)],
    excluded: &Excluded,
) -> Compared {
    let found = differences(import, twin, pairs, excluded);
    let row = |d: &Difference| {
        listed
            .iter()
            .find(|(k, id, f, _)| d.kind == *k && d.id == *id && d.field == *f)
    };
    let unlisted = found.iter().filter(|d| row(d).is_none()).cloned().collect();
    let wrong = found
        .iter()
        .filter(|d| row(d).is_some_and(|(.., expect)| !expect.admits(&d.import)))
        .cloned()
        .collect();
    let gone = listed
        .iter()
        .filter(|(k, id, f, _)| {
            !found
                .iter()
                .any(|d| d.kind == *k && d.id == *id && d.field == *f)
        })
        .map(|(k, id, f, _)| (k.to_string(), id.to_string(), f.to_string()))
        .collect();
    Compared {
        unlisted,
        gone,
        wrong,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn node(kind: &str, id: &str, title: &str, fields: Value) -> Value {
        json!({"kind": kind, "id": id, "title": title, "fields": fields})
    }

    fn excluded() -> Excluded {
        Excluded::of(&json!({"items": [{"name": "plan", "fields": [
            {"name": "status", "writer": "computed"},
            {"name": "context_refs"},
            {"name": "tasks", "members": [{"name": "done", "writer": "computed"}, {"name": "name"}]},
        ]}]}))
    }

    #[test]
    fn fold_shortens_as_the_twins_do() {
        assert_eq!(fold("  `Complete` tasks."), "complete tasks");
        assert_eq!(fold("Ends.."), "ends.");
        assert_eq!(fold("``"), "");
    }

    #[test]
    fn a_folded_match_is_no_difference() {
        let import = [node(
            "plan",
            "v1.0/01/01-01",
            "01-01: `list` tasks.",
            json!({"goal": "Lists `tasks`.", "context_refs": [], "status": "complete",
                   "tasks": [{"name": "A", "done": true}]}),
        )];
        let twin = [node(
            "plan",
            "v1.0/01/01-01",
            "01-01: list tasks",
            json!({"goal": "lists tasks", "tasks": [{"name": "a"}]}),
        )];
        assert_eq!(differences(&import, &twin, &[], &excluded()), []);
    }

    #[test]
    fn a_real_mismatch_is_a_difference() {
        let import = [node("plan", "P", "P", json!({"goal": "one"}))];
        let twin = [node("plan", "P", "P", json!({"goal": "two"}))];
        let found = differences(&import, &twin, &[], &excluded());
        assert_eq!(
            found,
            [Difference {
                kind: "plan".into(),
                id: "P".into(),
                field: "goal".into(),
                twin: json!("two"),
                import: json!("one"),
            }]
        );
    }

    #[test]
    fn a_title_is_compared_only_for_a_titled_kind() {
        let import = [
            node("plan", "P", "One", json!({})),
            node("research", "R", "One", json!({})),
        ];
        let twin = [
            node("plan", "P", "Two", json!({})),
            node("research", "R", "Two", json!({})),
        ];
        let found = differences(&import, &twin, &[], &excluded());
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            (found[0].kind.as_str(), found[0].field.as_str()),
            ("plan", "title")
        );
    }

    #[test]
    fn a_pair_maps_the_imported_key_to_the_twins() {
        let import = [node("requirement", "OOS-01", "x", json!({"tier": "out"}))];
        let twin = [node("requirement", "SHARE-01", "y", json!({"tier": "out"}))];
        let pairs = [("requirement", "OOS-01", "SHARE-01")];
        assert_eq!(differences(&import, &twin, &pairs, &excluded()), []);
        assert_eq!(differences(&import, &twin, &[], &excluded()).len(), 2);
    }

    /// A pair stands only for an id the import mints (ADR 0391 D4): an out-of-scope requirement, once per fixture.
    #[test]
    fn id_pairs_name_only_minted_out_of_scope_ids() {
        for (fixture, kind, from, to) in ID_PAIRS {
            assert_eq!(kind, "requirement", "{fixture}");
            assert!(
                from.starts_with("OOS-") && !to.starts_with("OOS-"),
                "{fixture}"
            );
        }
        let fixtures: BTreeSet<&str> = ID_PAIRS.iter().map(|(f, ..)| *f).collect();
        assert_eq!(fixtures.len(), ID_PAIRS.len());
    }

    #[test]
    fn an_entity_on_one_side_is_a_difference() {
        let import = [node("plan", "A", "A", json!({}))];
        let twin = [node("plan", "B", "B", json!({}))];
        let found: Vec<(String, String)> = differences(&import, &twin, &[], &excluded())
            .into_iter()
            .map(|d| (d.id, d.field))
            .collect();
        assert_eq!(
            found,
            [
                ("A".to_string(), IMPORT_ONLY.to_string()),
                ("B".to_string(), TWIN_ONLY.to_string())
            ]
        );
    }

    #[test]
    fn a_listed_difference_that_is_gone_fails() {
        let same = [node("plan", "P", "P", json!({"goal": "g"}))];
        let listed = [("plan", "P", "goal", Expect::Is(r#""g""#))];
        let compared = compare(&same, &same, &[], &listed, &excluded());
        assert_eq!(compared.unlisted, []);
        assert_eq!(compared.gone, [("plan".into(), "P".into(), "goal".into())]);
    }

    #[test]
    fn an_unlisted_difference_fails_and_a_listed_one_passes() {
        let import = [node("plan", "P", "P", json!({"goal": "one"}))];
        let twin = [node("plan", "P", "P", json!({"goal": "two"}))];
        let unlisted = compare(&import, &twin, &[], &[], &excluded());
        assert_eq!(unlisted.unlisted.len(), 1);
        let row = [("plan", "P", "goal", Expect::Is(r#""one""#))];
        let listed = compare(&import, &twin, &[], &row, &excluded());
        assert_eq!(listed, Compared::default());
    }

    /// A listed field is no exemption: an import value other than the row's fails, the twin's differing or not.
    #[test]
    fn a_listed_difference_with_another_import_value_fails() {
        let import = [node("plan", "P", "P", json!({"goal": "", "name": "one"}))];
        let twin = [node(
            "plan",
            "P",
            "P",
            json!({"goal": "two", "name": "two"}),
        )];
        let rows = [
            ("plan", "P", "goal", Expect::Is(r#""one""#)),
            ("plan", "P", "name", Expect::Absent),
        ];
        let compared = compare(&import, &twin, &[], &rows, &excluded());
        let wrong: Vec<(&str, &Value)> = compared
            .wrong
            .iter()
            .map(|d| (d.field.as_str(), &d.import))
            .collect();
        assert_eq!(wrong, [("goal", &Value::Null), ("name", &json!("one"))]);
        let rows = [
            ("plan", "P", "goal", Expect::Absent),
            ("plan", "P", "name", Expect::Is(r#""one""#)),
        ];
        assert_eq!(
            compare(&import, &twin, &[], &rows, &excluded()),
            Compared::default()
        );
    }

    #[test]
    fn the_gsd_exclusions_hold_the_computed_and_context_fields() {
        let excluded = Excluded::gsd();
        assert!(excluded.field("plan", "context_pack"));
        let security = gsd_security::kinds::computed();
        assert!(!security.is_empty());
        assert!(security.iter().all(|f| excluded.0.contains(f)));
        assert!(excluded.0.len() > 80, "{}", excluded.0.len());
    }
}
