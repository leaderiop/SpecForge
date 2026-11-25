//! A plan's `<task>` elements and its frontmatter `must_haves` (13-RESEARCH Part B, the `NN-MM-PLAN.md` table; the rows
//! are `table::sections::PLAN_SHARED` and `QUICK_PLAN`): each `auto`, `tdd` or `tracer` task is `task T<n>`, each truth
//! `truth TR<n>`, each artifact `artifact AR<n>`, each key link `key_link KL<n>`, numbered in document order and scoped
//! by the plan. A `checkpoint:*` task and a `<behavior>` block are reported with the table's reasons.

use std::collections::BTreeMap;

use serde_json::{Map, Value as Json, json};

use super::plan::{Doc, elements, items, list};
use crate::frontmatter::{Located, Value};
use crate::records::{Record, Records};
use crate::report::Report;

/// The task types a `task` takes; a `checkpoint:*` task is a `checkpoint` (Tier 2).
const TASK_TYPES: &[&str] = &["auto", "tdd", "tracer"];

/// The table's key of a mapped task.
const TASK: &str = "<task type=\"auto|tdd|tracer\">";

/// One entity scoped by the plan `plan`, written to `file`.
pub fn scoped(
    doc: &Doc<'_>,
    kind: &'static str,
    id: String,
    title: String,
    (plan, file): (&str, &str),
    mut fields: BTreeMap<String, Json>,
    line: usize,
) -> Record {
    fields.insert("plan".to_string(), json!(plan));
    Record {
        kind,
        id: format!("{plan}/{id}"),
        title,
        file: file.to_string(),
        fields,
        refs: vec![plan.to_string()],
        links: Vec::new(),
        source: doc.at(line),
    }
}

/// Reads the `<task>` elements of `body` into tasks of the plan `plan`, written to `file`.
pub fn tasks(
    doc: &Doc<'_>,
    body: &str,
    plan: &str,
    file: &str,
    records: &mut Records,
    report: &mut Report,
) {
    let mut n = 0;
    for t in elements(doc, body, 1, "task", report) {
        let kind = t
            .attrs
            .iter()
            .find(|(k, _)| k == "type")
            .map_or("", |(_, v)| v.as_str());
        if kind.starts_with("checkpoint:") {
            let key = "<task type=\"checkpoint:*\">";
            report.push(doc.row(key, t.line, key));
            continue;
        }
        if !TASK_TYPES.contains(&kind) {
            report.push(doc.unmapped(
                t.line,
                "<task>",
                format!("a task of type `{kind}`, none of auto, tdd, tracer or checkpoint:*: not imported"),
            ));
            continue;
        }
        n += 1;
        let mut fields = BTreeMap::new();
        fields.insert("type".to_string(), json!(kind));
        let child = |name: &str, report: &mut Report| {
            elements(doc, &t.body, t.line, name, report)
                .into_iter()
                .next()
        };
        let title = child("name", report).map_or_else(
            || format!("T{n}"),
            |c| task_title(c.body.trim()).to_string(),
        );
        if let Some(c) = child("files", report) {
            let files: Vec<&str> = c
                .body
                .split(',')
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .collect();
            if !files.is_empty() {
                fields.insert("files".to_string(), json!(files));
            }
        }
        if let Some(c) = child("read_first", report) {
            // A `- path (why)` line: the path is its first word.
            let paths: Vec<&str> = c
                .body
                .split('\n')
                .filter_map(|l| l.trim().strip_prefix("- "))
                .filter_map(|l| l.split_whitespace().next())
                .map(|p| p.trim_matches('`'))
                .filter(|p| !p.is_empty())
                .collect();
            if !paths.is_empty() {
                fields.insert("read_first".to_string(), json!(paths));
            }
        }
        for (name, field) in [("action", "action"), ("done", "done")] {
            if let Some(c) = child(name, report).filter(|c| !c.body.trim().is_empty()) {
                fields.insert(field.to_string(), json!(c.body.trim()));
            }
        }
        if let Some(c) = child("verify", report) {
            verify(doc, &c.body, c.line, &mut fields, report);
        }
        if let Some(c) = child("acceptance_criteria", report) {
            let all = items(&c.body);
            if !all.is_empty() {
                fields.insert("acceptance".to_string(), json!(all));
            }
        }
        if let Some(c) = child("behavior", report) {
            report.push(doc.row("<behavior>", c.line, TASK));
        }
        let record = scoped(
            doc,
            "task",
            format!("T{n}"),
            title,
            (plan, file),
            fields,
            t.line,
        );
        records.add(record, report);
    }
}

/// A `<name>`'s title: the text after `Task <N>: `, or all of it.
fn task_title(name: &str) -> &str {
    name.strip_prefix("Task ")
        .and_then(|rest| rest.split_once(": "))
        .filter(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .map_or(name, |(_, title)| title.trim())
}

/// A `<verify>` body: its `<automated>` commands, each with the `<fails_when>` of the same rank, as `verify` blocks;
/// a body with none as `verify_text`.
fn verify(
    doc: &Doc<'_>,
    body: &str,
    line: usize,
    fields: &mut BTreeMap<String, Json>,
    report: &mut Report,
) {
    let automated = elements(doc, body, line, "automated", report);
    if automated.is_empty() {
        if !body.trim().is_empty() {
            fields.insert("verify_text".to_string(), json!(body.trim()));
        }
        return;
    }
    let fails = elements(doc, body, line, "fails_when", report);
    let blocks: Vec<Json> = automated
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let mut block = Map::new();
            block.insert("automated".to_string(), json!(a.body.trim()));
            if let Some(f) = fails.get(i) {
                block.insert("fails_when".to_string(), json!(f.body.trim()));
            }
            Json::Object(block)
        })
        .collect();
    fields.insert("verify".to_string(), json!(blocks));
}

/// Reads the frontmatter `must_haves` map `v` into the truths, artifacts and key links of the plan `plan`.
pub fn must_haves(
    doc: &Doc<'_>,
    v: &Located<Value>,
    plan: &str,
    file: &str,
    records: &mut Records,
    report: &mut Report,
) {
    let Value::Map(entries) = &v.value else {
        report.push(doc.unmapped(
            v.line,
            "must_haves",
            "must_haves is not a map: not imported".to_string(),
        ));
        return;
    };
    for (key, value) in entries {
        let section = format!("must_haves.{key}");
        let read = match key.as_str() {
            "truths" => truth,
            "artifacts" => artifact,
            "key_links" => key_link,
            _ => {
                report.push(doc.row(&section, value.line, &section));
                continue;
            }
        };
        let items: &[Located<Value>] = match &value.value {
            Value::List(items) => items,
            Value::Str(s) if s.trim().is_empty() => &[],
            _ => {
                report.push(doc.unmapped(
                    value.line,
                    &section,
                    format!("{section} is not a list: not imported"),
                ));
                continue;
            }
        };
        for (i, item) in items.iter().enumerate() {
            let record = read(doc, item, i + 1, (plan, file), report);
            if let Some(record) = record {
                records.add(record, report);
            }
        }
    }
}

/// A truth: one text.
fn truth(
    doc: &Doc<'_>,
    item: &Located<Value>,
    n: usize,
    owner: (&str, &str),
    report: &mut Report,
) -> Option<Record> {
    let Some(text) = item.value.as_str().map(str::trim).filter(|t| !t.is_empty()) else {
        report.push(doc.unmapped(
            item.line,
            "must_haves.truths",
            "a truth that is not one text: not imported".to_string(),
        ));
        return None;
    };
    let fields = BTreeMap::from([("text".to_string(), json!(text))]);
    Some(scoped(
        doc,
        "truth",
        format!("TR{n}"),
        text.to_string(),
        owner,
        fields,
        item.line,
    ))
}

/// The members of a must-have map `item` that the kind `kind` takes, each through `read`; any other member reported.
fn members(
    doc: &Doc<'_>,
    item: &Located<Value>,
    section: &str,
    read: impl Fn(&str, &Value) -> Option<Option<Json>>,
    report: &mut Report,
) -> Option<BTreeMap<String, Json>> {
    let Value::Map(entries) = &item.value else {
        report.push(doc.unmapped(
            item.line,
            section,
            "an item that is not a map: not imported".to_string(),
        ));
        return None;
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        match read(key, &value.value) {
            Some(Some(json)) => {
                fields.insert(key.clone(), json);
            }
            Some(None) => report.push(doc.unmapped(
                value.line,
                section,
                format!("`{key}` is not a value its field takes: not imported"),
            )),
            None => report.push(doc.unmapped(
                value.line,
                section,
                format!("a member `{key}` no row of the table names: no field holds it"),
            )),
        }
    }
    Some(fields)
}

/// A text member's value; none when it is not one text.
fn text(value: &Value) -> Option<Json> {
    value
        .as_str()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| json!(t))
}

/// An artifact: `path`, `provides`, `contains`, `min_lines` (a whole number), `exports` (a list).
fn artifact(
    doc: &Doc<'_>,
    item: &Located<Value>,
    n: usize,
    owner: (&str, &str),
    report: &mut Report,
) -> Option<Record> {
    let section = "must_haves.artifacts";
    let fields = members(
        doc,
        item,
        section,
        |key, value| match key {
            "path" | "provides" | "contains" => Some(text(value)),
            "min_lines" => Some(
                value
                    .as_str()
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .map(|n| json!(n)),
            ),
            "exports" => Some(list(value).map(|l| json!(l))),
            _ => None,
        },
        report,
    )?;
    let title = ["provides", "path"]
        .iter()
        .find_map(|k| fields.get(*k).and_then(Json::as_str))
        .map_or_else(|| format!("AR{n}"), str::to_string);
    Some(scoped(
        doc,
        "artifact",
        format!("AR{n}"),
        title,
        owner,
        fields,
        item.line,
    ))
}

/// A key link: `from`, `to`, `via`, `pattern`; its title `<from> -> <to>`.
fn key_link(
    doc: &Doc<'_>,
    item: &Located<Value>,
    n: usize,
    owner: (&str, &str),
    report: &mut Report,
) -> Option<Record> {
    let section = "must_haves.key_links";
    let fields = members(
        doc,
        item,
        section,
        |key, value| match key {
            "from" | "to" | "via" | "pattern" => Some(text(value)),
            _ => None,
        },
        report,
    )?;
    let end = |k: &str| fields.get(k).and_then(Json::as_str).unwrap_or("?");
    let title = format!("{} -> {}", end("from"), end("to"));
    Some(scoped(
        doc,
        "key_link",
        format!("KL{n}"),
        title,
        owner,
        fields,
        item.line,
    ))
}
