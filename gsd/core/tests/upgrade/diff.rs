//! The diff of a newer gsd-core against the tracked snapshot (D-r44-19): what changed upstream, and which table of
//! `@gsd/core` owns each change. Pure functions; `upgrade.rs` feeds them and says who owns what.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// What a release holds that the roster check compares: the inventory manifest's ten families, the capability ids and
/// the config keys.
#[derive(Clone, Default)]
pub struct Roster {
    pub families: BTreeMap<String, BTreeSet<String>>,
    pub capabilities: BTreeSet<String>,
    pub keys: BTreeSet<String>,
}

fn strings(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// A roster from the three documents the snapshot (and `upgrade.sh`) carry.
pub fn roster(manifest: &str, capability_ids: &str, config: &str) -> Roster {
    let parse = |text: &str, what: &str| -> Value {
        serde_json::from_str(text).unwrap_or_else(|e| panic!("{what} is JSON: {e}"))
    };
    let manifest = parse(manifest, "the inventory manifest");
    let families = manifest["families"]
        .as_object()
        .expect("families")
        .iter()
        .map(|(name, items)| (name.clone(), strings(items)))
        .collect();
    Roster {
        families,
        capabilities: strings(&parse(capability_ids, "the capability ids")),
        keys: strings(&parse(config, "the config manifest")["validKeys"]),
    }
}

/// One line of `git diff --name-status --no-renames`.
pub struct Change {
    pub status: char,
    pub path: String,
}

pub fn changes(text: &str) -> Vec<Change> {
    text.lines()
        .filter_map(|line| {
            let (status, path) = line.split_once('\t')?;
            Some(Change {
                status: status.chars().next()?,
                path: path.to_string(),
            })
        })
        .collect()
}

/// The family and item an upstream path stands for: the names of the inventory manifest (a `src/**/*.cts` and its
/// built `gsd-core/bin/lib/**/*.cjs` are one `cli_modules` item), `templates` for `gsd-core/templates/`, `suites` for a
/// copied `tests/*.test.cjs`, `gate_cutover` for a verdict and `other` for a path no family claims.
pub fn locate(path: &str) -> (String, String) {
    let (family, item) = if let Some(rest) = path.strip_prefix("commands/gsd/") {
        ("commands", format!("/gsd-{}", rest.trim_end_matches(".md")))
    } else if let Some(rest) = path.strip_prefix("agents/") {
        ("agents", rest.trim_end_matches(".md").to_string())
    } else if let Some(rest) = path
        .strip_prefix("hooks/")
        .filter(|r| r.starts_with("gsd-") && !r.contains('/'))
    {
        ("hooks", rest.to_string())
    } else if let Some(rest) = path.strip_prefix("gsd-core/templates/") {
        ("templates", rest.to_string())
    } else if let Some(rest) = path.strip_prefix("gsd-core/bin/lib/") {
        ("cli_modules", rest.to_string())
    } else if let Some(rest) = path.strip_prefix("gsd-core/workflows/") {
        let family = ["modes", "steps", "detail", "templates"]
            .iter()
            .find(|d| rest.contains(&format!("/{d}/")))
            .map_or("workflows".to_string(), |d| format!("workflow_{d}"));
        return (family, rest.to_string());
    } else if let Some(rest) = path.strip_prefix("gsd-core/references/") {
        ("references", rest.to_string())
    } else if let Some(rest) = path
        .strip_prefix("src/")
        .and_then(|r| r.strip_suffix(".cts"))
    {
        ("cli_modules", format!("{rest}.cjs"))
    } else if let Some(rest) = path.strip_prefix("capabilities/") {
        (
            "capabilities",
            rest.split('/').next().unwrap_or(rest).to_string(),
        )
    } else if let Some(rest) = path.strip_prefix("tests/fixtures/gate-cutover/") {
        ("gate_cutover", rest.trim_end_matches(".json").to_string())
    } else if path.starts_with("tests/") && path.ends_with(".test.cjs") {
        ("suites", path.to_string())
    } else {
        ("other", path.to_string())
    };
    (family.to_string(), item)
}

/// One change of the backlog.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row {
    pub family: String,
    pub change: &'static str,
    pub path: String,
    pub owner: String,
}

/// The backlog: every changed path, and every roster item that appeared or went that no path explains, each with the
/// SpecForge owner `owner` names (`None` is "unowned") and the files of `gsd/core/src` that cite the path.
pub fn backlog(
    old: &Roster,
    new: &Roster,
    changed: &[Change],
    owner: &dyn Fn(&str, &str) -> Option<String>,
    cites: &BTreeMap<String, Vec<String>>,
) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for change in changed {
        let (family, item) = locate(&change.path);
        seen.insert((family.clone(), item.clone()));
        let word = match change.status {
            'A' => "added",
            'D' => "removed",
            _ => "changed",
        };
        let mut owners: Vec<String> = owner(&family, &item).into_iter().collect();
        if let Some(files) = cites.get(&change.path) {
            owners.push(format!("cited by {}", files.join(", ")));
        }
        rows.push(Row {
            family,
            change: word,
            path: change.path.clone(),
            owner: if owners.is_empty() {
                "unowned".to_string()
            } else {
                owners.join("; ")
            },
        });
    }
    let mut roster_rows = |family: &str, was: &BTreeSet<String>, is: &BTreeSet<String>| {
        for (word, items) in [
            ("added", is.difference(was)),
            ("removed", was.difference(is)),
        ] {
            for item in items {
                if !seen.insert((family.to_string(), item.clone())) {
                    continue;
                }
                rows.push(Row {
                    family: family.to_string(),
                    change: word,
                    path: format!("(roster) {item}"),
                    owner: owner(family, item).unwrap_or_else(|| "unowned".to_string()),
                });
            }
        }
    };
    let none = BTreeSet::new();
    for family in old
        .families
        .keys()
        .chain(new.families.keys())
        .collect::<BTreeSet<_>>()
    {
        let (was, is) = (old.families.get(family), new.families.get(family));
        roster_rows(family, was.unwrap_or(&none), is.unwrap_or(&none));
    }
    roster_rows("capabilities", &old.capabilities, &new.capabilities);
    roster_rows("config", &old.keys, &new.keys);
    rows.sort();
    rows
}

/// The backlog as `gsd/parity/backlog/<tag>.md`.
pub fn render(tag: &str, tracked: &str, rows: &[Row]) -> String {
    let mut out = format!("# Parity backlog: gsd-core {tag} against {tracked}\n\n");
    if rows.is_empty() {
        out.push_str("Nothing changed in the families the parity suite compares.\n");
        return out;
    }
    out.push_str(&format!(
        "{} change(s). A row owned by a table is an edit of that table; `unowned` is new work.\n\n",
        rows.len()
    ));
    out.push_str("| Family | Change | Upstream path | SpecForge owner |\n|---|---|---|---|\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | `{}` | {} |\n",
            row.family, row.change, row.path, row.owner
        ));
    }
    out
}
