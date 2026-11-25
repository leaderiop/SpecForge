//! `NN-MM-SUMMARY.md` and a quick summary to a `summary` and its `deliverable`s (13-RESEARCH Part B, the
//! `NN-MM-SUMMARY.md` and `quick/` tables; the rows are `table::sections::{SUMMARY, SUMMARY_SHARED, QUICK,
//! QUICK_SUMMARY}`): the file stem gives the id, scoped by its plan; the frontmatter and the bold one-liner the fields;
//! each `coverage` item one `deliverable` (its checks' `status` is computed, never written). `started` and `completed`
//! are harness timestamps and never written (ADR 0391 D7). A summary whose plan file was not served is not imported.

use std::collections::BTreeMap;

use serde_json::{Map, Value as Json, json};

use super::Context;
use super::plan::{self, Doc, Scope, defined, list};
use crate::frontmatter::{self, Located, Value};
use crate::ids;
use crate::records::{Record, Records};
use crate::report::{Entry, Outcome, Report};
use crate::sections;

/// The fate table's name of a phase summary.
pub const SM: &str = "NN-MM-SUMMARY.md";

/// A summary's `status:` values.
const STATUS: &[&str] = &["complete", "halted"];

/// A deliverable check's `kind` values.
const CHECK_KINDS: &[&str] = &[
    "unit",
    "integration",
    "e2e",
    "automated_ui",
    "manual_procedural",
    "other",
];

/// Reads the phase summary `text`, served at `path` (`rel` under `.planning/`): `phases/<NN-slug>/<NN-MM>-SUMMARY.md`.
pub fn read_phase(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let doc = Doc {
        path,
        format: SM,
        prefix: "",
    };
    let Some((dir, id)) = plan::in_phase(rel, "-SUMMARY.md").filter(|(_, id)| ids::plan(id)) else {
        report.push(doc.unmapped(
            1,
            "file name",
            "the file name gives no plan id `NN-MM`: not imported".to_string(),
        ));
        return;
    };
    if !has_plan(cx, &format!("phases/{dir}/{id}-PLAN.md"), &doc, report) {
        return;
    }
    if let Some(scope) = plan::phase_scope(&doc, dir, id, cx, report) {
        read(&doc, text, id, &scope, cx, records, report);
    }
}

/// Whether the plan file `rel` was served; reports the summary when it was not.
pub fn has_plan(cx: &Context<'_>, rel: &str, doc: &Doc<'_>, report: &mut Report) -> bool {
    let found = cx.served.contains(&rel);
    if !found {
        report.push(doc.unmapped(
            1,
            "file name",
            format!("no plan for this summary (.planning/{rel} was not served): not imported"),
        ));
    }
    found
}

/// Reads the summary `text` of the plan `plan_id` under `scope`.
pub fn read(
    doc: &Doc<'_>,
    text: &str,
    plan_id: &str,
    scope: &Scope,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let plan = scope.plan_path(plan_id);
    let id = format!("{plan}/{plan_id}-SUMMARY");
    let file = scope.file();
    let mut fields: BTreeMap<String, Json> = BTreeMap::new();
    fields.insert("plan".to_string(), json!(plan));
    let mut links = Vec::new();
    let (front, start) = plan::front(doc, text, report);
    let mut coverage = None;
    for (key, v) in &front {
        let line = v.line;
        let text = v.value.as_str().map(str::trim).filter(|t| !t.is_empty());
        match (key.as_str(), scope) {
            ("phase" | "plan", Scope::Phase { number, .. }) => {
                report.push(plan::names_file(doc, (key, v), plan_id, number))
            }
            ("quick_id" | "slug", Scope::Quick) => {
                report.push(doc.row("quick_id, slug", line, key))
            }
            ("status", _) => {
                match text
                    .map(str::to_ascii_lowercase)
                    .filter(|s| STATUS.contains(&s.as_str()))
                {
                    Some(status) => {
                        fields.insert("status".to_string(), json!(status));
                    }
                    None => report.push(doc.unmapped(
                        line,
                        key,
                        "a status none of complete, halted: not imported".to_string(),
                    )),
                }
            }
            ("subsystem", _) => {
                if let Some(t) = text {
                    fields.insert("subsystem".to_string(), json!(t));
                }
            }
            ("tags" | "provides" | "affects", _) => texts(doc, (key, v), key, &mut fields, report),
            ("tech-stack", _) => tech_stack(doc, v, &mut fields, report),
            ("requirements-completed", _) => match list(&v.value) {
                Some(ids) => {
                    let kept = defined(doc, &ids, line, key, records, report);
                    links.extend(kept.iter().map(|r| ("requirements_completed", r.clone())));
                    fields.insert("requirements_completed".to_string(), json!(kept));
                }
                None => report.push(doc.unmapped(
                    line,
                    key,
                    format!("`{key}` is not a list of text: not imported"),
                )),
            },
            ("requires", _) => {
                let blocks = requires(doc, v, cx, &mut links, report);
                if !blocks.is_empty() {
                    fields.insert("requires".to_string(), json!(blocks));
                }
            }
            ("coverage", _) => coverage = Some(v),
            _ => report.push(doc.row(key, line, key)),
        }
    }
    if *scope == Scope::Quick {
        for (field, value, key) in [
            ("subsystem", json!("quick"), "subsystem"),
            (
                "requirements_completed",
                json!([]),
                "requirements-completed",
            ),
        ] {
            if !fields.contains_key(field) {
                fields.insert(field.to_string(), value);
                report.push(doc.row(key, 1, key));
            }
        }
    }
    let body = frontmatter::body(text, start);
    body_sections(doc, &body, cx, &mut fields, report);
    let title = records
        .get("plan", &plan)
        .map_or_else(|| plan_id.to_string(), |p| p.title.clone());
    records.add(
        Record {
            kind: "summary",
            id: id.clone(),
            title: format!("{title}: summary"),
            file: file.clone(),
            fields,
            refs: vec![plan.clone()],
            links,
            source: doc.at(1),
        },
        report,
    );
    if let Some(v) = coverage {
        deliverables(doc, v, (&id, &file), records, report);
    }
}

/// `tech-stack:`: `added` and `patterns`, each a list.
fn tech_stack(
    doc: &Doc<'_>,
    v: &Located<Value>,
    fields: &mut BTreeMap<String, Json>,
    report: &mut Report,
) {
    let Value::Map(members) = &v.value else {
        report.push(doc.unmapped(
            v.line,
            "tech-stack",
            "tech-stack is not a map: not imported".to_string(),
        ));
        return;
    };
    for (member, value) in members {
        let key = format!("tech-stack.{member}");
        let field = match member.as_str() {
            "added" => "tech_added",
            "patterns" => "tech_patterns",
            _ => {
                report.push(doc.row(&key, value.line, &key));
                continue;
            }
        };
        texts(doc, (&key, value), field, fields, report);
    }
}

/// The list `v` of the key `key` into `field`, when it holds an item; reported when it is no list of text.
fn texts(
    doc: &Doc<'_>,
    (key, v): (&str, &Located<Value>),
    field: &str,
    fields: &mut BTreeMap<String, Json>,
    report: &mut Report,
) {
    match list(&v.value) {
        Some(items) if !items.is_empty() => {
            fields.insert(field.to_string(), json!(items));
        }
        Some(_) => {}
        None => report.push(doc.unmapped(
            v.line,
            key,
            format!("`{key}` is not a list of text: not imported"),
        )),
    }
}

/// `requires:` in its block form (`- phase: 01-core` with `provides:`), each block's phase as its path; the flow list of
/// plan ids names no `provides` and is reported.
fn requires(
    doc: &Doc<'_>,
    v: &Located<Value>,
    cx: &Context<'_>,
    links: &mut Vec<(&'static str, String)>,
    report: &mut Report,
) -> Vec<Json> {
    let items = match &v.value {
        Value::List(items) => items,
        Value::Str(s) if s.trim().is_empty() => return Vec::new(),
        _ => {
            report.push(doc.unmapped(
                v.line,
                "requires",
                "requires is not a list: not imported".to_string(),
            ));
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for item in items {
        let Value::Map(members) = &item.value else {
            report.push(doc.row("requires: [plan ids]", item.line, "requires"));
            continue;
        };
        for (member, value) in members {
            if !matches!(member.as_str(), "phase" | "provides") {
                report.push(doc.unmapped(
                    value.line,
                    "requires",
                    format!(
                        "a requires member `{member}` no row of the table names: no field holds it"
                    ),
                ));
            }
        }
        let get = |k: &str| {
            item.value
                .get(k)
                .and_then(|v| v.value.as_str())
                .map(str::trim)
        };
        let phase = get("phase").and_then(|p| ids::padded(p.split_once('-').map_or(p, |(n, _)| n)));
        match (
            phase.and_then(|p| cx.roadmap.phases.get(&p)),
            get("provides"),
        ) {
            (Some(path), Some(provides)) if !provides.is_empty() => {
                links.push(("requires", path.clone()));
                out.push(json!({"phase": path, "provides": provides}));
            }
            _ => report.push(
                doc.unmapped(
                    item.line,
                    "requires",
                    "a requires block without a ROADMAP.md phase or a provides: not imported"
                        .to_string(),
                ),
            ),
        }
    }
    out
}

/// The body's sections: the H1 and its bold one-liner, the table's sections, `## Threat Flags` with `@gsd/security`.
fn body_sections(
    doc: &Doc<'_>,
    body: &str,
    cx: &Context<'_>,
    fields: &mut BTreeMap<String, Json>,
    report: &mut Report,
) {
    let lines: Vec<&str> = body.split('\n').collect();
    let fenced = sections::fenced(body);
    let mut h1 = false;
    // Whether a `##` section, reported with its sub-sections, holds the lines here.
    let mut under = false;
    for s in sections::sections(body) {
        let line = s.line + 1;
        if s.level <= 2 {
            under = s.level == 2;
        }
        match (s.level, s.title.as_str()) {
            (1, _) if !h1 => {
                h1 = true;
                report.push(doc.row("# title", line, &s.title));
                let bold = s
                    .body
                    .clone()
                    .filter(|&n| !fenced[n])
                    .map(|n| lines[n].trim())
                    .find(|l| !l.is_empty())
                    .and_then(|l| l.strip_prefix("**")?.strip_suffix("**"))
                    .map(str::trim)
                    .filter(|l| !l.is_empty());
                match bold {
                    Some(one_liner) => {
                        fields.insert("one_liner".to_string(), json!(one_liner));
                    }
                    None => report.push(doc.unmapped(line, &s.title, "no bold one-liner under the title: not imported".to_string())),
                }
            }
            (2, "Threat Flags") if cx.manifest.security => {
                report.push(super::mapped(&doc.at(line), &s.title, "summary.threat_flags"));
                let flags: Vec<String> = gsd_kit::markdown::tables(body)
                    .into_iter()
                    .filter(|t| t.heading == "Threat Flags")
                    .flat_map(|t| t.rows)
                    .map(|cells| cells.iter().map(|c| c.trim()).collect::<Vec<_>>().join(" | "))
                    .collect();
                if !flags.is_empty() {
                    fields.insert("threat_flags".to_string(), json!(flags));
                }
            }
            (2, "Threat Flags") => report.push(Entry::at(
                &doc.at(line),
                &s.title,
                Outcome::NeedsExtension,
                "summary.threat_flags follows @gsd/security's threat model, which the project does not load".to_string(),
            )),
            (2, title @ ("Task Commits" | "Performance" | "Accomplishments" | "Issues Encountered"
            | "Next Phase Readiness" | "Deviations from Plan")) => {
                report.push(doc.row(&format!("## {title}"), line, title));
            }
            // A sub-section of a `##` section is its section's; one under the title alone is no row's.
            (3.., _) if under => {}
            (_, title) => report.push(doc.unmapped(
                line,
                title,
                format!("a section no row of the {} table names: no field holds it", doc.format),
            )),
        }
    }
}

/// Each `coverage` item of `v` as a `deliverable` of the summary `summary`, written to `file`.
fn deliverables(
    doc: &Doc<'_>,
    v: &Located<Value>,
    (summary, file): (&str, &str),
    records: &mut Records,
    report: &mut Report,
) {
    let Value::List(items) = &v.value else {
        report.push(doc.unmapped(
            v.line,
            "coverage",
            "coverage is not a list: not imported".to_string(),
        ));
        return;
    };
    for item in items {
        let Value::Map(members) = &item.value else {
            report.push(doc.unmapped(
                item.line,
                "coverage",
                "a coverage item that is not a map: not imported".to_string(),
            ));
            continue;
        };
        let mut fields: BTreeMap<String, Json> = BTreeMap::new();
        fields.insert("summary".to_string(), json!(summary));
        let mut links = Vec::new();
        let mut id = None;
        for (key, value) in members {
            let text = value
                .value
                .as_str()
                .map(str::trim)
                .filter(|t| !t.is_empty());
            let bad = |why: &str| {
                doc.unmapped(
                    value.line,
                    "coverage",
                    format!("`{key}` {why}: not imported"),
                )
            };
            match key.as_str() {
                "id" => match text.filter(|t| ids::name(t)) {
                    Some(t) => id = Some(t.to_string()),
                    None => report.push(bad("is not an id a deliverable takes")),
                },
                "description" | "rationale" => match text {
                    Some(t) => {
                        fields.insert(key.clone(), json!(t));
                    }
                    None => report.push(bad("is not one text")),
                },
                "requirement" if matches!(value.value, Value::List(_) | Value::Map(_)) => {
                    report.push(bad("is not one requirement id"))
                }
                "requirement" => {
                    let kept = defined(
                        doc,
                        &text.map(str::to_string).into_iter().collect::<Vec<_>>(),
                        value.line,
                        key,
                        records,
                        report,
                    );
                    if let Some(r) = kept.into_iter().next() {
                        links.push(("requirement", r.clone()));
                        fields.insert(key.clone(), json!(r));
                    }
                }
                "human_judgment" => match text {
                    Some("true") => {
                        fields.insert(key.clone(), json!(true));
                    }
                    Some("false") => {
                        fields.insert(key.clone(), json!(false));
                    }
                    _ => report.push(bad("is neither true nor false")),
                },
                "verification" => {
                    let checks = checks(doc, value, report);
                    if !checks.is_empty() {
                        fields.insert("checks".to_string(), json!(checks));
                    }
                }
                _ => report.push(doc.unmapped(
                    value.line,
                    "coverage",
                    format!("a member `{key}` no row of the table names: no field holds it"),
                )),
            }
        }
        let Some(id) = id else {
            report.push(doc.unmapped(
                item.line,
                "coverage",
                "a coverage item with no id: not imported".to_string(),
            ));
            continue;
        };
        let title = fields
            .get("description")
            .and_then(Json::as_str)
            .map_or_else(|| id.clone(), str::to_string);
        records.add(
            Record {
                kind: "deliverable",
                id: format!("{summary}/{id}"),
                title,
                file: file.to_string(),
                fields,
                refs: vec![summary.to_string()],
                links,
                source: doc.at(item.line),
            },
            report,
        );
    }
}

/// A coverage item's `verification` list as `checks` blocks `{kind, ref}`; each `status` is reported computed, and any
/// other member, a check that is no map and a value that is no list are reported.
fn checks(doc: &Doc<'_>, v: &Located<Value>, report: &mut Report) -> Vec<Json> {
    let items = match &v.value {
        Value::List(items) => items.as_slice(),
        Value::Str(s) if s.trim().is_empty() => &[],
        _ => {
            report.push(doc.unmapped(
                v.line,
                "coverage",
                "`verification` is not a list: not imported".to_string(),
            ));
            &[]
        }
    };
    let mut out = Vec::new();
    for item in items {
        let Value::Map(members) = &item.value else {
            report.push(doc.unmapped(
                item.line,
                "coverage",
                "a check that is not a map: not imported".to_string(),
            ));
            continue;
        };
        let get = |k: &str| item.value.get(k);
        for (member, value) in members {
            match member.as_str() {
                "kind" | "ref" => {}
                "status" => {
                    report.push(doc.row("coverage verification status", value.line, "coverage"))
                }
                _ => report.push(doc.unmapped(
                    value.line,
                    "coverage",
                    format!(
                        "a check member `{member}` no row of the table names: no field holds it"
                    ),
                )),
            }
        }
        let text = |k: &str| get(k).and_then(|v| v.value.as_str()).map(str::trim);
        match (
            text("kind").filter(|k| CHECK_KINDS.contains(k)),
            text("ref").filter(|r| !r.is_empty()),
        ) {
            (Some(kind), Some(reference)) => {
                let mut block = Map::new();
                block.insert("kind".to_string(), json!(kind));
                block.insert("ref".to_string(), json!(reference));
                out.push(Json::Object(block));
            }
            _ => report.push(doc.unmapped(
                item.line,
                "coverage",
                format!(
                    "a check without a kind of {} or a ref: not imported",
                    CHECK_KINDS.join(", ")
                ),
            )),
        }
    }
    out
}
