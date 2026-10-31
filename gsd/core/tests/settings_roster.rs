//! The configuration roster: every key of the tracked gsd-core is declared by `@gsd/core`, owned by another `@gsd/*`
//! extension, or not applicable, per `spec/settings.md` (parity check "roster"). The snapshot is
//! `gsd/parity/upstream/v1.19.0/`, so the check runs offline.

use std::collections::BTreeSet;

use gsd_core::build;
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

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

fn backticked(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

/// `(extension, tokens)` of each non-core row of the owner table.
fn other_owners() -> Vec<(String, Vec<String>)> {
    SETTINGS_MD
        .lines()
        .filter(|l| l.starts_with("| `@gsd/") && !l.starts_with("| `@gsd/core`"))
        .map(|line| {
            let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
            let owner = backticked(cells[0])[0].to_string();
            (
                owner,
                backticked(cells[1]).into_iter().map(String::from).collect(),
            )
        })
        .collect()
}

/// The tokens of the "Not applicable" table's first column, except the secrets row.
fn not_applicable() -> Vec<String> {
    SETTINGS_MD
        .split("\n## ")
        .find(|section| section.starts_with("Not applicable"))
        .expect("a Not applicable section")
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| GSD key") && !l.starts_with("|---"))
        .filter_map(|l| l.trim().trim_matches('|').split('|').next())
        .filter(|cell| !cell.contains("API key half"))
        .flat_map(backticked)
        .map(String::from)
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
    // `profile`, the five `specforge.` settings and the ten `specforge.surface.` ones are SpecForge's, not GSD's.
    let (ours, statics): (Vec<&Declared>, Vec<&Declared>) = settings
        .iter()
        .filter(|s| !s.name.contains('*'))
        .partition(|s| s.name == "profile" || s.name.starts_with("specforge."));
    assert_eq!(
        ours.len(),
        16,
        "profile, five specforge. settings and ten specforge.surface. clusters"
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
