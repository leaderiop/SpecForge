//! The upgrade check (D-r44-19, D-r44-20): a diff of a newer gsd-core against the tracked snapshot writes a backlog that
//! names, per change, the table of `@gsd/core` it lands in. `gsd/parity/upgrade.sh <gsd-core-dir> <tag>` extracts the
//! tag's documents and runs `write_the_backlog`; the other tests run offline on the vendored snapshot.

// The roster, modules and cutover tables are shared with `parity_roster.rs` and `parity_rules.rs`.
#[allow(
    dead_code,
    reason = "a shared table: this check reads the owner of a row, not its reason"
)]
#[path = "parity_roster/assets.rs"]
mod assets;
#[path = "upgrade/citations.rs"]
mod citations;
#[allow(
    dead_code,
    reason = "a shared table: this check reads the owner of a row, not its reason"
)]
#[path = "parity_rules/cutover.rs"]
mod cutover;
#[path = "upgrade/diff.rs"]
mod diff;
#[allow(
    dead_code,
    reason = "a shared table: this check reads the owner of a row, not its reason"
)]
#[path = "parity_roster/modules.rs"]
mod modules;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use assets::{NOT_EMBEDDED, Why};
use cutover::{Class, GATE_CUTOVER};
use diff::{Change, Roster};
use gsd_core::agents::roster::AGENTS;
use gsd_core::hooks::roster::HOOK_ROSTER;
use gsd_core::prompts::asset_paths;
use gsd_core::prompts::roster::ROSTER;
use modules::{MODULES, Module};

const MANIFEST: &str = include_str!("../../parity/upstream/v1.18.0/INVENTORY-MANIFEST.json");
const CAPABILITY_IDS: &str = include_str!("../../parity/upstream/v1.18.0/capability-ids.json");
const CONFIG: &str = include_str!("../../parity/upstream/v1.18.0/config-schema.manifest.json");
const SETTINGS_MD: &str = include_str!("../spec/settings.md");
const README: &str = include_str!("../../README.md");

fn core_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn snapshot() -> Roster {
    diff::roster(MANIFEST, CAPABILITY_IDS, CONFIG)
}

/// Which table of `@gsd/core` claims `item` of `family`, if any.
fn owner(family: &str, item: &str) -> Option<String> {
    match family {
        "commands" => {
            let stem = item.strip_prefix("/gsd-").unwrap_or(item);
            ROSTER
                .iter()
                .any(|(n, _)| *n == stem)
                .then(|| "ROSTER".to_string())
        }
        "agents" => {
            let base = item.strip_suffix(".compact").unwrap_or(item);
            AGENTS
                .iter()
                .any(|a| a.id == base)
                .then(|| "AGENTS".to_string())
                .or_else(|| {
                    let table = include_str!("../contributions/README.md");
                    table
                        .contains(&format!("`{base}`"))
                        .then(|| "the Agents table of contributions/README.md".to_string())
                })
        }
        "hooks" => HOOK_ROSTER
            .iter()
            .any(|(f, _)| *f == item)
            .then(|| "HOOK_ROSTER".to_string()),
        "cli_modules" => MODULES
            .iter()
            .find(|(pattern, _)| match pattern.strip_suffix('*') {
                Some(prefix) => item.starts_with(prefix),
                None => *pattern == item,
            })
            .map(|(pattern, module)| {
                let what = match module {
                    Module::Ported(file) => format!("ported in {file}"),
                    Module::Host(command) => format!("host: {command}"),
                    Module::Owned(ext) => format!("owned by {ext}"),
                    Module::NotApplicable(_) => "not applicable".to_string(),
                };
                format!("MODULES `{pattern}` ({what})")
            }),
        "workflows" | "references" | "workflow_modes" | "workflow_steps" | "workflow_detail"
        | "workflow_templates" => {
            let base = if family == "references" {
                "gsd-core/references/"
            } else {
                "gsd-core/workflows/"
            };
            if asset_paths().any(|p| p == format!("{base}{item}")) {
                return Some("the asset table (src/prompts/assets.rs)".to_string());
            }
            NOT_EMBEDDED
                .iter()
                .find(|(f, i, _)| *f == family && *i == item)
                .map(|(_, _, why)| match why {
                    Why::Owned(ext) => format!("NOT_EMBEDDED: owned by {ext}"),
                    Why::Command(name) => format!("NOT_EMBEDDED: the command `{name}`"),
                    Why::NotApplicable(_) => "NOT_EMBEDDED: not applicable".to_string(),
                })
        }
        "capabilities" => README
            .contains(item)
            .then(|| "the README capability table".to_string()),
        "config" => SETTINGS_MD
            .contains(&format!("`{item}`"))
            .then(|| "spec/settings.md".to_string()),
        "gate_cutover" => GATE_CUTOVER
            .iter()
            .find(|(n, _)| *n == item)
            .map(|(_, class)| {
                let word = match class {
                    Class::Reproduced(_) => "reproduced",
                    Class::ByConstruction(_) => "by construction",
                    Class::Owned(_) => "owned by an extension",
                    Class::NotApplicable(_) => "not applicable",
                    Class::NotReproduced(_) => "not reproduced",
                    Class::Diverges(_) => "diverges",
                };
                format!("GATE_CUTOVER ({word})")
            }),
        _ => None,
    }
}

fn tracked() -> String {
    format!("v{}", gsd_core::TRACKS.trim_start_matches("gsd-core@"))
}

#[test]
fn upgrade_diff_on_v1_18_0_is_empty() {
    let old = snapshot();
    let rows = diff::backlog(&old, &old, &[], &owner, &BTreeMap::new());
    assert!(rows.is_empty(), "{rows:?}");
    let text = diff::render("v1.18.0", &tracked(), &rows);
    assert!(text.contains("Nothing changed"), "{text}");
    assert_eq!(tracked(), "v1.18.0");
}

#[test]
fn a_changed_upstream_file_lands_in_the_backlog_with_its_owner() {
    let old = snapshot();
    let mut new = old.clone();
    new.families
        .get_mut("commands")
        .expect("commands")
        .insert("/gsd-brand-new".into());
    new.families
        .get_mut("hooks")
        .expect("hooks")
        .remove("gsd-statusline.js");
    new.capabilities.insert("brand-new-capability".into());
    new.keys.insert("workflow.brand_new".into());
    new.keys.remove("workflow.inline_plan_threshold");
    let changed = diff::changes(
        "A\tcommands/gsd/brand-new.md\nD\thooks/gsd-statusline.js\nM\tsrc/schema-detect.cts\nM\tgsd-core/workflows/plan-phase.md\nM\ttests/fixtures/gate-cutover/decision-coverage-plan-uncovered.json\nM\tsrc/never-heard-of-it.cts\n",
    );
    assert_eq!(changed.len(), 6);
    let cites = citations::index(&core_dir());
    let rows = diff::backlog(&old, &new, &changed, &owner, &cites);
    let owner_of = |path: &str| -> String {
        rows.iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| panic!("no row for {path}: {rows:#?}"))
            .owner
            .clone()
    };
    assert_eq!(owner_of("commands/gsd/brand-new.md"), "unowned");
    let statusline = owner_of("hooks/gsd-statusline.js");
    assert!(
        statusline.starts_with("HOOK_ROSTER; cited by ")
            && statusline.contains("src/hooks/statusline.rs"),
        "{statusline}"
    );
    assert!(
        owner_of("src/schema-detect.cts").contains("MODULES"),
        "{rows:#?}"
    );
    assert!(
        owner_of("src/schema-detect.cts").contains("cited by src/passes/schema_drift"),
        "{rows:#?}"
    );
    assert!(owner_of("gsd-core/workflows/plan-phase.md").contains("asset table"));
    assert!(
        owner_of("tests/fixtures/gate-cutover/decision-coverage-plan-uncovered.json")
            .contains("reproduced")
    );
    assert_eq!(owner_of("src/never-heard-of-it.cts"), "unowned");
    // The roster items no path explains: a capability and two config keys.
    assert_eq!(owner_of("(roster) brand-new-capability"), "unowned");
    assert_eq!(owner_of("(roster) workflow.brand_new"), "unowned");
    assert_eq!(
        owner_of("(roster) workflow.inline_plan_threshold"),
        "spec/settings.md"
    );
    // Each change is one row: the new command is not listed again by the roster.
    assert_eq!(
        rows.iter()
            .filter(|r| r.path.contains("brand-new") && r.family == "commands")
            .count(),
        1
    );
    let text = diff::render("v9.9.9", &tracked(), &rows);
    assert!(
        text.contains("| commands | added | `commands/gsd/brand-new.md` | unowned |"),
        "{text}"
    );
}

#[test]
fn every_citation_names_a_file_of_the_snapshot() {
    let cites = citations::index(&core_dir());
    assert!(
        cites.len() >= 30,
        "the headers cite upstream files: {}",
        cites.len()
    );
    let roster = snapshot();
    let vendored = core_dir().join("../parity/upstream").join(tracked());
    let missing: Vec<&String> = cites
        .keys()
        .filter(|path| {
            let (family, item) = diff::locate(path);
            let listed = roster
                .families
                .get(&family)
                .is_some_and(|items| items.contains(&item));
            !(vendored.join(path).is_file() || listed)
        })
        .collect();
    assert!(
        missing.is_empty(),
        "cited, and neither vendored nor in the manifest: {missing:?}"
    );
}

#[test]
fn upgrade_sh_parses() {
    let script = core_dir().join("../parity/upgrade.sh");
    let parsed = Command::new("bash")
        .arg("-n")
        .arg(&script)
        .status()
        .expect("bash");
    assert!(parsed.success(), "bash -n");
    let run = Command::new("bash").arg(&script).output().expect("bash");
    assert!(!run.status.success(), "no argument is an error");
    let usage = String::from_utf8_lossy(&run.stderr);
    assert!(
        usage.contains("usage: upgrade.sh <gsd-core-dir> <tag>"),
        "{usage}"
    );
}

/// Writes `gsd/parity/backlog/<tag>.md`. Run by `upgrade.sh`, which extracts the tag's documents into `GSD_UPGRADE_DIR`.
#[test]
#[ignore = "run by gsd/parity/upgrade.sh"]
fn write_the_backlog() {
    let dir = PathBuf::from(std::env::var("GSD_UPGRADE_DIR").expect("GSD_UPGRADE_DIR"));
    let tag = std::env::var("GSD_UPGRADE_TAG").expect("GSD_UPGRADE_TAG");
    let read = |name: &str| {
        std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
    };
    let new = diff::roster(
        &read("INVENTORY-MANIFEST.json"),
        &read("capability-ids.json"),
        &read("config-schema.manifest.json"),
    );
    let changed: Vec<Change> = diff::changes(&read("changes.txt"));
    let rows = diff::backlog(
        &snapshot(),
        &new,
        &changed,
        &owner,
        &citations::index(&core_dir()),
    );
    let out = core_dir().join("../parity/backlog");
    std::fs::create_dir_all(&out).expect("the backlog dir");
    std::fs::write(
        out.join(format!("{tag}.md")),
        diff::render(&tag, &tracked(), &rows),
    )
    .expect("the backlog");
    let kinds: BTreeSet<&str> = rows.iter().map(|r| r.change).collect();
    eprintln!("{} rows ({kinds:?})", rows.len());
}
