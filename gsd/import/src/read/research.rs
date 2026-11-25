//! `NN-RESEARCH.md` to a `research` and its items (13-RESEARCH Part B, the RESEARCH rows; the rows are
//! `table::sections::RESEARCH`): the file stem is its id; the primary recommendation a `finding F1`; the Standard
//! Stack rows `stack_choice SC<n>`; each `### Pattern N:` and Anti-Patterns bullet a `pattern P<n>`; the Don't
//! Hand-Roll rows `dont_hand_roll DH<n>`; each `### Pitfall N:` a `pitfall PF<n>`; each Open Questions item an
//! `open_question OQ<n>`; each Sources bullet a `source SRC<n>`, all scoped by the research. A decision the research
//! names (`user_constraints`, `resolved_by`) is one of the phase's CONTEXT.md, read before it; another is left out and
//! reported. The package-legitimacy table is reported and its `legitimacy` (a verdict) never written.

use std::collections::BTreeMap;

use gsd_kit::markdown::{self, Table};
use serde_json::{Value, json};

use super::context::{Text, discretion};
use super::{Context, PhaseFile, blocks_in, is_date, is_footer, labelled, phase_file, prose};
use crate::ids;
use crate::records::{Record, Records};
use crate::report::Report;
use crate::sections::{self, Section};

/// The fate table's name of a phase research.
pub const RS: &str = "NN-RESEARCH.md";

const UNKNOWN: &str = "a section no row of the NN-RESEARCH.md table names: no field holds it";
const CONFIDENCE: &[&str] = &["high", "medium", "low"];
const RECOMMENDATION: &str = "Primary recommendation:";

const CONSTRAINTS: &str = "User Constraints";
const STACK: &str = "Standard Stack";
const PATTERNS: &str = "Architecture Patterns";
const PITFALLS: &str = "Common Pitfalls";
const SOURCES: &str = "Sources";

/// What one research file gives its items: its phase file, its path, its confidence, and each id prefix's last number.
struct Items<'f, 'a> {
    f: &'f PhaseFile<'a>,
    research: String,
    confidence: Option<String>,
    numbers: BTreeMap<&'static str, usize>,
}

impl Items<'_, '_> {
    /// The record of the next item `kind <prefix><n>` of the research, in document order from 1.
    fn next(
        &mut self,
        (kind, prefix): (&'static str, &'static str),
        title: &str,
        fields: BTreeMap<String, Value>,
        line: usize,
    ) -> Record {
        let n = self.numbers.entry(prefix).or_default();
        *n += 1;
        let id = format!("{prefix}{n}");
        self.f
            .within(kind, ("research", &self.research), &id, title, fields, line)
    }

    /// Adds the next item `kind <prefix><n>`.
    fn add(
        &mut self,
        kind: (&'static str, &'static str),
        title: &str,
        fields: BTreeMap<String, Value>,
        line: usize,
        records: &mut Records,
        report: &mut Report,
    ) {
        let record = self.next(kind, title, fields, line);
        records.add(record, report);
    }
}

const FINDING: (&str, &str) = ("finding", "F");
const STACK_CHOICE: (&str, &str) = ("stack_choice", "SC");
const PATTERN: (&str, &str) = ("pattern", "P");
const DONT: (&str, &str) = ("dont_hand_roll", "DH");
const PITFALL: (&str, &str) = ("pitfall", "PF");
const QUESTION: (&str, &str) = ("open_question", "OQ");
const SOURCE: (&str, &str) = ("source", "SRC");

/// Reads the research `text`, served at `path` (`rel` under `.planning/`): `phases/<NN-slug>/<NN>-RESEARCH.md`.
pub fn read(
    path: &str,
    rel: &str,
    text: &str,
    cx: &Context<'_>,
    records: &mut Records,
    report: &mut Report,
) {
    let Some(f) = phase_file(path, rel, RS, cx, report) else {
        return;
    };
    let t = Text::new(text);
    let all = sections::sections(text);
    let tables = markdown::tables(text);
    let mut fields: BTreeMap<String, Value> = BTreeMap::new();
    let mut links = Vec::new();
    // The header: the lines before the first `##`.
    let first = all
        .iter()
        .find(|s| s.level == 2)
        .map_or(t.lines.len(), |s| s.line);
    for (n, line) in t.raw() {
        let value = |label: &str| line.strip_prefix(label).map(str::trim);
        if n < first
            && let Some(v) = value("**Researched:**")
        {
            if is_date(v) {
                fields.insert("researched".to_string(), json!(v));
            } else {
                report.push(f.unmapped(
                    n,
                    "Researched",
                    "**Researched:** is not a YYYY-MM-DD date: left out".to_string(),
                ));
            }
        } else if n < first
            && let Some(v) = value("**Domain:**").filter(|v| !v.is_empty())
        {
            fields.insert("domain".to_string(), json!(v));
        } else if n < first
            && let Some(v) = value("**Confidence:**")
        {
            let word = v.to_lowercase();
            if CONFIDENCE.contains(&word.as_str()) {
                fields.insert("confidence".to_string(), json!(word));
            } else {
                report.push(f.unmapped(
                    n,
                    "Confidence",
                    format!("`{v}` is none of high, medium, low: left out"),
                ));
            }
        } else if is_footer(line) {
            report.push(f.row("footer", n, "footer"));
        }
    }
    let mut items = Items {
        f: &f,
        research: format!("{}/{}", f.phase, f.stem),
        confidence: fields
            .get("confidence")
            .and_then(Value::as_str)
            .map(str::to_string),
        numbers: BTreeMap::new(),
    };
    let mut title = None;
    let mut parent = String::new();
    let mut covered = 0..0;
    for s in &all {
        if covered.contains(&s.line) {
            continue;
        }
        if s.level <= 2 {
            parent = s.title.clone();
        }
        let in_parent = |name: &str| parent.starts_with(name);
        let (level, title_) = (s.level, s.title.as_str());
        if level == 1 {
            if title.is_none() {
                title = Some(s.title.clone());
            } else {
                report.push(f.unmapped(s.line, title_, UNKNOWN.to_string()));
            }
        } else if level == 2
            && [CONSTRAINTS, STACK, PATTERNS, PITFALLS, SOURCES]
                .iter()
                .any(|c| title_.starts_with(c))
        {
            report.push(f.mapped(s.line, title_, "each ### section under it"));
        } else if level == 3 && in_parent(CONSTRAINTS) && title_ == "Locked Decisions" {
            report.push(f.mapped(s.line, title_, "research.user_constraints"));
            let named = locked(&f, &t, s, records, report);
            if !named.is_empty() {
                links.extend(named.iter().map(|d| ("user_constraints", d.clone())));
                fields.insert("user_constraints".to_string(), json!(named));
            }
        } else if level == 3 && in_parent(CONSTRAINTS) && discretion(title_) {
            report.push(f.row("<user_constraints> ### Claude's Discretion", s.line, title_));
        } else if level == 3 && in_parent(CONSTRAINTS) && title_.starts_with("Deferred Ideas") {
            report.push(f.row("<user_constraints> ### Deferred Ideas", s.line, title_));
        } else if level == 2 && title_ == "Architectural Responsibility Map" {
            report.push(f.mapped(s.line, title_, "research.responsibility_map"));
            let map = responsibility_map(&f, s, &tables, report);
            if !map.is_empty() {
                fields.insert("responsibility_map".to_string(), json!(map));
            }
        } else if level == 2 && title_ == "Summary" {
            report.push(f.mapped(s.line, title_, "research.summary and its findings"));
            summary(&mut items, &t, s, &mut fields, records, report);
        } else if level == 3
            && in_parent(STACK)
            && let Some(tier) = tier_of(title_)
        {
            report.push(f.mapped(s.line, title_, "each row a stack_choice"));
            stack(&mut items, s, tier, &tables, records, report);
        } else if level == 3 && in_parent(PATTERNS) && numbered(title_, "Pattern").is_some() {
            report.push(f.mapped(s.line, title_, "a pattern"));
            pattern(&mut items, &t, s, text, records, report);
        } else if level == 3 && in_parent(PATTERNS) && title_ == "Anti-Patterns to Avoid" {
            report.push(f.mapped(s.line, title_, "each bullet an anti-pattern"));
            anti_patterns(&mut items, &t, s, records, report);
        } else if level == 2 && matches!(title_, "Don't Hand-Roll" | "Don’t Hand-Roll") {
            report.push(f.mapped(s.line, title_, "each row a dont_hand_roll"));
            dont_hand_roll(&mut items, s, &tables, records, report);
        } else if level == 3
            && in_parent(PITFALLS)
            && let Some(name) = numbered(title_, "Pitfall")
        {
            report.push(f.mapped(s.line, title_, "a pitfall"));
            pitfall(&mut items, &t, s, name, records, report);
        } else if level == 2 && title_.starts_with("Open Questions") {
            covered = s.body.clone();
            report.push(f.mapped(s.line, title_, "each question an open_question"));
            questions(&mut items, &t, s, records, report);
        } else if level == 3
            && in_parent(SOURCES)
            && let Some(tier) = source_tier(title_)
        {
            report.push(f.mapped(s.line, title_, "each bullet a source"));
            sources(&mut items, &t, s, tier, records, report);
        } else if level == 2 && title_.starts_with("Package Legitimacy") {
            covered = s.body.clone();
            report.push(f.row("package legitimacy", s.line, title_));
        } else {
            covered = s.body.clone();
            report.push(f.unmapped(s.line, title_, UNKNOWN.to_string()));
        }
    }
    let mut record = f.within(
        "research",
        ("phase", &f.phase),
        &f.stem,
        title.as_deref().unwrap_or(&f.stem),
        fields,
        0,
    );
    record.links = links;
    records.add(record, report);
}

/// `Pattern 1: Write then rename` as `Write then rename` (for `word` `Pattern`).
fn numbered<'t>(title: &'t str, word: &str) -> Option<&'t str> {
    let (n, name) = title.strip_prefix(word)?.trim_start().split_once(':')?;
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && !name.trim().is_empty())
        .then(|| name.trim())
}

/// A Standard Stack `###` heading's tier.
fn tier_of(title: &str) -> Option<&'static str> {
    match title {
        "Core" => Some("core"),
        "Supporting" => Some("supporting"),
        "Alternatives Considered" => Some("alternative"),
        _ => None,
    }
}

/// A Sources `###` heading's tier (`Primary (HIGH confidence)`).
fn source_tier(title: &str) -> Option<&'static str> {
    ["primary", "secondary", "tertiary"]
        .into_iter()
        .find(|t| title.to_lowercase().starts_with(t))
}

/// The phase decision of each `D-NN` a `### Locked Decisions` bullet starts with; any other is reported, left out.
fn locked(
    f: &PhaseFile<'_>,
    t: &Text<'_>,
    s: &Section,
    records: &Records,
    report: &mut Report,
) -> Vec<String> {
    let mut named = Vec::new();
    for (line, text) in t.bullets(&s.body) {
        let id = text.split_once(':').map_or(text, |(id, _)| id).trim();
        match decision(f, id, records) {
            Some(path) => named.push(path),
            None => report.push(f.unmapped(
                line,
                &s.title,
                format!("`{id}` names no decision of the phase's CONTEXT.md: left out"),
            )),
        }
    }
    named
}

/// The path of the phase's decision `id`, when its CONTEXT.md gave it.
fn decision(f: &PhaseFile<'_>, id: &str, records: &Records) -> Option<String> {
    let path = format!("{}/{id}", f.phase);
    (ids::decision(id) && records.contains("decision", &path)).then_some(path)
}

/// The first table of `s`, if any.
fn table_in<'t>(s: &Section, tables: &'t [Table]) -> Option<&'t Table> {
    tables.iter().find(|t| s.body.contains(&t.line))
}

/// The index of the first of `names` among `table`'s columns, ignoring case and apostrophe style.
fn column(table: &Table, names: &[&str]) -> Option<usize> {
    let fold = |s: &str| s.to_lowercase().replace('’', "'");
    table
        .header
        .iter()
        .position(|h| names.iter().any(|n| fold(h) == fold(n)))
}

/// The trimmed cell `c` of `row`, none when empty.
fn cell(row: &[String], c: Option<usize>) -> Option<&str> {
    c.and_then(|c| row.get(c))
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
}

/// Reports, with the row `key`, each column of `table` no member reads (any index not in `read`).
fn other_columns(
    f: &PhaseFile<'_>,
    table: &Table,
    read: &[Option<usize>],
    key: &str,
    report: &mut Report,
) {
    for (i, header) in table.header.iter().enumerate() {
        if !read.contains(&Some(i)) {
            report.push(f.row(key, table.line, header));
        }
    }
}

/// The `{capability, tier}` rows of the responsibility map's table.
fn responsibility_map(
    f: &PhaseFile<'_>,
    s: &Section,
    tables: &[Table],
    report: &mut Report,
) -> Vec<Value> {
    let Some(table) = table_in(s, tables) else {
        return Vec::new();
    };
    let (capability, tier) = (
        column(table, &["Capability"]),
        column(table, &["Primary Tier", "Tier"]),
    );
    other_columns(
        f,
        table,
        &[capability, tier],
        "| Secondary Tier | Rationale |",
        report,
    );
    table
        .rows
        .iter()
        .filter_map(|row| {
            let mut block = json!({ "capability": cell(row, capability)? });
            if let Some(tier) = cell(row, tier) {
                block["tier"] = json!(tier);
            }
            Some(block)
        })
        .collect()
}

/// `## Summary`: its paragraphs the research's `summary`, its `**Primary recommendation:**` the finding `F1`.
fn summary(
    items: &mut Items<'_, '_>,
    t: &Text<'_>,
    s: &Section,
    fields: &mut BTreeMap<String, Value>,
    records: &mut Records,
    report: &mut Report,
) {
    let (recommended, rest): (Vec<_>, Vec<_>) = blocks_in(&t.blocks, &s.body)
        .partition(|b| b.text.trim_start().starts_with(RECOMMENDATION));
    let paragraphs: Vec<_> = rest.into_iter().cloned().collect();
    if let Some(text) = prose(&paragraphs, &s.body) {
        fields.insert("summary".to_string(), json!(text));
    }
    for b in recommended {
        let text = b.text.trim_start()[RECOMMENDATION.len()..].trim();
        let mut finding = BTreeMap::from([("text".to_string(), json!(text))]);
        if let Some(confidence) = &items.confidence {
            finding.insert("confidence".to_string(), json!(confidence));
        }
        items.add(FINDING, text, finding, b.lines[0], records, report);
    }
}

/// One `stack_choice SC<n>` per row of the Standard Stack table of `s`, its tier the heading's.
fn stack(
    items: &mut Items<'_, '_>,
    s: &Section,
    tier: &str,
    tables: &[Table],
    records: &mut Records,
    report: &mut Report,
) {
    let Some(table) = table_in(s, tables) else {
        return;
    };
    let name = column(table, &["Library", "Could Use"]);
    let version = column(table, &["Version"]);
    let purpose = column(table, &["Purpose", "Tradeoff"]);
    other_columns(
        items.f,
        table,
        &[name, version, purpose],
        "| Why Standard |",
        report,
    );
    for (i, row) in table.rows.iter().enumerate() {
        let line = table.line + 2 + i;
        let Some(n) = cell(row, name) else {
            report.push(items.f.unmapped(
                line,
                &s.title,
                "a stack row with no name: not imported".to_string(),
            ));
            continue;
        };
        let mut fields = BTreeMap::from([
            ("name".to_string(), json!(n)),
            ("tier".to_string(), json!(tier)),
        ]);
        for (field, c) in [("version", version), ("purpose", purpose)] {
            if let Some(v) = cell(row, c) {
                fields.insert(field.to_string(), json!(v));
            }
        }
        items.add(STACK_CHOICE, n, fields, line, records, report);
    }
}

/// `### Pattern N: Name` as `pattern P<n>`: its `**What:**`, `**When to use:**` and `**Example:**` (its text, else the
/// fence under it).
fn pattern(
    items: &mut Items<'_, '_>,
    t: &Text<'_>,
    s: &Section,
    text: &str,
    records: &mut Records,
    report: &mut Report,
) {
    let name = numbered(&s.title, "Pattern").unwrap_or(&s.title);
    let mut fields = BTreeMap::from([("name".to_string(), json!(name))]);
    let label = |labels: &[&str]| labelled(&t.lines, &t.fenced, &s.body, labels);
    for (field, labels) in [("what", "**What:**"), ("when", "**When to use:**")] {
        if let Some((_, v)) = label(&[labels]).filter(|(_, v)| !v.is_empty()) {
            fields.insert(field.to_string(), json!(v));
        }
    }
    if let Some((line, v)) = label(&["**Example:**"]) {
        let fence = markdown::fences(text)
            .into_iter()
            .find(|f| f.open > line && s.body.contains(&f.open));
        let example = match (v, fence) {
            ("", Some(fence)) => fence.text.trim_end().to_string(),
            (v, _) => v.to_string(),
        };
        if !example.is_empty() {
            fields.insert("example".to_string(), json!(example));
        }
    }
    items.add(PATTERN, name, fields, s.line, records, report);
}

/// Each `- **Name:** text` bullet of Anti-Patterns to Avoid as `pattern P<n>` with `anti` true.
fn anti_patterns(
    items: &mut Items<'_, '_>,
    t: &Text<'_>,
    s: &Section,
    records: &mut Records,
    report: &mut Report,
) {
    for (line, text) in t.bullets(&s.body) {
        let Some((name, what)) = text
            .split_once(':')
            .filter(|(n, w)| !n.trim().is_empty() && !w.trim().is_empty())
        else {
            report.push(items.f.unmapped(
                line,
                &s.title,
                "an anti-pattern needs `**Name:** text`: not imported".to_string(),
            ));
            continue;
        };
        let name = name.trim();
        let fields = BTreeMap::from([
            ("name".to_string(), json!(name)),
            ("what".to_string(), json!(what.trim())),
            ("anti".to_string(), json!(true)),
        ]);
        items.add(PATTERN, name, fields, line, records, report);
    }
}

/// One `dont_hand_roll DH<n>` per row of the table of `s`; its Don't Build column is reported.
fn dont_hand_roll(
    items: &mut Items<'_, '_>,
    s: &Section,
    tables: &[Table],
    records: &mut Records,
    report: &mut Report,
) {
    let Some(table) = table_in(s, tables) else {
        return;
    };
    let problem = column(table, &["Problem"]);
    let instead = column(table, &["Use Instead"]);
    let why = column(table, &["Why"]);
    other_columns(
        items.f,
        table,
        &[problem, instead, why],
        "| Don't Build |",
        report,
    );
    for (i, row) in table.rows.iter().enumerate() {
        let mut fields = BTreeMap::new();
        for (field, c) in [("problem", problem), ("instead", instead), ("why", why)] {
            if let Some(v) = cell(row, c) {
                fields.insert(field.to_string(), json!(v));
            }
        }
        let title = cell(row, problem).unwrap_or_default();
        items.add(DONT, title, fields, table.line + 2 + i, records, report);
    }
}

/// `### Pitfall N: Name` as `pitfall PF<n>` with its four bold lines.
fn pitfall(
    items: &mut Items<'_, '_>,
    t: &Text<'_>,
    s: &Section,
    name: &str,
    records: &mut Records,
    report: &mut Report,
) {
    let mut fields = BTreeMap::from([("name".to_string(), json!(name))]);
    for (field, label) in [
        ("goes_wrong", "**What goes wrong:**"),
        ("why", "**Why it happens:**"),
        ("avoid", "**How to avoid:**"),
        ("warning_signs", "**Warning signs:**"),
    ] {
        if let Some((_, v)) =
            labelled(&t.lines, &t.fenced, &s.body, &[label]).filter(|(_, v)| !v.is_empty())
        {
            fields.insert(field.to_string(), json!(v));
        }
    }
    items.add(PITFALL, name, fields, s.line, records, report);
}

/// Each numbered item of Open Questions as `open_question OQ<n>`: `**Question** — RESOLVED: …` and its bullets
/// `What we know:`, `What's unclear:`, `Recommendation:`; `resolved_by` the first `D-NN` after `RESOLVED`.
fn questions(
    items: &mut Items<'_, '_>,
    t: &Text<'_>,
    s: &Section,
    records: &mut Records,
    report: &mut Report,
) {
    let mut asked: Vec<(usize, BTreeMap<String, Value>, Option<String>)> = Vec::new();
    for b in blocks_in(&t.blocks, &s.body) {
        let line = b.lines[0];
        let text = b.text.trim();
        if ordered(t.lines.get(line).copied().unwrap_or_default()) {
            let (question, rest) = text.split_once(" — ").unwrap_or((text, ""));
            let resolved = rest.split_once("RESOLVED").map(|(_, r)| r);
            let mut fields = BTreeMap::from([("question".to_string(), json!(question.trim()))]);
            let mut refs = None;
            if let Some(resolved) = resolved {
                let named = resolved
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .find(|w| ids::decision(w));
                match named.map(|id| (id, decision(items.f, id, records))) {
                    Some((_, Some(path))) => {
                        fields.insert("resolved_by".to_string(), json!(path));
                        refs = Some(path);
                    }
                    Some((id, None)) => report.push(items.f.unmapped(
                        line,
                        &s.title,
                        format!("resolved_by `{id}` names no decision of the phase's CONTEXT.md: left out"),
                    )),
                    None => {}
                }
            }
            asked.push((line, fields, refs));
        } else if let Some((_, fields, _)) = asked.last_mut() {
            let value = |labels: &[&str]| {
                labels
                    .iter()
                    .find_map(|l| text.strip_prefix(l))
                    .map(str::trim)
            };
            let found = [
                ("known", value(&["What we know:"])),
                ("unclear", value(&["What's unclear:", "What’s unclear:"])),
                ("recommendation", value(&["Recommendation:"])),
            ];
            match found.into_iter().find_map(|(k, v)| Some((k, v?))) {
                Some((field, v)) if !v.is_empty() => {
                    fields.insert(field.to_string(), json!(v));
                }
                _ => report.push(items.f.unmapped(line, &s.title, UNKNOWN_LINE.to_string())),
            }
        } else {
            report.push(items.f.unmapped(line, &s.title, UNKNOWN_LINE.to_string()));
        }
    }
    for (line, fields, resolved) in asked {
        let title = fields["question"].as_str().unwrap_or_default().to_string();
        let mut record = items.next(QUESTION, &title, fields, line);
        record.links.extend(resolved.map(|d| ("resolved_by", d)));
        records.add(record, report);
    }
}

const UNKNOWN_LINE: &str = "no member of an open question holds it";

/// Whether the raw line `line` starts an ordered list item (`1. `).
fn ordered(line: &str) -> bool {
    let t = line.trim_start();
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && t[digits..].starts_with(". ")
}

/// Each bullet `- url - note` of a Sources section as `source SRC<n>` of `tier`.
fn sources(
    items: &mut Items<'_, '_>,
    t: &Text<'_>,
    s: &Section,
    tier: &str,
    records: &mut Records,
    report: &mut Report,
) {
    for (line, text) in t.bullets(&s.body) {
        let (url, checked) = match text.split_once(" - ") {
            Some((url, checked)) => (url.trim(), Some(checked.trim())),
            None => (text, None),
        };
        let mut fields = BTreeMap::from([
            ("url".to_string(), json!(url)),
            ("tier".to_string(), json!(tier)),
        ]);
        if let Some(checked) = checked.filter(|c| !c.is_empty()) {
            fields.insert("checked".to_string(), json!(checked));
        }
        items.add(SOURCE, url, fields, line, records, report);
    }
}
