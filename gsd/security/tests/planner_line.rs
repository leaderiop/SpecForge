//! The `specforge new threat` line core's planner prompt gives (REVIEW-security-part1 m6), checked against the kind
//! `@gsd/security` declares: core cannot see that declaration, so its own `specforge new` test skips these kinds.

use gsd_security::build;
use regex::Regex;
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

fn planner_line() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/assets/agents/gsd-planner.md");
    let text = std::fs::read_to_string(path).expect("the planner");
    text.lines()
        .find(|l| l.contains("`specforge new threat"))
        .expect("the planner's line for a threat")
        .to_string()
}

#[test]
fn the_planner_s_new_threat_line_fits_the_kind() {
    let line = planner_line();
    let call = Regex::new(r"`specforge new threat ([^`]*)`")
        .expect("a regex")
        .captures(&line)
        .unwrap_or_else(|| panic!("no command in: {line}"))[1]
        .to_string();
    let host = MockHost::new(build());
    let entities: Value =
        serde_json::from_str(&host.describe_json("entities").expect("entities")).expect("JSON");
    let threat = entities["items"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k["name"] == "threat"))
        .expect("the threat kind");
    let fields = threat["fields"].as_array().expect("fields");
    let given: Vec<&str> = call
        .split_whitespace()
        .filter_map(|w| w.strip_prefix("--"))
        .collect();
    // `--in` is the file the entity is written to, as for every kind
    assert!(given.contains(&"in"), "{call}");
    for flag in given.iter().filter(|f| **f != "in") {
        let field = fields.iter().find(|f| {
            f["name"]
                .as_str()
                .is_some_and(|n| n.replace('_', "-") == *flag)
        });
        assert!(
            field.is_some_and(|f| f["writer"].as_str().unwrap_or("user") == "user"),
            "--{flag} is not a user-written field of `threat`: {call}"
        );
    }
    for field in fields {
        let name = field["name"].as_str().unwrap_or_default();
        let required =
            field["required"] == true && field["writer"].as_str().unwrap_or("user") == "user";
        assert!(
            !required || given.contains(&name.replace('_', "-").as_str()),
            "`threat` needs the required --{name}: {call}"
        );
    }
    // the positional after the id is the title, not the component
    assert!(call.contains("\"<title>\""), "{call}");
}
