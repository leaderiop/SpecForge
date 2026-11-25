//! `NN-MM-PLAN.md` and a quick plan to a `plan` (13-RESEARCH Part B, the `NN-MM-PLAN.md` and `quick/` tables; the rows
//! are `table::sections::{PLAN, PLAN_SHARED, QUICK, QUICK_PLAN}`): the file name gives the id, the frontmatter and the
//! `<objective>`, `<context>`, `<verification>` and `<success_criteria>` elements the fields, and [`super::tasks`] the
//! tasks and must-haves, and [`super::threats`] its threat model. Every other key and element is reported with the table's reason.
//!
//! The file name wins over the frontmatter: a `phase:` or `plan:` (or a quick plan's `quick_id:`) that disagrees with it
//! is reported at its line, never used. Element bodies are cut by their literal closing tag ([`crate::xmltags`]) and
//! kept verbatim, trimmed at both ends.

use std::collections::BTreeMap;

use serde_json::{Value as Json, json};

use super::{Context, is_bullet, row, tasks, threats};
use crate::frontmatter::{self, Entries, Located, Value};
use crate::ids;
use crate::paths;
use crate::records::{Record, Records, Source};
use crate::report::{Entry, Outcome, Report};
use crate::sections;
use crate::xmltags::{self, Tagged};

/// The fate table's name of a phase plan.
pub const PL: &str = "NN-MM-PLAN.md";

/// A plan's `type:` values.
const PLAN_TYPES: &[&str] = &["execute", "tdd"];

/// The elements this reader reads or reports by name; any other element is reported as one no row names.
const ELEMENTS: &[&str] = &[
    "objective",
    "execution_context",
    "context",
    "tasks",
    "task",
    "name",
    "files",
    "read_first",
    "action",
    "verify",
    "automated",
    "fails_when",
    "done",
    "acceptance_criteria",
    "behavior",
    "verification",
    "success_criteria",
    "output",
    "threat_model",
];

/// The directory GSD keeps its own files in: a `<context>` path under it is GSD-internal.
const PLANNING: &str = ".planning/";

/// One served file of a plan's or a summary's format: where it is, and its rows in the fate table.
#[derive(Debug, Clone, Copy)]
pub struct Doc<'a> {
    /// The served path (`.planning/phases/01-core/01-01-PLAN.md`).
    pub path: &'a str,
    /// The `FATES` name of its format (`NN-MM-PLAN.md`, `quick/*`).
    pub format: &'static str,
    /// What its keys start with in the table (`PLAN ` for a quick plan).
    pub prefix: &'static str,
}

impl Doc<'_> {
    /// The 1-based `line` of the file.
    pub fn at(&self, line: usize) -> Source {
        Source {
            file: self.path.to_string(),
            line,
        }
    }

    /// The table's entry for its key `key`, at the 1-based `line`, in `section`.
    pub fn row(&self, key: &str, line: usize, section: &str) -> Entry {
        row(
            self.format,
            &format!("{}{key}", self.prefix),
            &self.at(line),
            section,
        )
    }

    /// An `unmapped` entry at the 1-based `line`.
    pub fn unmapped(&self, line: usize, section: &str, why: String) -> Entry {
        Entry::at(&self.at(line), section, Outcome::Unmapped, why)
    }
}

/// Where a plan sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Under the phase `number` (`01`), whose graph path is `path` (`v1.0/01`).
    Phase { number: String, path: String },
    /// A quick plan: no phase, in the project's scope.
    Quick,
}

impl Scope {
    /// The graph path of the plan `id`.
    pub fn plan_path(&self, id: &str) -> String {
        match self {
            Scope::Phase { path, .. } => format!("{path}/{id}"),
            Scope::Quick => id.to_string(),
        }
    }

    /// The `spec/gsd/` file of everything under the plan.
    pub fn file(&self) -> String {
        match self {
            Scope::Phase { number, .. } => paths::phase(number),
            Scope::Quick => paths::quick(),
        }
    }
}

/// Reads the phase plan `text`, served at `path` (`rel` under `.planning/`): `phases/<NN-slug>/<NN-MM>-PLAN.md`.
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
        format: PL,
        prefix: "",
    };
    let Some((dir, id)) = in_phase(rel, "-PLAN.md").filter(|(_, id)| ids::plan(id)) else {
        report.push(doc.unmapped(
            1,
            "file name",
            "the file name gives no plan id `NN-MM`: not imported".to_string(),
        ));
        return;
    };
    if let Some(scope) = phase_scope(&doc, dir, id, cx, report) {
        read(&doc, text, id, &scope, cx, records, report);
    }
}

/// The directory and the stem of `rel`, a file `phases/<dir>/<stem><suffix>`.
pub fn in_phase<'r>(rel: &'r str, suffix: &str) -> Option<(&'r str, &'r str)> {
    let (dir, name) = rel.strip_prefix("phases/")?.split_once('/')?;
    Some((dir, name.strip_suffix(suffix)?))
}

/// The phase of the plan `id` (`01-02`) in the directory `dir`: the file name's number wins over the directory's, and
/// a phase the roadmap does not hold imports nothing.
pub fn phase_scope(
    doc: &Doc<'_>,
    dir: &str,
    id: &str,
    cx: &Context<'_>,
    report: &mut Report,
) -> Option<Scope> {
    let number = id.rsplit_once('-').map_or(id, |(number, _)| number);
    let named = ids::padded(dir.split_once('-').map_or(dir, |(n, _)| n));
    if named.as_deref() != Some(number) {
        report.push(doc.unmapped(
            1,
            "file name",
            format!("the directory `{dir}` names another phase than the file name's {number}: the file name wins"),
        ));
    }
    match cx.roadmap.phases.get(number) {
        Some(path) => Some(Scope::Phase {
            number: number.to_string(),
            path: path.clone(),
        }),
        None => {
            report.push(doc.unmapped(
                1,
                "file name",
                format!("phase {number} is no phase of ROADMAP.md: not imported"),
            ));
            None
        }
    }
}

/// The frontmatter entries of `text` in the order written (none when it has no frontmatter, or one refused and
/// reported); and the index of the body's first line.
pub fn front(doc: &Doc<'_>, text: &str, report: &mut Report) -> (Entries, usize) {
    let (front, start) = frontmatter::split(text);
    match front.and_then(|f| f.map(|f| frontmatter::read(&f)).transpose()) {
        Ok(front) => (front.map(|f| f.value).unwrap_or_default(), start),
        Err(refused) => {
            report.push(doc.unmapped(
                refused.line,
                "---",
                format!("frontmatter not read: {}", refused.reason),
            ));
            (Vec::new(), start)
        }
    }
}

/// A list of texts: a list's items, or one text; none for a map or a list holding one.
pub fn list(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::Str(s) if s.trim().is_empty() => Some(Vec::new()),
        Value::Str(s) => Some(vec![s.trim().to_string()]),
        Value::List(items) => items
            .iter()
            .map(|i| i.value.as_str().map(|s| s.trim().to_string()))
            .collect(),
        Value::Map(_) => None,
    }
}

/// The `<name>` elements of `text`, whose first line is the file's 1-based line `base`, each at its line in the file;
/// each refusal (an unclosed element, broken attributes) is reported.
pub fn elements(
    doc: &Doc<'_>,
    text: &str,
    base: usize,
    name: &str,
    report: &mut Report,
) -> Vec<Tagged> {
    let (found, refused) = xmltags::tagged(text, name);
    for r in refused {
        report.push(doc.unmapped(
            base + r.line - 1,
            &format!("<{name}>"),
            format!("{}: not read", r.reason),
        ));
    }
    found
        .into_iter()
        .map(|t| Tagged {
            line: base + t.line - 1,
            ..t
        })
        .collect()
}

/// The items of an element's body: each bullet (its continuation lines joined by a line break) when it has bullets,
/// else each paragraph; every line trimmed.
pub fn items(body: &str) -> Vec<String> {
    let bulleted = body.split('\n').any(is_bullet);
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in body.split('\n') {
        let t = line.trim();
        if t.is_empty() {
            if !bulleted {
                out.extend(current.take());
            }
            continue;
        }
        if bulleted && is_bullet(line) {
            out.extend(current.take());
            current = Some(t[2..].trim().to_string());
        } else if let Some(item) = &mut current {
            item.push('\n');
            item.push_str(t);
        } else {
            current = Some(t.to_string());
        }
    }
    out.extend(current);
    out
}

/// Reads the plan `text` with the id `id` (its file name's) under `scope`.
pub fn read(
    doc: &Doc<'_>,
    text: &str,
    id: &str,
    scope: &Scope,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let plan = scope.plan_path(id);
    let file = scope.file();
    let mut fields: BTreeMap<String, Json> = BTreeMap::new();
    let mut refs: Vec<String> = Vec::new();
    let mut links = Vec::new();
    if let Scope::Phase { path, .. } = scope {
        fields.insert("phase".to_string(), json!(path));
        refs.push(path.clone());
    }
    let (front, start) = front(doc, text, report);
    let mut must_haves = None;
    for (key, v) in &front {
        let built = (&mut fields, &mut links);
        frontmatter_key(doc, (key, v), id, scope, cx, records, built, report);
        if key == "must_haves" {
            must_haves = Some(v);
        }
    }
    let body = frontmatter::body(text, start);
    let mut title = None;
    for (n, o) in elements(doc, &body, 1, "objective", report)
        .iter()
        .enumerate()
    {
        if n > 0 {
            report.push(doc.unmapped(
                o.line,
                "<objective>",
                "a second <objective>: not read".to_string(),
            ));
            continue;
        }
        title = objective(doc, o, &mut fields, report);
    }
    for element in ["execution_context", "output"] {
        for e in elements(doc, &body, 1, element, report) {
            let key = format!("<{element}>");
            report.push(doc.row(&key, e.line, &key));
        }
    }
    let mut context_refs = Vec::new();
    for c in elements(doc, &body, 1, "context", report) {
        context_refs.extend(context(doc, &c, report));
    }
    if !context_refs.is_empty() {
        fields.insert("context_refs".to_string(), json!(context_refs));
    }
    for (element, field) in [
        ("verification", "verification"),
        ("success_criteria", "success_criteria"),
    ] {
        let all: Vec<String> = elements(doc, &body, 1, element, report)
            .iter()
            .flat_map(|e| items(&e.body))
            .collect();
        if !all.is_empty() {
            fields.insert(field.to_string(), json!(all));
        }
    }
    unknown_elements(doc, &body, report);
    let title = match (scope, cx.roadmap.plan_titles.get(id)) {
        (Scope::Phase { .. }, Some(t)) => t.clone(),
        (Scope::Phase { .. }, None) => {
            let line = title.as_ref().map_or(1, |(_, line)| *line);
            report.push(Entry::at(
                &doc.at(line),
                "<objective>",
                Outcome::Filled,
                format!("ROADMAP.md's Plans: names no title for {id}: the objective's first line is its title"),
            ));
            title.map_or_else(|| id.to_string(), |(t, _)| t)
        }
        (Scope::Quick, _) => title.map_or_else(|| id.to_string(), |(t, _)| t),
    };
    records.add(
        Record {
            kind: "plan",
            id: plan.clone(),
            title,
            file: file.clone(),
            fields,
            refs,
            links,
            source: doc.at(1),
        },
        report,
    );
    tasks::tasks(doc, &body, &plan, &file, records, report);
    if let Some(v) = must_haves {
        tasks::must_haves(doc, v, &plan, &file, records, report);
    }
    let security = cx.manifest.security;
    threats::read(doc, &body, (&plan, &file), security, records, report);
}

/// One frontmatter key of the plan `id`: a field, a cross-check, or a report entry with the table's reason.
#[allow(
    clippy::too_many_arguments,
    reason = "a key reads the plan's scope, the roadmap and the requirements read before it"
)]
fn frontmatter_key(
    doc: &Doc<'_>,
    (key, v): (&str, &Located<Value>),
    id: &str,
    scope: &Scope,
    cx: &Context<'_>,
    records: &Records,
    (fields, links): (
        &mut BTreeMap<String, Json>,
        &mut Vec<(&'static str, String)>,
    ),
    report: &mut Report,
) {
    let line = v.line;
    let text = v.value.as_str().map(str::trim);
    let not_list = |report: &mut Report| {
        report.push(doc.unmapped(
            line,
            key,
            format!("`{key}` is not a list of text: not imported"),
        ))
    };
    match (key, scope) {
        ("phase" | "plan", Scope::Phase { number, .. }) => {
            report.push(names_file(doc, (key, v), id, number))
        }
        ("quick_id", Scope::Quick) if text == Some(id) => {}
        ("quick_id", Scope::Quick) => report.push(doc.unmapped(
            line,
            key,
            format!(
                "`quick_id: {}` disagrees with the file name {id}: the file name wins",
                text.unwrap_or_default()
            ),
        )),
        ("slug", Scope::Quick) => match text.filter(|t| !t.is_empty()) {
            Some(slug) => {
                fields.insert("slug".to_string(), json!(slug));
            }
            None => report.push(doc.unmapped(line, key, "an empty slug: not imported".to_string())),
        },
        ("type", _) => match text.filter(|t| PLAN_TYPES.contains(t)) {
            Some(t) => {
                fields.insert("type".to_string(), json!(t));
            }
            None => report.push(doc.unmapped(
                line,
                key,
                format!(
                    "`{}` is none of {}: not imported",
                    text.unwrap_or_default(),
                    PLAN_TYPES.join(", ")
                ),
            )),
        },
        ("wave", Scope::Phase { .. }) => {
            report.push(doc.row(key, line, key));
            if text == Some("0") {
                fields.insert("wave0".to_string(), json!(true));
            }
        }
        ("files_modified", _) => match list(&v.value) {
            Some(files) if !files.is_empty() => {
                fields.insert(key.to_string(), json!(files));
            }
            Some(_) => {}
            None => not_list(report),
        },
        ("depends_on", Scope::Phase { .. }) => {
            match list(&v.value) {
                Some(plans) => {
                    let mut kept = Vec::new();
                    for p in plans {
                        let phase = p.rsplit_once('-').map(|(n, _)| n).filter(|_| ids::plan(&p));
                        match phase.and_then(|n| cx.roadmap.phases.get(n)) {
                        Some(path) => kept.push(format!("{path}/{p}")),
                        None => report.push(doc.unmapped(
                            line,
                            key,
                            format!("depends_on names `{p}`, no plan of a ROADMAP.md phase: left out"),
                        )),
                    }
                    }
                    if !kept.is_empty() {
                        links.extend(kept.iter().map(|p| ("depends_on", p.clone())));
                        fields.insert(key.to_string(), json!(kept));
                    }
                }
                None => not_list(report),
            }
        }
        ("requirements", Scope::Phase { .. }) => match list(&v.value) {
            Some(ids) => {
                let kept = defined(doc, &ids, line, key, records, report);
                if !kept.is_empty() {
                    links.extend(kept.iter().map(|r| ("requirements", r.clone())));
                    fields.insert(key.to_string(), json!(kept));
                }
            }
            None => not_list(report),
        },
        // Read by `tasks::must_haves` once the plan's path is known.
        ("must_haves", _) => {}
        _ => report.push(doc.row(key, line, key)),
    }
}

/// A frontmatter `phase:` or `plan:` against the file name's plan `id` in the phase `number`: `redundant` when it agrees
/// (the `phase, plan` row), else the mismatch, at its line; the file name wins either way.
pub fn names_file(
    doc: &Doc<'_>,
    (key, v): (&str, &Located<Value>),
    id: &str,
    number: &str,
) -> Entry {
    let (_, own) = id.rsplit_once('-').unwrap_or((id, id));
    let written = v.value.as_str().map_or("", str::trim);
    let agrees = match key {
        "phase" => {
            ids::padded(written.split_once('-').map_or(written, |(n, _)| n)).as_deref()
                == Some(number)
        }
        _ => format!("{written:0>2}") == own,
    };
    if agrees {
        doc.row("phase, plan", v.line, key)
    } else {
        doc.unmapped(
            v.line,
            key,
            format!("`{key}: {written}` disagrees with the file name {id}: the file name wins"),
        )
    }
}

/// The requirement ids a list may keep: the ones REQUIREMENTS.md defined (read before every plan), each other one
/// reported and left out, as the roadmap leaves it out of a phase.
pub fn defined(
    doc: &Doc<'_>,
    ids: &[String],
    line: usize,
    key: &str,
    records: &Records,
    report: &mut Report,
) -> Vec<String> {
    let mut kept = Vec::new();
    for id in ids {
        if records.contains("requirement", id) {
            kept.push(id.clone());
        } else {
            report.push(doc.unmapped(
                line,
                key,
                format!("`{key}` names {id}, which REQUIREMENTS.md does not define: left out"),
            ));
        }
    }
    kept
}

/// The objective's fields: its first paragraph, `Purpose:` and `Output:`; the first line of that paragraph less its
/// final period (the title of a plan the roadmap names no title for) and that line's number.
fn objective(
    doc: &Doc<'_>,
    o: &Tagged,
    fields: &mut BTreeMap<String, Json>,
    report: &mut Report,
) -> Option<(String, usize)> {
    let mut first: Vec<&str> = Vec::new();
    let (mut first_line, mut past_first, mut extra) = (o.line, false, false);
    for (i, raw) in o.body.split('\n').enumerate() {
        let t = raw.trim();
        let line = o.line + i;
        if let Some(rest) = t.strip_prefix("Purpose:") {
            fields.insert("purpose".to_string(), json!(rest.trim()));
        } else if let Some(rest) = t.strip_prefix("Output:") {
            fields.insert("output".to_string(), json!(rest.trim()));
        } else if t.is_empty() {
            past_first |= !first.is_empty();
        } else if !past_first {
            if first.is_empty() {
                first_line = line;
            }
            first.push(t);
        } else if !extra {
            extra = true;
            report.push(doc.unmapped(
                line,
                "<objective>",
                "the objective's text past its first paragraph: no field holds it".to_string(),
            ));
        }
    }
    let head = first.first()?;
    fields.insert("objective".to_string(), json!(first.join("\n")));
    Some((
        head.strip_suffix('.').unwrap_or(head).trim().to_string(),
        first_line,
    ))
}

/// The `@` paths of a `<context>` element: a path under `.planning/` is GSD's own and reported, the others kept.
fn context(doc: &Doc<'_>, c: &Tagged, report: &mut Report) -> Vec<String> {
    let mut kept = Vec::new();
    for (i, raw) in c.body.split('\n').enumerate() {
        let line = c.line + i;
        let t = raw.trim();
        match t.strip_prefix('@').map(str::trim) {
            _ if t.is_empty() => {}
            Some(path) if path.starts_with(PLANNING) => {
                report.push(doc.row("<context> @.planning/", line, "<context>"));
            }
            Some(path) if !path.is_empty() => kept.push(path.to_string()),
            _ => report.push(doc.unmapped(
                line,
                "<context>",
                "a <context> line that names no @path: no field holds it".to_string(),
            )),
        }
    }
    kept
}

/// Reports each element outside a fence whose name this reader neither reads nor reports.
fn unknown_elements(doc: &Doc<'_>, body: &str, report: &mut Report) {
    let fenced = sections::fenced(body);
    for (n, raw) in body.split('\n').enumerate() {
        let Some(rest) = raw.trim_start().strip_prefix('<') else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            .collect();
        if !fenced[n] && !name.is_empty() && !ELEMENTS.contains(&name.as_str()) {
            let key = format!("<{name}>");
            report.push(doc.row(&key, n + 1, &key));
        }
    }
}
