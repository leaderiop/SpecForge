//! The parity roster (D-r44-17): every item of the tracked gsd-core's inventory manifest, its 45 capabilities and its
//! configuration keys is accounted in one place, against one table of SpecForge's. Commands are compared by
//! `roster.rs`, configuration keys by `settings_roster.rs`; this file maps those families to their tables so the
//! whole snapshot is accounted together, and holds the tables of the rest.

#[path = "parity_roster/accept.rs"]
mod accept;
#[path = "parity_roster/assets.rs"]
mod assets;
#[allow(
    dead_code,
    reason = "a shared table: the acceptance counts read its classes, not their reasons"
)]
#[path = "parity_rules/cutover.rs"]
mod cutover;
#[allow(
    dead_code,
    reason = "a shared table: the acceptance counts read its kinds, not the test names"
)]
#[path = "parity_rules/evidence.rs"]
mod evidence;
#[path = "parity_roster/modules.rs"]
mod modules;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use assets::{NOT_EMBEDDED, Why};
use gsd_core::agents::roster::AGENTS;
use gsd_core::hooks::roster::HOOK_ROSTER;
use gsd_core::prompts::asset_paths as paths;
use modules::{MODULES, Module};
use roster::{Owner, ROSTER};

#[path = "common/roster.rs"]
mod roster;
use serde_json::Value;

const MANIFEST: &str = include_str!(gsd_kit::snapshot!("INVENTORY-MANIFEST.json"));
const CAPABILITY_IDS: &str = include_str!(gsd_kit::snapshot!("capability-ids.json"));
const CONFIG_SCHEMA: &str = include_str!(gsd_kit::snapshot!("config-schema.manifest.json"));
const CAPABILITIES: &str = include_str!(gsd_kit::snapshot!("capabilities.json"));
const README: &str = include_str!("../../README.md");
const CONTRIBUTIONS: &str = include_str!("../contributions/README.md");

const EXTENSIONS: [&str; 6] = [
    "@gsd/ai",
    "@gsd/codebase",
    "@gsd/import",
    "@gsd/ops",
    "@gsd/security",
    "@gsd/ui",
];
const ASSET_FAMILIES: [&str; 6] = [
    "workflows",
    "references",
    "workflow_modes",
    "workflow_steps",
    "workflow_detail",
    "workflow_templates",
];

fn core_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn family(name: &str) -> Vec<String> {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("the manifest");
    manifest["families"][name]
        .as_array()
        .unwrap_or_else(|| panic!("no family {name}"))
        .iter()
        .map(|v| v.as_str().expect("a name").to_string())
        .collect()
}

/// The manifest's ten families, in the order the manifest gives them.
fn families() -> Vec<String> {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("the manifest");
    manifest["families"]
        .as_object()
        .expect("families")
        .keys()
        .cloned()
        .collect()
}

/// The path an embedded asset has for an item of `family`.
fn asset_path(family: &str, item: &str) -> String {
    let base = if family == "references" {
        "gsd-core/references/"
    } else {
        "gsd-core/workflows/"
    };
    format!("{base}{item}")
}

fn embedded() -> BTreeSet<&'static str> {
    paths().collect()
}

/// The agents the extensions own, from the contributions README's Agents table.
fn agent_owners() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let section = CONTRIBUTIONS
        .split("\n## Agents\n")
        .nth(1)
        .expect("an Agents section");
    for row in section.lines().take_while(|l| !l.starts_with("###")) {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        if cells.len() < 4 || !cells[1].starts_with("@gsd/") {
            continue;
        }
        for name in cells[2].split('`').skip(1).step_by(2) {
            assert!(
                out.insert(name.to_string(), cells[1].to_string()).is_none(),
                "{name} is in two rows"
            );
        }
    }
    out
}

/// How many rows of `MODULES` name `module`.
fn module_rows(module: &str) -> Vec<&'static str> {
    MODULES
        .iter()
        .filter(|(pattern, _)| match pattern.strip_suffix('*') {
            Some(prefix) => module.starts_with(prefix),
            None => *pattern == module,
        })
        .map(|(pattern, _)| *pattern)
        .collect()
}

/// `None` when `item` of `family` is accounted exactly once, else what is wrong.
fn unaccounted(family: &str, item: &str) -> Option<String> {
    let problem = |what: &str| Some(format!("{family}: {item}: {what}"));
    match family {
        "commands" => {
            let stem = item.strip_prefix("/gsd-").unwrap_or(item);
            (!ROSTER.iter().any(|(n, _)| *n == stem))
                .then(|| format!("{family}: {item}: no row of ROSTER"))
        }
        "agents" => {
            let base = item.strip_suffix(".compact").unwrap_or(item);
            let ours = AGENTS.iter().any(|a| a.id == base);
            match agent_owners().get(base).map(String::as_str) {
                Some("@gsd/core") if ours => None,
                Some("@gsd/core") => problem("owned by @gsd/core in the table but not in AGENTS"),
                Some(_) if ours => problem("in AGENTS but owned elsewhere in the table"),
                Some(_) => None,
                None => problem("no owner in the Agents table"),
            }
        }
        "hooks" => (!HOOK_ROSTER.iter().any(|(f, _)| *f == item))
            .then(|| format!("{family}: {item}: no row of HOOK_ROSTER")),
        "cli_modules" => match module_rows(item).len() {
            1 => None,
            0 => problem("no row of MODULES"),
            n => problem(&format!("{n} rows of MODULES: {:?}", module_rows(item))),
        },
        f if ASSET_FAMILIES.contains(&f) => {
            let is_embedded = embedded().contains(asset_path(f, item).as_str());
            let listed = NOT_EMBEDDED
                .iter()
                .filter(|(fam, i, _)| *fam == f && *i == item)
                .count();
            match (is_embedded, listed) {
                (true, 0) | (false, 1) => None,
                (false, 0) => problem("neither embedded nor in NOT_EMBEDDED"),
                (true, _) => problem("embedded and in NOT_EMBEDDED"),
                (false, _) => problem("twice in NOT_EMBEDDED"),
            }
        }
        other => Some(format!("no table for the family {other}")),
    }
}

#[test]
fn every_inventory_family_is_accounted() {
    let names = families();
    assert_eq!(names.len(), 10, "{names:?}");
    for name in names {
        for item in family(&name) {
            if let Some(why) = unaccounted(&name, &item) {
                panic!("{why}");
            }
        }
    }
}

#[test]
fn commands_match_the_snapshot() {
    let theirs = family("commands");
    let mut ours: Vec<String> = ROSTER.iter().map(|(n, _)| format!("/gsd-{n}")).collect();
    ours.sort();
    let mut theirs = theirs;
    theirs.sort();
    assert_eq!(ours, theirs);
}

#[test]
fn agents_match_the_snapshot() {
    let all = family("agents");
    let base: BTreeSet<&str> = all
        .iter()
        .filter(|a| !a.ends_with(".compact"))
        .map(String::as_str)
        .collect();
    let compact: BTreeSet<&str> = all
        .iter()
        .filter_map(|a| a.strip_suffix(".compact"))
        .collect();
    assert_eq!((all.len(), base.len(), compact.len()), (64, 35, 29));
    let owners = agent_owners();
    assert_eq!(owners.len(), 35, "the table totals 35");
    let core: BTreeSet<&str> = owners
        .iter()
        .filter(|(_, o)| *o == "@gsd/core")
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(
        core,
        AGENTS.iter().map(|a| a.id).collect::<BTreeSet<_>>(),
        "the 15 core agents"
    );
    // every base name has one owner, and a compact name follows its base
    assert_eq!(
        owners.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        base
    );
    assert!(compact.is_subset(&base));
    // a core agent with a compact body upstream ships one
    for agent in AGENTS {
        assert_eq!(
            agent.compact.is_some(),
            compact.contains(agent.id),
            "{}",
            agent.id
        );
    }
    // the table's owners are the extensions
    for owner in owners.values() {
        assert!(
            *owner == "@gsd/core" || EXTENSIONS.contains(&owner.as_str()),
            "{owner}"
        );
    }
}

#[test]
fn hooks_match_the_snapshot() {
    let theirs: Vec<String> = family("hooks");
    let ours: Vec<&str> = HOOK_ROSTER.iter().map(|(f, _)| *f).collect();
    assert_eq!(ours, theirs.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn prompt_assets_match_the_snapshot() {
    let mut items = 0;
    let mut not_embedded = 0;
    for fam in ASSET_FAMILIES {
        for item in family(fam) {
            items += 1;
            let path = asset_path(fam, &item);
            if !embedded().contains(path.as_str()) {
                not_embedded += 1;
            }
        }
    }
    assert_eq!((items, not_embedded, NOT_EMBEDDED.len()), (295, 77, 77));
    let seen: BTreeSet<(&str, &str)> = NOT_EMBEDDED.iter().map(|(f, i, _)| (*f, *i)).collect();
    assert_eq!(seen.len(), NOT_EMBEDDED.len(), "a row twice");
    let commands: BTreeSet<&str> = ROSTER
        .iter()
        .filter(|(_, o)| *o == Owner::CoreCommand)
        .map(|(n, _)| *n)
        .collect();
    for (fam, item, why) in NOT_EMBEDDED {
        assert!(
            family(fam).contains(&item.to_string()),
            "{fam}: {item} is not upstream's"
        );
        match why {
            Why::Owned(ext) => assert!(EXTENSIONS.contains(ext), "{item}: {ext}"),
            Why::Command(name) => {
                assert!(commands.contains(name), "{item}: no core command {name}")
            }
            Why::NotApplicable(reason) => assert!(reason.len() > 30, "{item}: a reason"),
        }
    }
}

#[test]
fn cli_modules_match_the_snapshot() {
    let modules = family("cli_modules");
    assert_eq!(modules.len(), 282);
    let owned: BTreeSet<&str> = EXTENSIONS.into_iter().collect();
    let mut uncited = Vec::new();
    for (pattern, module) in MODULES {
        assert!(
            modules.iter().any(|m| module_rows(m).contains(pattern)),
            "{pattern} names no module"
        );
        match module {
            Module::Ported(file) => {
                assert!(core_dir().join(file).is_file(), "{pattern}: {file}");
                // The file's `//!` header says it ports the module (REVIEW-r44 M11): a row pointed at another file fails.
                let text = std::fs::read_to_string(core_dir().join(file)).expect("a source file");
                let header: String = text
                    .lines()
                    .take_while(|l| l.starts_with("//!") || l.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                if !header_names(&header, pattern) {
                    uncited.push(format!("{pattern} -> {file}"));
                }
            }
            Module::Host(command) => {
                assert!(command.starts_with("specforge "), "{pattern}: {command}")
            }
            Module::Owned(ext) => assert!(owned.contains(ext), "{pattern}: {ext}"),
            Module::NotApplicable(reason) => assert!(reason.len() > 30, "{pattern}: a reason"),
        }
    }
    assert!(
        uncited.is_empty(),
        "{} Ported rows whose file's header does not name the module:\n{}",
        uncited.len(),
        uncited.join("\n")
    );
    // no prefix row shadows an exact row: a module has one row
    for module in &modules {
        assert_eq!(
            module_rows(module).len(),
            1,
            "{module}: {:?}",
            module_rows(module)
        );
    }
}

/// The capability names the README's table lands.
fn readme_capabilities() -> Vec<String> {
    let section = README
        .split("## GSD capabilities, one by one")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("the capabilities section");
    section
        .lines()
        .filter(|l| {
            l.starts_with("| ") && !l.starts_with("| GSD capability") && !l.starts_with("|---")
        })
        .flat_map(|l| {
            l.split('|')
                .nth(1)
                .unwrap_or_default()
                .split(',')
                .map(|n| n.trim().to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn capabilities_match_the_snapshot() {
    let ids: Vec<String> = serde_json::from_str(CAPABILITY_IDS).expect("capability ids");
    assert_eq!(ids.len(), 45);
    let mut rows = readme_capabilities();
    rows.sort();
    assert_eq!(
        rows, ids,
        "every capability in one README row, and no extra"
    );
}

#[test]
fn config_keys_match_the_snapshot() {
    let schema: Value = serde_json::from_str(CONFIG_SCHEMA).expect("schema");
    let capabilities: Value = serde_json::from_str(CAPABILITIES).expect("capabilities");
    let mut keys: BTreeSet<&str> = schema["validKeys"]
        .as_array()
        .expect("validKeys")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    keys.extend(
        capabilities
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str),
    );
    // `settings_roster.rs` accounts each of them: declared, owned elsewhere or not applicable
    assert_eq!(keys.len(), 211);
}

/// Whether `header` names the module `pattern` (`phase.cjs`, or a prefix row `profile*`) as a whole file name: the
/// file name stands at a word start (after a space, `/`, a backtick or a comma) and, for an exact row, ends there too
/// (REVIEW-r44 post-fix N4: a substring matched prose words and other modules).
fn header_names(header: &str, pattern: &str) -> bool {
    let (stem, prefix) = match pattern.strip_suffix('*') {
        Some(p) => (p, true),
        None => (pattern, false),
    };
    header.match_indices(stem).any(|(at, _)| {
        let before = header[..at].chars().next_back();
        let after = header[at + stem.len()..].chars().next();
        before.is_none_or(|c| matches!(c, ' ' | '/' | '`' | ',' | '(' | '\n'))
            && (prefix
                || after.is_none_or(|c| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))))
    })
}

#[test]
fn a_header_names_a_module_as_a_whole_file_name() {
    let header = concat!(
        "//! Ported from gsd-core v",
        gsd_kit::tracked_version!(),
        " bin/lib/state.cjs and phase.cjs, MIT; the phase of a plan"
    );
    assert!(header_names(header, "phase.cjs") && header_names(header, "state.cjs"));
    assert!(!header_names(header, "verify.cjs") && !header_names(header, "phas.cjs"));
    assert!(
        !header_names("//! the phase.cjsx file", "phase.cjs")
            && !header_names("//! the phase-id.cjs", "phase.cjs")
    );
    assert!(header_names("//! ports phase.cjs.", "phase.cjs"));
    assert!(header_names(
        "//! Ported from profile-output.cjs",
        "profile*"
    ));
    assert!(
        !header_names("//! a profile of the user", "profile.cjs")
            && !header_names("//! the phase of it", "phase.cjs")
    );
}

const ACCEPTANCE: &str = include_str!("../../parity/ACCEPTANCE.md");
const PARITY_README: &str = include_str!("../../parity/README.md");

#[test]
fn the_acceptance_counts_match() {
    let by: BTreeMap<String, usize> = families()
        .into_iter()
        .map(|f| (f.clone(), family(&f).len()))
        .collect();
    assert_eq!(by["commands"], ROSTER.len());
    assert_eq!(by["hooks"], HOOK_ROSTER.len());
    assert_eq!(by["cli_modules"], 282);
    let assets: usize = ASSET_FAMILIES.iter().map(|f| by[*f]).sum();
    assert_eq!(assets, 295);
    assert_eq!(by["agents"], 64);
    assert_eq!(by.values().sum::<usize>(), 72 + 64 + 28 + 282 + 295);
    let rows: [(&str, accept::Counts); 9] = [
        ("Commands", accept::commands()),
        ("Agents", accept::agents()),
        ("Hooks", accept::hooks()),
        ("Capabilities", accept::capabilities()),
        ("Workflows and references", accept::assets()),
        ("CLI modules", accept::cli_modules()),
        ("Config keys", accept::config_keys()),
        ("Gate-cutover verdicts", accept::gate_cutover()),
        ("Codes", accept::codes()),
    ];
    for (name, computed) in rows {
        assert_eq!(
            accept::row(ACCEPTANCE, name),
            computed,
            "{name}: ACCEPTANCE.md against the tables"
        );
    }
    // the totals the plan names
    assert_eq!(accept::hooks(), [28, 16, 0, 10, 2]);
    assert_eq!(accept::capabilities(), [45, 17, 10, 0, 18]);
    assert_eq!(accept::gate_cutover(), [171, 92, 51, 14, 14]);
    assert_eq!(accept::codes(), [63, 63, 0, 0, 0]);
    assert_eq!(accept::config_keys()[0], 211);
}

/// REVIEW-k20b m5: each family's "Where" file in ACCEPTANCE.md exists.
#[test]
fn each_acceptance_where_path_exists() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut checked = 0;
    for line in ACCEPTANCE
        .lines()
        .skip_while(|l| !l.starts_with("| Family | Total"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
    {
        let where_ = line.split('|').nth(7).unwrap_or_default();
        let path = where_.split('`').nth(1).expect("a `path` in Where");
        assert!(root.join(path).is_file(), "{path}: no such file");
        checked += 1;
    }
    assert_eq!(checked, 9);
}

#[test]
fn the_program_docs_shell_blocks_parse() {
    for (name, doc) in [
        ("ACCEPTANCE.md", ACCEPTANCE),
        ("parity/README.md", PARITY_README),
    ] {
        let mut blocks = 0;
        for block in doc.split("```").skip(1).step_by(2) {
            let (lang, body) = block.split_once('\n').unwrap_or((block, ""));
            if !matches!(lang.trim(), "sh" | "bash") {
                continue;
            }
            blocks += 1;
            let out = std::process::Command::new("bash")
                .arg("-n")
                .arg("-c")
                .arg(body)
                .output()
                .expect("bash");
            assert!(
                out.status.success(),
                "{name}: {body}\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        assert!(
            blocks > 0 || name == "parity/README.md",
            "{name} has no shell block"
        );
    }
}
