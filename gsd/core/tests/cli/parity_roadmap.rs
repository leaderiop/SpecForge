//! The taskboard's SpecForge twin against what gsd-tools 1.18.0 says about its `.planning/` (parity suite, "scenario"):
//! the phases `roadmap analyze` lists, the requirements `REQUIREMENTS.md` defines and the mapping `ROADMAP.md` makes.
//! A mismatch prints the GSD field and the SpecForge value.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::harness::taskboard;

fn parity(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../parity")
        .join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display()))
}

/// GSD's `2`, `02.1` as the twin writes them: the integer part on two digits.
fn padded(number: &str) -> String {
    match number.split_once('.') {
        Some((whole, part)) => format!("{whole:0>2}.{part}"),
        None => format!("{number:0>2}"),
    }
}

/// `Phase 2` -> `["02"]`; `Nothing (first phase)` -> `[]`.
fn depends(text: &str) -> Vec<String> {
    text.strip_prefix("Phase ")
        .map(|rest| {
            rest.split(',')
                .map(|n| padded(n.trim().trim_start_matches("Phase ")))
                .collect()
        })
        .unwrap_or_default()
}

/// `phase number -> requirement ids` from `ROADMAP.md`'s "Phase Details".
fn roadmap_requirements() -> BTreeMap<String, Vec<String>> {
    let roadmap = parity("fixtures/taskboard/.planning/ROADMAP.md");
    let mut found = BTreeMap::new();
    let mut phase = None;
    for line in roadmap.lines() {
        if let Some(heading) = line.strip_prefix("### Phase ") {
            phase = heading.split(':').next().map(padded);
        } else if let (Some(number), Some(ids)) = (&phase, line.strip_prefix("**Requirements**:")) {
            let ids = ids
                .trim()
                .trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty() && *id != "TBD")
                .map(String::from)
                .collect();
            found.insert(number.clone(), ids);
        }
    }
    found
}

/// `id -> v1 | v2` from `REQUIREMENTS.md`'s bold ids under the "v1" and "v2" headings.
fn defined_requirements() -> BTreeMap<String, &'static str> {
    let text = parity("fixtures/taskboard/.planning/REQUIREMENTS.md");
    let mut tier = "";
    let mut found = BTreeMap::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            tier = if heading.starts_with("v1") {
                "v1"
            } else if heading.starts_with("v2") {
                "v2"
            } else {
                ""
            };
        } else if let Some(rest) = line
            .split("**")
            .nth(1)
            .filter(|_| !tier.is_empty() && line.starts_with("- "))
        {
            found.insert(rest.to_string(), tier);
        }
    }
    found
}

fn query(p: &crate::harness::Project, args: &[&str]) -> Value {
    let mut full = vec!["query"];
    full.extend_from_slice(args);
    full.extend_from_slice(&["--format", "json"]);
    let run = p.run(&full);
    assert_eq!(run.code, 0, "{full:?}: {}", run.all());
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("{full:?} is JSON ({e}): {}", run.all()))
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn taskboard_roadmap_matches_gsd_tools() {
    let p = taskboard();
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    assert!(
        check.all().contains("0 errors, 0 warnings"),
        "check: {}",
        check.all()
    );

    let golden: Value =
        serde_json::from_str(&parity("goldens/taskboard/roadmap-analyze.json")).expect("golden");
    let mapping = roadmap_requirements();
    let graph = query(&p, &["v1.0", "--kinds", "phase", "--depth", "1"]);
    let twin: BTreeMap<String, &Value> = graph["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|n| n["kind"] == "phase")
        .map(|n| {
            (
                n["id"]
                    .as_str()
                    .expect("an id")
                    .trim_start_matches("v1.0/")
                    .to_string(),
                &n["fields"],
            )
        })
        .collect();

    let mut mismatches = Vec::new();
    let gsd_phases = golden["phases"].as_array().expect("phases");
    let gsd_numbers: Vec<String> = gsd_phases
        .iter()
        .map(|g| padded(g["number"].as_str().expect("a number")))
        .collect();
    let twin_numbers: Vec<String> = twin.keys().cloned().collect();
    if gsd_numbers != twin_numbers {
        mismatches.push(format!(
            "phases: GSD {gsd_numbers:?} != SpecForge {twin_numbers:?}"
        ));
    }
    for gsd in gsd_phases {
        let number = padded(gsd["number"].as_str().expect("a number"));
        let Some(fields) = twin.get(&number) else {
            continue;
        };
        let mut compare = |what: &str, gsd_value: String, twin_value: String| {
            if gsd_value != twin_value {
                mismatches.push(format!(
                    "phase {number} {what}: GSD {gsd_value:?} != SpecForge {twin_value:?}"
                ));
            }
        };
        compare(
            "name",
            gsd["name"].as_str().unwrap_or_default().into(),
            fields["name"].as_str().unwrap_or_default().into(),
        );
        compare(
            "goal",
            gsd["goal"].as_str().unwrap_or_default().into(),
            fields["goal"].as_str().unwrap_or_default().into(),
        );
        compare(
            "depends_on",
            format!(
                "{:?}",
                depends(gsd["depends_on"].as_str().unwrap_or_default())
            ),
            format!("{:?}", strings(&fields["depends_on"])),
        );
        compare(
            "requirements",
            format!("{:?}", mapping.get(&number).cloned().unwrap_or_default()),
            format!("{:?}", strings(&fields["requirements"])),
        );
    }

    for (id, tier) in defined_requirements() {
        let node = query(&p, &[&id, "--depth", "0"]);
        let found = node["nodes"][0]["fields"]["tier"]
            .as_str()
            .unwrap_or("(missing)")
            .to_string();
        if found != tier {
            mismatches.push(format!(
                "requirement {id} tier: GSD {tier:?} != SpecForge {found:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "the twin differs from gsd-tools:\n{}",
        mismatches.join("\n")
    );
}
