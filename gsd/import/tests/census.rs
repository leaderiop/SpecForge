//! The measure of ADR 0391 D3 and D8: nothing a `.planning/` folder holds is dropped silently, and one folder gives the
//! same plan and report whatever the order its files are served in.

mod common;

use std::collections::BTreeSet;

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use common::{fact, feed, fixture, fixture_manifest, import, manifest, payload, planning_facts};
use gsd_import::command::imported;
use gsd_import::frontmatter;
use gsd_import::preflight;
use gsd_import::read::Served;
use gsd_import::table::{self, EXTRAS, FATES, Fate};

/// A path under `.planning/` for the `FATES` name `name`: each placeholder filled (`NN`, `MM` → `01`, `<version>` →
/// `1.0`, `*` → `x`), a phase file in phase 01's directory.
fn path_for(name: &str) -> String {
    let concrete = name
        .replace("NN-", "01-")
        .replace("MM-", "01-")
        .replace("<version>", "1.0")
        .replace('*', "x");
    if name.starts_with("NN") {
        format!("phases/01-core/{concrete}")
    } else {
        concrete
    }
}

/// The served facts of `.planning/` in the census tree: the taskboard's folder, the ledger's quick plan and the vault's
/// security audit (the Tier 1 files, with sources the readers map), then one placeholder for every other `FATES` name,
/// and a file no row names.
fn census_tree() -> Vec<Value> {
    let mut facts = planning_facts(&fixture("taskboard"));
    let extra = |name: &str, rel: &str| -> Vec<Value> {
        planning_facts(&fixture(name))
            .into_iter()
            .filter(|f| f["path"].as_str().is_some_and(|p| p.starts_with(rel)))
            .collect()
    };
    facts.extend(extra("ledger", ".planning/quick/"));
    facts.extend(extra("vault", ".planning/phases/01-core/01-SECURITY.md"));
    let named = |facts: &[Value], name: &str| {
        facts.iter().any(|f| {
            let rel = f["path"]
                .as_str()
                .unwrap_or_default()
                .trim_start_matches(".planning/");
            table::fate_of(rel).is_some_and(|(row, _)| row == name)
        })
    };
    for (name, _) in FATES.iter().chain(EXTRAS) {
        if !named(&facts, name) {
            let path = path_for(name);
            assert_eq!(
                table::fate_of(&path).map(|(row, _)| row),
                Some(*name),
                "the placeholder {path} must take the row {name}"
            );
            facts.push(fact(&format!(".planning/{path}"), "placeholder\n"));
        }
    }
    facts.push(fact(".planning/notes.txt", "a note\n"));
    facts
}

/// Whether `entry` is the one `emit` writes for a record it creates (`<kind> <id>` in the section `<kind>`).
fn is_record(entry: &Value) -> bool {
    let (section, reason) = (entry["section"].as_str(), entry["reason"].as_str());
    entry["outcome"] == "mapped"
        && section
            .zip(reason)
            .is_some_and(|(s, r)| r.starts_with(&format!("{s} ")))
}

/// The file level of D3: each served file of the census tree is a record's source or has one whole-file entry.
#[test]
fn every_served_file_has_a_record_or_one_file_entry() {
    let facts = census_tree();
    let out = import(
        &CommandGraph::default(),
        &feed(
            facts.clone(),
            false,
            Some(&manifest(&["@gsd/core", "@gsd/security"])),
        ),
    );
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let report = payload(&out);
    let entries = report["entries"].as_array().expect("entries");
    // Every name of the table is in the tree.
    let served: Vec<&str> = facts.iter().filter_map(|f| f["path"].as_str()).collect();
    for (name, _) in FATES.iter().chain(EXTRAS) {
        assert!(
            served
                .iter()
                .any(|p| table::fate_of(p.trim_start_matches(".planning/"))
                    .is_some_and(|(row, _)| row == *name)),
            "{name}"
        );
    }
    let mut silent = Vec::new();
    for path in &served {
        let of: Vec<&Value> = entries.iter().filter(|e| e["file"] == *path).collect();
        let records = of.iter().filter(|e| is_record(e)).count();
        let whole = of
            .iter()
            .filter(|e| e["line"].is_null() && e["section"] == "")
            .count();
        if records == 0 && whole != 1 {
            silent.push(format!("{path}: {records} records, {whole} file entries"));
        }
    }
    assert!(silent.is_empty(), "{silent:#?}");
}

/// The words of `text`: its runs of letters and digits, lower-cased, each followed by one space.
fn words(text: &str) -> String {
    let mut out = String::from(" ");
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        out.push_str(&word.to_lowercase());
        out.push(' ');
    }
    out
}

/// Every text the plan of `out` writes, as [`words`]: each create's title and each string of its fields at any depth,
/// and each list or block whole, its texts in order (a `<files>` line is a list, a constraint bullet a block).
fn written(out: &CommandOutput) -> Vec<String> {
    fn strings(value: &Value, into: &mut Vec<String>) -> String {
        let parts: Vec<String> = match value {
            Value::String(s) => vec![words(s)],
            Value::Array(items) => items.iter().map(|v| strings(v, into)).collect(),
            Value::Object(map) => map.values().map(|v| strings(v, into)).collect(),
            _ => Vec::new(),
        };
        let parts: Vec<&str> = parts
            .iter()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();
        let joined = format!(" {} ", parts.join(" "));
        into.push(joined.clone());
        joined
    }
    let mut out_texts = Vec::new();
    for c in common::creates(out) {
        out_texts.push(words(&c.title));
        strings(&c.fields, &mut out_texts);
    }
    out_texts
}

/// Each line of the served Tier 1 file `path` that nothing accounts for (ADR 0391 D3, D8): a line outside the
/// frontmatter (whose keys `read::row` holds) and every fence, holding a letter or a digit, that is not an XML tag alone.
/// A line is accounted for when:
///
/// - an entry is at it, or at the first line of the leaf block holding it (a reader reports a paragraph once);
/// - its text is written: its words (XML tags and a list marker cut), or the words after its label (`**Goal**:`,
///   `- [x] 01-01:`), run inside a text of the plan; a table row cell by cell, a cell's column reported at the header
///   line counting for the cell;
/// - it is structure: a label with no word after it (`Plans:`, `**Success Criteria** (what must be TRUE):`, `missing:
///   []`), whose text is the lines under it or nothing, or a table's header row, whose columns go with the cells;
/// - it lies in a section (level 2 or deeper) or an element whose entry is not `mapped`, up to the next heading that has
///   an entry of its own: the section is reported whole. A level-1 heading never covers what follows it.
fn unaccounted(path: &str, text: &str, entries: &[Value], written: &[String]) -> Vec<String> {
    let tag = regex::Regex::new(r#"^</?[A-Za-z][\w-]*(\s[^<>]*)?/?>$"#).expect("a pattern");
    let tags = regex::Regex::new(r#"^\s*<[A-Za-z][\w-]*(\s[^<>]*)?>|</[A-Za-z][\w-]*>\s*$"#)
        .expect("a pattern");
    let marker = regex::Regex::new(r#"^\s*([-*+]|\d+[.)])\s+(\[[ xX]\]\s+)?"#).expect("a pattern");
    let held = |text: &str| {
        let w = words(text);
        !w.trim().is_empty() && written.iter().any(|t| t.contains(w.as_str()))
    };
    let (_, start) = frontmatter::split(text);
    let body = frontmatter::body(text, start);
    let lines: Vec<&str> = body.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let fenced = gsd_import::sections::fenced(&body);
    let of: Vec<&Value> = entries.iter().filter(|e| e["file"] == path).collect();
    let at = |n: usize| -> Vec<&&Value> {
        of.iter()
            .filter(|e| e["line"].as_u64() == Some(n as u64 + 1))
            .collect()
    };
    // The first line of the leaf block holding each line.
    let mut first: Vec<usize> = (0..lines.len()).collect();
    for b in gsd_kit::markdown::inline_blocks(&body) {
        for &l in &b.lines {
            if l < first.len() {
                first[l] = first[l].min(b.lines[0]);
            }
        }
    }
    let headings: BTreeSet<usize> = gsd_kit::markdown::headings(&body)
        .iter()
        .filter(|h| h.level > 1)
        .map(|h| h.line)
        .collect();
    let h1: BTreeSet<usize> = gsd_kit::markdown::headings(&body)
        .iter()
        .filter(|h| h.level == 1)
        .map(|h| h.line)
        .collect();
    // Each table line: its header (no cells) or a row's cells.
    let tables = gsd_kit::markdown::tables(&body);
    let mut rows: std::collections::BTreeMap<
        usize,
        (&gsd_kit::markdown::Table, Option<&Vec<String>>),
    > = std::collections::BTreeMap::new();
    for t in &tables {
        rows.insert(t.line, (t, None));
        for (i, cells) in t.rows.iter().enumerate() {
            rows.insert(t.line + 2 + i, (t, Some(cells)));
        }
    }
    let mut covered = false;
    // The element a non-mapped entry covers, until its closing tag.
    let mut element: Option<String> = None;
    let mut silent = Vec::new();
    for (n, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let here = at(n);
        let reported_whole = !here.is_empty() && here.iter().all(|e| e["outcome"] != "mapped");
        if h1.contains(&n) {
            covered = false;
        } else if headings.contains(&n) && !here.is_empty() {
            covered = reported_whole;
        }
        if element.is_none() && reported_whole && !headings.contains(&n) {
            let name: String = line
                .strip_prefix('<')
                .unwrap_or_default()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                .collect();
            if !name.is_empty() && !line.contains(&format!("</{name}>")) {
                element = Some(name);
            }
        }
        let inside = element.is_some();
        if let Some(name) = &element
            && line.contains(&format!("</{name}>"))
        {
            element = None;
        }
        if fenced.get(n).copied().unwrap_or(false)
            || words(line).trim().is_empty()
            || tag.is_match(line)
            || covered
            || inside
            || !here.is_empty()
            || !at(first[n]).is_empty()
        {
            continue;
        }
        let accounted = match rows.get(&n) {
            Some((t, None)) => t.header.iter().all(|h| !h.is_empty()),
            Some((t, Some(cells))) => cells.iter().enumerate().all(|(i, cell)| {
                words(cell).trim().is_empty()
                    || held(cell)
                    || t.header.get(i).is_some_and(|h| {
                        at(t.line)
                            .iter()
                            .any(|e| e["section"].as_str() == Some(h.as_str()))
                    })
            }),
            None => {
                let plain = tags.replace_all(line, "");
                let plain = marker.replace(&plain, "");
                let label = plain.split_once(':');
                let after_label = label.map_or("", |(_, rest)| rest);
                // A label whose value holds no word (`missing: []`, `Plans:`) carries nothing to lose.
                let empty = label.is_some() && words(after_label).trim().is_empty();
                empty || held(&plain) || held(after_label)
            }
        };
        if !accounted {
            silent.push(format!("{path}:{} {line}", n + 1));
        }
    }
    silent
}

/// The silent lines of each Tier 1 file of `facts`, imported with the manifest `manifest`.
fn silent_lines(facts: &[Value], manifest: &str) -> Vec<String> {
    let out = import(
        &CommandGraph::default(),
        &feed(facts.to_vec(), false, Some(manifest)),
    );
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let report = payload(&out);
    let entries = report["entries"].as_array().expect("entries");
    let texts = written(&out);
    let mut silent = Vec::new();
    for f in facts {
        let path = f["path"].as_str().unwrap_or_default();
        let rel = path.trim_start_matches(".planning/");
        if matches!(table::fate_of(rel), Some((_, Fate::Maps))) {
            let text = f["text"].as_str().unwrap_or_default();
            silent.extend(unaccounted(path, text, entries, &texts));
        }
    }
    silent
}

/// The line level of D3 on the fixtures: every line of every Tier 1 file is accounted for.
#[test]
fn every_line_of_a_fixture_is_accounted_for() {
    let mut silent = Vec::new();
    for (name, security) in [("taskboard", false), ("ledger", false), ("vault", true)] {
        silent.extend(silent_lines(
            &planning_facts(&fixture(name)),
            &fixture_manifest(name, security),
        ));
    }
    assert!(silent.is_empty(), "lines nothing accounts for: {silent:#?}");
}

/// A `.planning/` folder holding, in each Tier 1 format, a construct a reader once skipped with no entry: a title that is
/// not `# Roadmap:`, a phase's prose line, a requirement bullet with no id, a sub-section under a prose section, a plan's
/// heading and prose outside its elements, a `###` under a summary's title, a diagnosed gap's map-form `artifacts`, the
/// intro of a context's decisions and a research's `**Installation:**` line.
fn hostile_tree() -> Vec<Value> {
    let files = [
        (
            "ROADMAP.md",
            "# Project Roadmap\n\n### Phase 1: One\n\nSome prose about the phase.\n**Goal**: g\n\
             **Success Criteria**:\n\n  1. c\n\n### Phase 2: Two\n**Goal**: h\n**Success Criteria**:\n  1. d\n",
        ),
        (
            "REQUIREMENTS.md",
            "# Requirements: X\n\n## v1 Requirements\n\n### Core\n\n- **CORE-01**: one\n- Users can share a board\n",
        ),
        (
            "PROJECT.md",
            "# Hostile\n\n## What This Is\n\nA thing.\n\n### Details\n\nMore detail.\n\n## Core Value\n\nIt works.\n\n\
             ## Context\n\nNone.\n\n## Constraints\n\n- **Budget**: none — free\n",
        ),
        (
            "phases/01-one/01-01-PLAN.md",
            "---\ntype: execute\nfiles_modified: [a.js]\n---\n\n# Plan 01-01\n\nIntro prose.\n\n<objective>\nDo it.\n\
             </objective>\n\n<tasks>\n<task type=\"auto\">\n  <name>Task 1: T</name>\n  <action>a</action>\n  <done>d</done>\n\
             </task>\n</tasks>\n\n## Notes\n\nSome notes.\n",
        ),
        (
            "phases/01-one/01-01-SUMMARY.md",
            "---\nphase: 01-one\nplan: 01\nsubsystem: core\nstatus: complete\nrequirements-completed: []\ncoverage:\n\
             \x20 - id: D1\n    description: \"x\"\n    verification: none\n---\n\n# Plan 01 Summary\n\n**It works.**\n\n\
             ### Stray\n\nstray words\n",
        ),
        (
            "phases/01-one/01-UAT.md",
            "---\nstatus: complete\nphase: 01-one\nsource: [01-01-SUMMARY.md]\nstarted: 2026-03-03T10:30:00Z\n\
             updated: 2026-03-03T10:40:00Z\n---\n\n## Tests\n\n### 1. Works\nexpected: it works\nresult: issue\n\
             reported: \"broken\"\nseverity: minor\n\n## Gaps\n\n- truth: \"It works\"\n  status: failed\n\
             \x20 reason: \"User reported: broken\"\n  severity: minor\n  test: 1\n  root_cause: \"the cause\"\n\
             \x20 artifacts:\n    - path: \"src/a.js\"\n      issue: \"the bug\"\n  missing: []\n",
        ),
        (
            "phases/01-one/01-CONTEXT.md",
            "# Phase 1: One - Context\n\n**Gathered:** 2026-03-04\n\n<domain>\n## Phase Boundary\n\nB.\n\n</domain>\n\n\
             <decisions>\n## Implementation Decisions\n\nIntro prose here.\n\n### Area\n- **D-01:** one\n</decisions>\n",
        ),
        (
            "phases/01-one/01-RESEARCH.md",
            "# Phase 1: One - Research\n\n**Researched:** 2026-03-04\n**Confidence:** HIGH\n\n## Summary\n\nS.\n\n\
             ## Standard Stack\n\n**Installation:** run npm\n\n### Core\n\n| Library | Version | Purpose |\n|---|---|---|\n\
             | a | 1 | b |\n",
        ),
    ];
    files
        .into_iter()
        .map(|(rel, text)| fact(&format!(".planning/{rel}"), text))
        .collect()
}

/// The line level of D3 on hostile input: every line of each Tier 1 file of [`hostile_tree`] is accounted for, and
/// each file is read, not refused whole.
#[test]
fn every_line_of_a_hostile_folder_is_accounted_for() {
    let facts = hostile_tree();
    let manifest = manifest(&["@gsd/core"]);
    let out = import(
        &CommandGraph::default(),
        &feed(facts.clone(), false, Some(&manifest)),
    );
    let silent = silent_lines(&facts, &manifest);
    assert!(silent.is_empty(), "lines nothing accounts for: {silent:#?}");
    let all = common::creates(&out);
    for (kind, path) in [
        ("phase", "01"),
        ("requirement", "CORE-01"),
        ("project", "hostile"),
        ("task", "v1.0/01/01-01/T1"),
        ("summary", "v1.0/01/01-01/01-01-SUMMARY"),
        ("uat_gap", "v1.0/01/01-UAT/U1/G1"),
        ("decision", "v1.0/01/D-01"),
        ("stack_choice", "v1.0/01/01-RESEARCH/SC1"),
    ] {
        common::find(&all, kind, path);
    }
}

/// The fixture `name`'s import through [`imported`], its served files in `order`.
fn imported_in(name: &str, security: bool, reversed: bool) -> (String, String, String) {
    let mut served: Vec<Served> = planning_facts(&fixture(name))
        .into_iter()
        .map(|f| Served {
            path: f["path"].as_str().unwrap_or_default().to_string(),
            text: f["text"].as_str().map(str::to_string),
            error: None,
        })
        .collect();
    if reversed {
        served.reverse();
    }
    let manifest = preflight::manifest(&[Served {
        path: "specforge.json".to_string(),
        text: Some(fixture_manifest(name, security)),
        error: None,
    }])
    .expect("a manifest");
    let plan = PlanBuilder::new(Default::default(), Default::default());
    let out = imported(served, &manifest, &CommandGraph::default(), plan).expect("imported");
    (
        serde_json::to_string(&out.plan.finish()).expect("JSON"),
        serde_json::to_string(&out.payload).expect("JSON"),
        out.text,
    )
}

#[test]
fn vault_owed_names_each_judged_entity() {
    let report = payload(&common::import_with("vault", true));
    let owed: Vec<String> = report["owed"]
        .as_array()
        .expect("owed")
        .iter()
        .map(|o| {
            format!(
                "{} {} {}",
                o["phase"].as_str().unwrap_or("-"),
                o["code"].as_str().unwrap_or_default(),
                o["entity"].as_str().unwrap_or_default()
            )
        })
        .collect();
    // The closed threats, the audit, the must-haves of the two complete plans, and the plan checks of 02-01, which has
    // no summary and so is open.
    assert_eq!(
        owed,
        [
            "01 gsd.security/W302 v1.0/01/01-01/T-01-01",
            "01 gsd.security/W302 v1.0/01/01-02/T-01-03",
            "01 gsd.security/W303 v1.0/01/01-SECURITY",
            "01 gsd/E501 v1.0/01/01-01/AR1",
            "01 gsd/E501 v1.0/01/01-01/KL1",
            "01 gsd/E501 v1.0/01/01-01/TR1",
            "01 gsd/E501 v1.0/01/01-02/AR1",
            "01 gsd/E501 v1.0/01/01-02/KL1",
            "01 gsd/E501 v1.0/01/01-02/TR1",
            "02 gsd/W311 v1.0/02/02-01",
            "02 gsd/W313 v1.0/02/02-01",
            "02 gsd/W315 v1.0/02/02-01",
            "02 gsd/W316 v1.0/02/02-01",
        ]
    );
    let (_, _, text) = imported_in("vault", true, false);
    assert!(text.contains("\n  /gsd-secure-phase 01\n    specforge query 'v1.0/01/01-01/T-01-01' --inputs gsd.security/W302 --format json\n"), "{text}");
    assert!(text.contains("\n  /gsd-verify-work 01\n"), "{text}");
    assert!(
        text.contains(
            "\n  13 verdicts owed: `specforge check` exits 1 until these verdicts are recorded\n"
        ),
        "{text}"
    );
}

#[test]
fn an_import_is_deterministic() {
    for (name, security) in [("taskboard", false), ("ledger", false), ("vault", true)] {
        let (plan, json, text) = imported_in(name, security, false);
        let (plan2, json2, text2) = imported_in(name, security, true);
        assert!(plan == plan2, "{name}: the plans differ");
        assert!(json == json2, "{name}: the JSON reports differ");
        assert_eq!(text, text2, "{name}: the text reports differ");
        assert!(plan.len() > 1000 && text.contains("import: "), "{name}");
    }
}
