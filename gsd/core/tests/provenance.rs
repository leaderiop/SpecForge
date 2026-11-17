//! ADR 0370 D1: every authored field of `@gsd/core` (a field whose `writer` is absent or `human`) names, in
//! `spec/provenance.md`, the gsd-core 1.19.0 line it mirrors or the SpecForge capability that reads it. The table is
//! read through gsd-kit's Markdown reader; an upstream line is read from the vendored snapshot.

#[allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use common::describe;

const HEADER: [&str; 4] = ["kind", "field", "source", "the line holds"];

/// One row of a provenance table.
#[derive(Clone, Debug)]
struct Row {
    kind: String,
    field: String,
    source: String,
    holds: String,
}

/// What the declaration names: its authored and computed fields, and the capabilities a row may cite.
struct Declared {
    authored: BTreeSet<(String, String)>,
    computed: BTreeSet<(String, String)>,
    passes: BTreeSet<String>,
    codes: BTreeSet<String>,
    routes: BTreeSet<String>,
    commands: BTreeSet<String>,
}

/// Every row of every table of `md` whose header is `kind | field | source | the line holds`.
fn rows(md: &str) -> Vec<Row> {
    gsd_kit::markdown::tables(md)
        .into_iter()
        .filter(|t| t.header == HEADER)
        .flat_map(|t| t.rows)
        .map(|cells| {
            let [kind, field, source, holds] = &cells[..] else {
                panic!("a provenance row has four cells: {cells:?}");
            };
            Row {
                kind: kind.clone(),
                field: field.clone(),
                source: source.clone(),
                holds: holds.clone(),
            }
        })
        .collect()
}

/// The names of a describe category's items.
fn names(category: &str) -> BTreeSet<String> {
    let names: BTreeSet<String> = describe(category)["items"]
        .as_array()
        .unwrap_or_else(|| panic!("{category} has items"))
        .iter()
        .map(|i| i["name"].as_str().expect("a name").to_string())
        .collect();
    assert!(!names.is_empty(), "no {category}");
    names
}

fn declared() -> Declared {
    let mut authored = BTreeSet::new();
    let mut computed = BTreeSet::new();
    for kind in describe("entities")["items"].as_array().expect("kinds") {
        let name = kind["name"].as_str().expect("a kind name");
        for field in kind["fields"].as_array().expect("fields") {
            let pair = (
                name.to_string(),
                field["name"].as_str().expect("a field name").to_string(),
            );
            match field["writer"].as_str() {
                None | Some("human") => authored.insert(pair),
                Some("computed") => computed.insert(pair),
                Some(_) => false,
            };
        }
    }
    let commands = gsd_core::build()
        .declaration()
        .surfaces
        .commands
        .iter()
        .map(|c| c.id.clone())
        .collect();
    let mut routes = names("routes");
    routes.extend(names("contexts"));
    Declared {
        authored,
        computed,
        passes: names("passes"),
        codes: gsd_core::codes::CODES
            .iter()
            .map(|c| c.id.to_string())
            .collect(),
        routes,
        commands,
    }
}

/// The vendored gsd-core release the upstream rows cite.
fn upstream() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/upstream/v1.19.0")
}

/// Why a capability does not name something the declaration declares, or `None` when it does.
fn capability_problem(declared: &Declared, prefix: &str, name: &str) -> Option<String> {
    let known = match prefix {
        "pass" => declared.passes.contains(name),
        "rule" => name
            .strip_prefix("gsd/")
            .is_some_and(|code| declared.codes.contains(code)),
        "route" => declared.routes.contains(name),
        "command" => declared.commands.contains(name),
        "computed" => name.split_once('.').is_some_and(|(kind, field)| {
            declared
                .computed
                .contains(&(kind.to_string(), field.to_string()))
        }),
        _ => return Some(format!("`{prefix}:` is no capability prefix")),
    };
    (!known).then(|| format!("`{prefix}:{name}` is not declared"))
}

/// Why an upstream source does not hold its text, or `None` when it does.
fn upstream_problem(
    root: &Path,
    files: &mut BTreeMap<String, Option<String>>,
    row: &Row,
) -> Option<String> {
    let Some((path, line)) = row.source.rsplit_once(':') else {
        return Some(format!(
            "`{}` is neither a capability nor `<path>:<line>`",
            row.source
        ));
    };
    let Ok(line) = line.parse::<usize>() else {
        return Some(format!("`{}` has no line number", row.source));
    };
    if row.holds.is_empty() {
        return Some(format!("`{}` names no text the line holds", row.source));
    }
    let text = files
        .entry(path.to_string())
        .or_insert_with(|| std::fs::read_to_string(root.join(path)).ok());
    let Some(text) = text else {
        return Some(format!("{path} is not in the upstream snapshot"));
    };
    match text.lines().nth(line.wrapping_sub(1)) {
        None => Some(format!("{path} has no line {line}")),
        Some(held) if !held.contains(&row.holds) => Some(format!(
            "{path}:{line} does not hold `{}`: it reads `{held}`",
            row.holds
        )),
        Some(_) => None,
    }
}

/// Every way `rows` fails the declaration, each naming its kind and field.
fn problems(rows: &[Row], declared: &Declared, root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    let mut files = BTreeMap::new();
    for row in rows {
        let at = format!("{}.{}", row.kind, row.field);
        let pair = (row.kind.clone(), row.field.clone());
        if !seen.insert(pair.clone()) {
            found.push(format!("{at}: a second row"));
        }
        if !declared.authored.contains(&pair) {
            found.push(format!("{at}: a row for a field that is not authored"));
        }
        let problem = match row.source.split_once(':') {
            Some((prefix, name)) if prefix.bytes().all(|b| b.is_ascii_lowercase()) => {
                let problem = capability_problem(declared, prefix, name);
                if problem.is_none() && !row.holds.is_empty() {
                    Some("a capability row holds no upstream text".to_string())
                } else {
                    problem
                }
            }
            _ => upstream_problem(root, &mut files, row),
        };
        if let Some(problem) = problem {
            found.push(format!("{at}: {problem}"));
        }
    }
    for (kind, field) in declared.authored.difference(&seen) {
        found.push(format!("{kind}.{field}: an authored field with no row"));
    }
    found
}

fn doc_rows() -> Vec<Row> {
    rows(include_str!("../spec/provenance.md"))
}

#[test]
fn every_authored_field_names_its_upstream_line_or_the_capability_that_reads_it() {
    let declared = declared();
    let rows = doc_rows();
    let found = problems(&rows, &declared, &upstream());
    assert!(found.is_empty(), "{}", found.join("\n"));
    assert_eq!(
        rows.len(),
        declared.authored.len(),
        "one row per authored field"
    );
}

/// Each check fails on the case it exists for, and names the kind and field.
#[test]
fn the_check_reports_each_kind_of_wrong_row() {
    let declared = declared();
    let rows = doc_rows();
    let case = |edit: &dyn Fn(&mut Vec<Row>)| {
        let mut rows = rows.clone();
        edit(&mut rows);
        problems(&rows, &declared, &upstream())
    };
    let row_of = |rows: &mut Vec<Row>, kind: &str, field: &str| -> usize {
        rows.iter()
            .position(|r| r.kind == kind && r.field == field)
            .unwrap_or_else(|| panic!("no row {kind}.{field}"))
    };

    // a declared authored field with no row
    let found = case(&|rows| {
        let at = row_of(rows, "pause", "reason");
        rows.remove(at);
    });
    assert_eq!(found, ["pause.reason: an authored field with no row"]);

    // a row for a computed field
    let found = case(&|rows| {
        rows.push(Row {
            kind: "milestone".into(),
            field: "state".into(),
            source: "computed:milestone.state".into(),
            holds: String::new(),
        });
    });
    assert_eq!(
        found,
        ["milestone.state: a row for a field that is not authored"]
    );

    // an upstream row whose line does not hold its text
    let found = case(&|rows| {
        let at = row_of(rows, "project", "name");
        rows[at].holds = "# [Product Name]".into();
    });
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("project.name: gsd-core/templates/project.md:8 does not hold"),
        "{found:?}"
    );

    // an upstream row whose file or line is not in the snapshot
    let found = case(&|rows| {
        let at = row_of(rows, "project", "name");
        rows[at].source = "gsd-core/templates/project.md:100000".into();
    });
    assert_eq!(
        found,
        ["project.name: gsd-core/templates/project.md has no line 100000"]
    );

    // a `pass:` row naming no declared pass, and a rule that is no code
    let found = case(&|rows| {
        let at = row_of(rows, "workstream", "purpose");
        rows[at].source = "pass:no-such-pass".into();
        let at = row_of(rows, "decision", "settled");
        rows[at].source = "rule:gsd/W999".into();
    });
    assert_eq!(
        found,
        [
            "workstream.purpose: `pass:no-such-pass` is not declared",
            "decision.settled: `rule:gsd/W999` is not declared",
        ]
    );
}
