//! The profiles of `spec/settings.md` and the gates of `spec/rules.md`, against what `@gsd/core` declares.

mod common;

use std::collections::BTreeMap;

use gsd_core::build;
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

fn items() -> Vec<Value> {
    let host = MockHost::new(build());
    let text = host.describe_json("settings").expect("a settings category");
    let described: Value = serde_json::from_str(&text).expect("JSON");
    described["items"].as_array().expect("items").clone()
}

const PROFILES: [&str; 3] = ["gsd-compat", "gsd-strict", "gsd-plus"];

fn rank(letter: &str) -> u8 {
    match letter {
        "E" => 3,
        "W" => 2,
        "I" => 1,
        _ => 0,
    }
}

fn letter(severity: &str) -> &'static str {
    match severity {
        "error" => "E",
        "warning" => "W",
        _ => "I",
    }
}

/// The preset entry `profile` selects.
fn preset(items: &[Value], profile: &str) -> Value {
    items
        .iter()
        .find(|i| i["kind"] == "preset" && i["source"] == "profile")
        .expect("a profile preset")["values"][profile]
        .clone()
}

#[test]
fn profiles_match_rules_md() {
    let items = items();
    let gated: BTreeMap<String, Vec<String>> = items
        .iter()
        .filter(|i| i["kind"] == "setting")
        .filter_map(|i| {
            let gates: Vec<String> = serde_json::from_value(i["gates"].clone()).unwrap_or_default();
            (!gates.is_empty()).then(|| (i["name"].as_str().expect("a name").to_string(), gates))
        })
        .collect();
    for row in common::rules_md() {
        for (index, profile) in PROFILES.iter().enumerate() {
            let entry = preset(&items, profile);
            let raised = entry["severities"][&row.code].as_str().map(letter);
            if row.compat == "—" {
                // A SpecForge-only rule: off until its setting is on. The strict column says when.
                let setting = gated
                    .iter()
                    .find(|(_, codes)| codes.contains(&row.code))
                    .unwrap_or_else(|| panic!("{} gates nothing", row.code))
                    .0;
                let on = entry["settings"][setting].as_bool().unwrap_or(false);
                let expected = match (index, row.strict.as_str()) {
                    (0, _) => false,
                    (1, strict) => !strict.contains("plus"),
                    _ => true,
                };
                assert_eq!(on, expected, "{} under {profile}: {setting}", row.code);
                assert_eq!(raised, None, "{} is not raised under {profile}", row.code);
                continue;
            }
            let strict_letter = row
                .strict
                .chars()
                .next()
                .map(String::from)
                .unwrap_or_default();
            let wanted = if index == 0 {
                row.compat.clone()
            } else {
                strict_letter
            };
            let effective = match raised {
                Some(r) if rank(r) > rank(&row.compat) => r.to_string(),
                _ => row.compat.clone(),
            };
            assert_eq!(effective, wanted, "{} under {profile}", row.code);
        }
    }
}

/// `(setting, [compat, strict, plus])` of the "A profile presets these settings" table of `settings.md`.
fn md_presets() -> Vec<(String, [bool; 3])> {
    let md = include_str!("../spec/settings.md");
    let table = md
        .split("A profile presets these settings:")
        .nth(1)
        .expect("the presets table");
    table
        .lines()
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let cells: Vec<&str> = l
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            let on = |c: &str| c == "true";
            (
                cells[0].trim_matches('`').to_string(),
                [on(cells[1]), on(cells[2]), on(cells[3])],
            )
        })
        .collect()
}

#[test]
fn the_profile_presets_are_the_settings_md_table() {
    let items = items();
    let table = md_presets();
    assert_eq!(table.len(), 5);
    for (setting, values) in table {
        for (index, profile) in PROFILES.iter().enumerate() {
            let entry = preset(&items, profile);
            assert_eq!(
                entry["settings"][&setting].as_bool(),
                Some(values[index]),
                "{setting} under {profile}"
            );
        }
    }
}

#[test]
fn the_gates_are_the_setting_gated_rules_table() {
    let md = include_str!("../spec/rules.md");
    let section = md
        .split("\n## ")
        .find(|s| s.starts_with("Setting-gated rules"))
        .expect("the section");
    let mut from_md: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in section.lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let codes = cells[3]
            .split(", ")
            .map(|c| c.trim_start_matches("gsd/").to_string())
            .collect();
        from_md.insert(cells[0].trim_matches('`').to_string(), codes);
    }
    let from_declaration: BTreeMap<String, Vec<String>> = items()
        .iter()
        .filter(|i| i["kind"] == "setting")
        .filter_map(|i| {
            let gates: Vec<String> = serde_json::from_value(i["gates"].clone()).unwrap_or_default();
            (!gates.is_empty()).then(|| (i["name"].as_str().expect("a name").to_string(), gates))
        })
        .collect();
    assert_eq!(from_declaration, from_md);
}
