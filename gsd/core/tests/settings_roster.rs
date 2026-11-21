//! The configuration roster: every key of the tracked gsd-core is declared by `@gsd/core`, owned by another `@gsd/*`
//! extension, or not applicable, per `spec/settings.md` (parity check "roster"). The snapshot is
//! `gsd/parity/upstream/v1.19.0/`, so the check runs offline.

use std::collections::BTreeSet;
use std::path::Path;

use gsd_core::build;
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

#[path = "common/files.rs"]
mod files;

const MANIFEST: &str = include_str!(gsd_kit::snapshot!("config-schema.manifest.json"));
const CAPABILITIES: &str = include_str!(gsd_kit::snapshot!("capabilities.json"));
const DEFAULTS: &str = include_str!(gsd_kit::snapshot!("config-defaults.manifest.json"));
const SETTINGS_MD: &str = include_str!("../spec/settings.md");

/// Keys only `docs/CONFIGURATION.md` lists (neither the manifest nor a capability does).
const DOCS_ONLY: &[&str] = &[
    "dynamic_routing.enabled",
    "dynamic_routing.escalate_on_failure",
    "dynamic_routing.max_escalations",
    "dynamic_routing.provider_escalation",
    "effort.routing_tier_defaults.heavy",
    "effort.routing_tier_defaults.light",
    "effort.routing_tier_defaults.standard",
    "fast_mode.routing_tier_defaults.heavy",
    "fast_mode.routing_tier_defaults.light",
    "fast_mode.routing_tier_defaults.standard",
    "parallelization.enabled",
    "parallelization.max_concurrent_agents",
    "parallelization.min_plans_for_parallel",
    "parallelization.plan_level",
    "parallelization.skip_checkpoints",
    "parallelization.task_level",
    "worktree.baseRef",
];

/// Capability keys whose default is empty there, and a URL in the docs: the docs' default is declared.
const DOCS_DEFAULTS: &[&str] = &[
    "review.llama_cpp_host",
    "review.lm_studio_host",
    "review.ollama_host",
];

/// Search availability flags the defaults manifest gives `false`: GSD detects them from the key's presence when they
/// are unset (`config-new-project`), so a default of `false` here would read as "forced off". Declared without one.
const DETECTED_FLAGS: &[&str] = &["brave_search", "firecrawl", "exa_search"];

/// Manifest keys that are objects: they are declared through their members, not as a setting of their own.
const CONTAINERS: &[&str] = &["agent_tools", "review.max_prompt_tokens_per_reviewer"];

struct Declared {
    name: String,
    default: Option<Value>,
    keys: Vec<Vec<String>>,
}

fn declared() -> Vec<Declared> {
    let host = MockHost::new(build());
    let text = host.describe_json("settings").expect("a settings category");
    let described: Value = serde_json::from_str(&text).expect("JSON");
    described["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| item["kind"] == "setting")
        .map(|item| Declared {
            name: item["name"].as_str().expect("a name").to_string(),
            default: item.get("default").cloned(),
            keys: serde_json::from_value(item.get("keys").cloned().unwrap_or(Value::Array(vec![])))
                .expect("keys"),
        })
        .collect()
}

/// `(key, capability default)` of every capability key, and the manifest's keys: the upstream roster.
fn upstream() -> (Vec<String>, serde_json::Map<String, Value>) {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest");
    let capabilities: Value = serde_json::from_str(CAPABILITIES).expect("capabilities");
    let capabilities = capabilities.as_object().expect("an object").clone();
    let mut keys: BTreeSet<String> = manifest["validKeys"]
        .as_array()
        .expect("validKeys")
        .iter()
        .map(|k| k.as_str().expect("a key").to_string())
        .collect();
    keys.extend(capabilities.keys().cloned());
    (keys.into_iter().collect(), capabilities)
}

/// Whether `token` (`a.b`, or `a.*` / `a_*` for a prefix) names `key`.
fn names(token: &str, key: &str) -> bool {
    match token.strip_suffix('*') {
        Some(prefix) => key.starts_with(prefix),
        None => token == key,
    }
}

/// The code spans of a table cell.
fn backticked(cell: &str) -> Vec<String> {
    gsd_kit::markdown::inline_code(cell)
        .into_iter()
        .map(|(_, code)| code)
        .collect()
}

/// `(extension, tokens)` of each non-core row of the owner table.
fn other_owners() -> Vec<(String, Vec<String>)> {
    table(|t| t.header[0] == "Owner")
        .rows
        .iter()
        .filter(|cells| cells[0].starts_with("`@gsd/") && cells[0] != "`@gsd/core`")
        .map(|cells| (backticked(&cells[0])[0].clone(), backticked(&cells[1])))
        .collect()
}

/// The table of `settings.md` that `pick` takes.
fn table(pick: impl Fn(&gsd_kit::markdown::Table) -> bool) -> gsd_kit::markdown::Table {
    gsd_kit::markdown::tables(SETTINGS_MD)
        .into_iter()
        .find(pick)
        .expect("a settings.md table")
}

/// The tokens of the "Not applicable" table's first column, except the secrets row.
fn not_applicable() -> Vec<String> {
    table(|t| t.heading == "Not applicable")
        .rows
        .iter()
        .map(|cells| cells[0].as_str())
        .filter(|cell| !cell.contains("API key half"))
        .flat_map(backticked)
        .collect()
}

/// Whether the family `name` (with its key lists) matches the written `key`.
fn family_matches(family: &Declared, key: &str) -> bool {
    let name: Vec<&str> = family.name.split('.').collect();
    let key: Vec<&str> = key.split('.').collect();
    if name.len() != key.len() {
        return false;
    }
    let mut star = 0;
    name.iter().zip(&key).all(|(n, k)| {
        if *n != "*" {
            return n == k;
        }
        let allowed = &family.keys[star];
        star += 1;
        allowed.is_empty() || allowed.iter().any(|a| a == k)
    })
}

#[test]
fn every_core_key_of_the_manifest_is_declared_with_its_default() {
    let (universe, capabilities) = upstream();
    let settings = declared();
    // `profile`, the seven `specforge.` settings and the ten `specforge.surface.` ones are SpecForge's, not GSD's.
    let (ours, statics): (Vec<&Declared>, Vec<&Declared>) = settings
        .iter()
        .filter(|s| !s.name.contains('*'))
        .partition(|s| s.name == "profile" || s.name.starts_with("specforge."));
    assert_eq!(
        ours.len(),
        18,
        "profile, seven specforge. settings and ten specforge.surface. clusters"
    );
    let families: Vec<&Declared> = settings.iter().filter(|s| s.name.contains('*')).collect();
    let owners = other_owners();
    let na = not_applicable();

    let mut core = Vec::new();
    let mut unaccounted = Vec::new();
    let mut doubly = Vec::new();
    for key in &universe {
        let is_declared = statics.iter().any(|s| s.name == *key)
            || families.iter().any(|f| family_matches(f, key))
            || CONTAINERS.contains(&key.as_str());
        let owned: Vec<&str> = owners
            .iter()
            .filter(|(_, tokens)| tokens.iter().any(|t| names(t, key)))
            .map(|(owner, _)| owner.as_str())
            .collect();
        let not_applicable = na.iter().any(|t| names(t, key));
        match (is_declared, owned.len() + usize::from(not_applicable)) {
            (true, 0) => core.push(key.as_str()),
            (false, 1) => {}
            (false, 0) => unaccounted.push(key.as_str()),
            _ => doubly.push(key.as_str()),
        }
    }
    assert_eq!(unaccounted, Vec::<&str>::new(), "keys nobody owns");
    assert_eq!(doubly, Vec::<&str>::new(), "keys claimed twice");
    assert_eq!(
        (universe.len(), core.len()),
        (212, 161),
        "the roster of v{}: 212 keys, 161 of them core",
        gsd_kit::tracked_version!()
    );

    // Nothing else: every static setting is a key of the roster or listed as documented only, every family is one of
    // the manifest's dynamic patterns.
    let known: BTreeSet<&str> = universe
        .iter()
        .map(String::as_str)
        .chain(DOCS_ONLY.iter().copied())
        .collect();
    let extra: Vec<&str> = statics
        .iter()
        .map(|s| s.name.as_str())
        .filter(|name| !known.contains(name))
        .collect();
    assert_eq!(extra, Vec::<&str>::new(), "declared, but not GSD's");
    let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest");
    let patterns: Vec<&str> = manifest["dynamicKeyPatterns"]
        .as_array()
        .expect("patterns")
        .iter()
        .map(|p| p["topLevel"].as_str().expect("a top level"))
        .collect();
    for family in &families {
        let top = family.name.split('.').next().expect("a segment");
        assert!(
            patterns.contains(&top),
            "family {} is no dynamic pattern of the manifest",
            family.name
        );
    }
    assert_eq!(
        (statics.len(), families.len()),
        (core.len() - CONTAINERS.len() + DOCS_ONLY.len(), 12),
        "159 manifest and capability keys, 18 only the docs list, and 12 families"
    );

    // Defaults: a capability's own (an empty string or -1 is its "unset"), then the defaults manifest's.
    let defaults: Value = serde_json::from_str(DEFAULTS).expect("defaults");
    let mut expected: Vec<(String, Value)> = Vec::new();
    flatten("", &defaults, &mut expected);
    for (key, cap) in &capabilities {
        expected.push((key.clone(), cap["default"].clone()));
    }
    let mut checked = 0;
    for (key, value) in &expected {
        let Some(setting) = statics.iter().find(|s| s.name == *key) else {
            continue;
        };
        if DOCS_DEFAULTS.contains(&key.as_str()) {
            continue;
        }
        if DETECTED_FLAGS.contains(&key.as_str()) {
            assert_eq!(setting.default, None, "{key} is detected, not defaulted");
            continue;
        }
        checked += 1;
        let want = match value {
            Value::Null => None,
            Value::String(s) if s.is_empty() => None,
            Value::Number(n) if n.as_i64() == Some(-1) => None,
            // GSD's `false | true | "omit"` is written `aliases | ids | omit`.
            Value::Bool(false) if key == "resolve_model_ids" => Some(Value::from("aliases")),
            other => Some(other.clone()),
        };
        assert_eq!(setting.default, want, "the default of {key}");
    }
    assert!(checked >= 100, "only {checked} defaults were compared");
}

/// Flattens nested objects to dotted keys; arrays and `agent_skills` stay values.
fn flatten(prefix: &str, value: &Value, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(map) if !map.is_empty() && prefix != "agent_skills" => {
            for (k, v) in map {
                if k.starts_with('_') {
                    continue;
                }
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        _ => out.push((prefix.to_string(), value.clone())),
    }
}

/// The asset `path` under `assets/`, as text.
fn asset(path: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(path);
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

/// The plan-phase workflow, the planner and the phase researcher read no `commit_docs`: `@gsd/core` declares no such
/// setting, and the spec they record is committed like code. Step 13d commits the plans and says what its "nothing to
/// commit" answer means.
#[test]
fn the_plan_phase_family_reads_no_commit_docs() {
    for path in [
        "gsd-core/workflows/plan-phase.md",
        "agents/gsd-planner.md",
        "agents/gsd-phase-researcher.md",
    ] {
        let named: Vec<String> = asset(path)
            .lines()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains("commit_docs"))
            .map(|(n, line)| format!("{path}:{}: {line}", n + 1))
            .collect();
        assert!(named.is_empty(), "{}", named.join("\n"));
    }
    let plan_phase = asset("gsd-core/workflows/plan-phase.md");
    let lines: Vec<&str> = plan_phase.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == "## 13d. Commit Plans")
        .expect("a line `## 13d. Commit Plans`");
    let end = start
        + lines[start..]
            .iter()
            .position(|line| line.starts_with("## 13e."))
            .expect("step 13e after 13d");
    let step = lines[start..end].join("\n");
    assert!(step.contains("nothing to commit"), "{step}");
    assert!(step.contains("`specforge git commit --add spec`"), "{step}");
}

/// Every `.md` file under `assets/`, with its path under `assets/` and its text.
fn asset_texts() -> Vec<(String, String)> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    files::under(&assets)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .map(|path| {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let rel = path
                .strip_prefix(&assets)
                .expect("under assets")
                .display()
                .to_string();
            (rel, text)
        })
        .collect()
}

/// A sentence that calls a key a setting ("`a`, `b` and `c` are settings", "`a` is a setting") names only keys of
/// the roster, `gsd_core::settings::key_names()`, never a copied list: a key `@gsd/core` does not declare is read as
/// unset by one agent and as on by another. The scan reads every asset's whole text, so a claim that a line break
/// splits is still one sentence, and it sees the 13 claims the assets hold.
#[test]
fn every_key_an_asset_calls_a_setting_is_a_key_of_the_roster() {
    let roster: BTreeSet<&str> = gsd_core::settings::key_names().collect();
    let claim = regex::Regex::new(
        r"((?:`[^`]+`(?:,\s*|\s+and\s+|,\s+and\s+)?)+)\s+(?:are settings|is a setting)\b",
    )
    .expect("the claim pattern");
    let code = regex::Regex::new(r"`([^`]+)`").expect("the code pattern");
    let mut seen = 0;
    let mut false_claims = Vec::new();
    for (path, text) in asset_texts() {
        for found in claim.captures_iter(&text) {
            seen += 1;
            for key in code.captures_iter(&found[1]).map(|c| c[1].to_string()) {
                if !roster.contains(key.as_str()) {
                    false_claims.push(format!("{path}: `{key}` is called a setting"));
                }
            }
        }
    }
    assert!(false_claims.is_empty(), "{}", false_claims.join("\n"));
    assert_eq!(seen, 13, "the claims the assets hold");
}

/// The step of the settings workflow named `name`.
fn settings_step(text: &str, name: &str) -> String {
    let open = format!("<step name=\"{name}\">");
    let at = text.find(&open).unwrap_or_else(|| panic!("no step {name}"));
    let end = text[at..].find("</step>").expect("the step's end");
    text[at..at + end].to_string()
}

/// The settings workflow presents the settings of its section layout and says how many: each setting of the layout
/// names its key, a key `@gsd/core` declares or one the owner table gives another `@gsd/*` extension, never a key the
/// Not applicable table lists; the layout's names are the `header`s of the questions, every question's header is in
/// the layout, the confirmation shows a row per setting, and every count the workflow states (the settings, the
/// sections) is the layout's.
#[test]
fn the_settings_workflow_counts_the_settings_of_its_section_layout() {
    const WORDS: [&str; 11] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];
    let text = asset("gsd-core/workflows/settings.md");
    let present = settings_step(&text, "present_settings");
    let layout = present
        .split_once("Section layout")
        .expect("the section layout")
        .1;
    let layout = layout
        .split_once("**Conditional visibility")
        .expect("the layout's end")
        .0;
    let item = regex::Regex::new(r"([A-Z][A-Za-z &/-]*?) \(`([^`]+)`").expect("the item pattern");
    let mut sections = 0;
    let mut asked = BTreeSet::new();
    let mut keys = Vec::new();
    for line in layout.lines() {
        if line.starts_with("### ") {
            sections += 1;
            continue;
        }
        for found in item.captures_iter(line) {
            asked.extend(found[1].split(" / ").map(str::to_string));
            keys.push(found[2].to_string());
        }
    }
    assert_eq!(sections, 6, "{layout}");

    let roster: BTreeSet<&str> = gsd_core::settings::key_names().collect();
    let owners = other_owners();
    let na = not_applicable();
    for key in &keys {
        let owned = roster.contains(key.as_str())
            || owners
                .iter()
                .any(|(_, tokens)| tokens.iter().any(|t| names(t, key)));
        assert!(owned, "`{key}` is no setting of a `@gsd/*` extension");
        assert!(
            !na.iter().any(|t| names(t, key)),
            "`{key}` is not applicable"
        );
    }

    let header = regex::Regex::new(r#"header: "([^"]+)""#).expect("the header pattern");
    let headers: BTreeSet<String> = header
        .captures_iter(&present)
        .map(|c| c[1].to_string())
        .collect();
    assert_eq!(headers, asked, "the questions' headers and the layout");

    let confirm = settings_step(&text, "confirm");
    let rows = confirm
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Setting "))
        .count();
    assert_eq!(rows, keys.len(), "the confirmation's rows");

    let stated =
        regex::Regex::new(r"User presented with (\d+) settings").expect("the count pattern");
    let count = stated.captures(&text).expect("the stated count")[1]
        .parse::<usize>()
        .expect("a number");
    assert_eq!(count, keys.len(), "the settings the layout holds");
    let grouped =
        regex::Regex::new(r"grouped into (?:its )?(\w+) (?:visual )?sections").expect("a pattern");
    let mut seen = 0;
    for found in grouped.captures_iter(&text) {
        seen += 1;
        assert_eq!(&found[1], WORDS[sections], "{}", &found[0]);
    }
    assert_eq!(seen, 2, "the statements of the section count");
}

/// The keys of the Not applicable table's row that holds `commit_docs`, by their last name: `planning.pr_strict` is
/// `pr_strict`, `phase_commit_docs.<phase-id>` is `phase_commit_docs`.
fn committed_spec_keys() -> BTreeSet<String> {
    let row = table(|t| t.heading == "Not applicable")
        .rows
        .into_iter()
        .find(|cells| backticked(&cells[0]).iter().any(|t| t == "commit_docs"))
        .expect("the row of `commit_docs`");
    backticked(&row[0])
        .iter()
        .filter_map(|token| {
            token
                .split('.')
                .rfind(|segment| *segment != "planning" && !segment.starts_with('<'))
                .map(str::to_string)
        })
        .collect()
}

/// `commit_docs`, `search_gitignored` and `pr_strict` are not settings of `@gsd/core` (the spec is source and is
/// committed like code: nothing of it is gitignored, and a PR branch always leaves it out), so an asset names one only
/// to say so: every line of every asset that names a key of that row of the Not applicable table, in any case, says
/// "not a setting here". The row is read from `spec/settings.md`, and the lines that say it (new-project's,
/// planning-config's and pr-branch's) are seen, so the scan cannot pass on an empty tree.
#[test]
fn the_committed_spec_keys_are_named_only_as_not_a_setting_here() {
    let keys = committed_spec_keys();
    for key in [
        "commit_docs",
        "phase_commit_docs",
        "search_gitignored",
        "pr_strict",
    ] {
        assert!(keys.contains(key), "the row lacks {key}: {keys:?}");
    }
    let mut allowed = BTreeSet::new();
    let mut named = Vec::new();
    for (path, text) in asset_texts() {
        for (n, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            let hits: Vec<&String> = keys.iter().filter(|k| lower.contains(k.as_str())).collect();
            if hits.is_empty() {
                continue;
            }
            if lower.contains("not a setting here") {
                allowed.extend(hits.into_iter().cloned());
            } else {
                named.push(format!("{path}:{}: {line}", n + 1));
            }
        }
    }
    assert!(named.is_empty(), "{}", named.join("\n"));
    for key in ["commit_docs", "search_gitignored", "pr_strict"] {
        assert!(
            allowed.contains(key),
            "no line says `{key}` is not a setting here"
        );
    }
}

/// The spec is committed like code, so no asset makes its commit conditional or treats an ignored spec as a skip: no
/// line holds a phrase of that conditional ("if planning docs committed", "intentional skip", "skipped (.spec"), and
/// the executor's line on a gitignored spec calls it an error to surface.
#[test]
fn no_asset_makes_the_spec_commit_conditional_or_skips_an_ignored_spec() {
    let mut found = Vec::new();
    for (path, text) in asset_texts() {
        for (n, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if [
                "if planning docs committed",
                "intentional skip",
                "skipped (.spec",
            ]
            .iter()
            .any(|phrase| lower.contains(phrase))
            {
                found.push(format!("{path}:{}: {line}", n + 1));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    let executor = asset("agents/gsd-executor.md");
    let ignored = executor
        .lines()
        .find(|l| l.contains("`.spec` files are gitignored"))
        .expect("the executor's line on a gitignored spec");
    assert!(ignored.contains("an error, never a skip"), "{ignored}");
}
