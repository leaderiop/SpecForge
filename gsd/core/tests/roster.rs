//! The roster of upstream's 72 commands (D-r43-14): it equals the vendored `commands.json` and the README's Commands
//! table, and every command `@gsd/core` owns is served or listed in `NOT_YET_SERVED` until its ticket lands.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gsd_core::prompts::roster::{Owner, ROSTER};
use regex::Regex;

/// The core rows no prompt serves yet: each r43 ticket deletes its rows, and the last one deletes this list.
const NOT_YET_SERVED: &[&str] = &[
    "ns-context",
    "ns-ideate",
    "ns-manage",
    "ns-project",
    "ns-review",
    "ns-workflow",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The names of `commands.json`, in file order.
fn upstream_names() -> Vec<String> {
    let text = std::fs::read_to_string(root().join("../parity/upstream/v1.18.0/commands.json"))
        .expect("commands.json");
    let rows: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    rows.as_array()
        .expect("an array")
        .iter()
        .map(|r| r["name"].as_str().expect("a name").to_string())
        .collect()
}

#[test]
fn the_roster_matches_upstreams_commands() {
    let ours: Vec<&str> = ROSTER.iter().map(|(n, _)| *n).collect();
    let theirs = upstream_names();
    assert_eq!(theirs.len(), 72);
    assert_eq!(ours, theirs.iter().map(String::as_str).collect::<Vec<_>>());
    let extensions: BTreeSet<&str> = ROSTER
        .iter()
        .filter_map(|(_, o)| match o {
            Owner::Extension(e) => Some(*e),
            _ => None,
        })
        .collect();
    assert_eq!(
        extensions.into_iter().collect::<Vec<_>>(),
        [
            "@gsd/ai",
            "@gsd/codebase",
            "@gsd/import",
            "@gsd/ops",
            "@gsd/security",
            "@gsd/ui"
        ]
    );
}

/// The owner label of each row of the README's Commands table, with the stems it lists.
fn readme_rows() -> BTreeMap<String, BTreeSet<String>> {
    let text = std::fs::read_to_string(root().join("contributions/README.md")).expect("README");
    let start = text.find("## Commands").expect("the Commands table");
    let table: Vec<&str> = text[start..]
        .lines()
        .skip(1)
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .collect();
    let stem = Regex::new(r"`/gsd-([a-z0-9-]+)`").expect("a regex");
    table
        .iter()
        .filter(|l| l.contains("`/gsd-"))
        .map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let names = stem
                .captures_iter(cells[2])
                .map(|c| c[1].to_string())
                .collect();
            (cells[1].to_string(), names)
        })
        .collect()
}

#[test]
fn the_roster_matches_contributions_readme() {
    let readme = readme_rows();
    let of = |pick: &dyn Fn(Owner) -> bool| -> BTreeSet<String> {
        ROSTER
            .iter()
            .filter(|(_, o)| pick(*o))
            .map(|(n, _)| n.to_string())
            .collect()
    };
    assert_eq!(
        readme["@gsd/core"],
        of(&|o| matches!(o, Owner::CoreCommand | Owner::CorePrompt))
    );
    let routers = readme
        .iter()
        .find(|(k, _)| k.starts_with("@gsd/core skills"))
        .expect("the routers row");
    assert_eq!(*routers.1, of(&|o| o == Owner::Router));
    for extension in [
        "@gsd/ai",
        "@gsd/codebase",
        "@gsd/import",
        "@gsd/ops",
        "@gsd/security",
        "@gsd/ui",
    ] {
        assert_eq!(
            readme[extension],
            of(&|o| o == Owner::Extension(extension)),
            "{extension}"
        );
    }
    assert_eq!(readme.values().map(BTreeSet::len).sum::<usize>(), 72);
}

/// The stems of the slash commands this extension declares.
fn served() -> BTreeSet<String> {
    let declaration = gsd_core::build().declaration();
    declaration
        .surfaces
        .slash_commands
        .iter()
        .filter_map(|s| s.prompt.as_deref().or(s.command.as_deref()))
        .map(|id| id.replace('_', "-"))
        .collect()
}

#[test]
fn every_core_row_is_served_or_not_yet() {
    let served = served();
    for (name, owner) in ROSTER.iter().filter(|(_, o)| o.is_core()) {
        let listed = NOT_YET_SERVED.contains(name);
        assert_eq!(
            served.contains(*name),
            !listed,
            "{name} ({owner:?}): served {}, listed as not yet {listed}",
            served.contains(*name)
        );
    }
    for name in NOT_YET_SERVED {
        assert!(
            ROSTER.iter().any(|(n, o)| n == name && o.is_core()),
            "{name} is no core row"
        );
    }
    for name in &served {
        assert!(
            ROSTER.iter().any(|(n, o)| n == name && o.is_core()),
            "{name} is served but is no core row"
        );
    }
}
