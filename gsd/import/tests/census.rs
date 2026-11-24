//! The measure of ADR 0391 D3 and D8: nothing a `.planning/` folder holds is dropped silently, and one folder gives the
//! same plan and report whatever the order its files are served in.

mod common;

use std::collections::BTreeSet;

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use common::{fact, feed, fixture, fixture_manifest, import, manifest, payload, planning_facts};
use gsd_import::command::imported;
use gsd_import::frontmatter;
use gsd_import::preflight;
use gsd_import::read::Served;
use gsd_import::table::{self, EXTRAS, FATES, Fate};

/// A path under `.planning/` for the `FATES` name `name`: each placeholder filled (`NN`, `MM` → `01`, `<version>` →
/// `1.0`, `*` → `x`), a phase file in phase 01's directory.
fn path_for(name: &str) -> String {
    let concrete = name
        .replace("NN-", "01-")
        .replace("MM-", "01-")
        .replace("<version>", "1.0")
        .replace('*', "x");
    if name.starts_with("NN") {
        format!("phases/01-core/{concrete}")
    } else {
        concrete
    }
}

/// The served facts of `.planning/` in the census tree: the taskboard's folder, the ledger's quick plan and the vault's
/// security audit (the Tier 1 files, with sources the readers map), then one placeholder for every other `FATES` name,
/// and a file no row names.
fn census_tree() -> Vec<Value> {
    let mut facts = planning_facts(&fixture("taskboard"));
    let extra = |name: &str, rel: &str| -> Vec<Value> {
        planning_facts(&fixture(name))
            .into_iter()
            .filter(|f| f["path"].as_str().is_some_and(|p| p.starts_with(rel)))
            .collect()
    };
    facts.extend(extra("ledger", ".planning/quick/"));
    facts.extend(extra("vault", ".planning/phases/01-core/01-SECURITY.md"));
    let named = |facts: &[Value], name: &str| {
        facts.iter().any(|f| {
            let rel = f["path"]
                .as_str()
                .unwrap_or_default()
                .trim_start_matches(".planning/");
            table::fate_of(rel).is_some_and(|(row, _)| row == name)
        })
    };
    for (name, _) in FATES.iter().chain(EXTRAS) {
        if !named(&facts, name) {
            let path = path_for(name);
            assert_eq!(
                table::fate_of(&path).map(|(row, _)| row),
                Some(*name),
                "the placeholder {path} must take the row {name}"
            );
            facts.push(fact(&format!(".planning/{path}"), "placeholder\n"));
        }
    }
    facts.push(fact(".planning/notes.txt", "a note\n"));
    facts
}

/// Whether `entry` is the one `emit` writes for a record it creates (`<kind> <id>` in the section `<kind>`).
fn is_record(entry: &Value) -> bool {
    let (section, reason) = (entry["section"].as_str(), entry["reason"].as_str());
    entry["outcome"] == "mapped"
        && section
            .zip(reason)
            .is_some_and(|(s, r)| r.starts_with(&format!("{s} ")))
}

#[test]
fn nothing_is_dropped_silently() {
    let facts = census_tree();
    let out = import(
        &CommandGraph::default(),
        &feed(
            facts.clone(),
            false,
            Some(&manifest(&["@gsd/core", "@gsd/security"])),
        ),
    );
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let report = payload(&out);
    let entries = report["entries"].as_array().expect("entries");
    // Every name of the table is in the tree.
    let served: Vec<&str> = facts.iter().filter_map(|f| f["path"].as_str()).collect();
    for (name, _) in FATES.iter().chain(EXTRAS) {
        assert!(
            served
                .iter()
                .any(|p| table::fate_of(p.trim_start_matches(".planning/"))
                    .is_some_and(|(row, _)| row == *name)),
            "{name}"
        );
    }
    let mut silent = Vec::new();
    for path in &served {
        let of: Vec<&Value> = entries.iter().filter(|e| e["file"] == *path).collect();
        let records = of.iter().filter(|e| is_record(e)).count();
        let whole = of
            .iter()
            .filter(|e| e["line"].is_null() && e["section"] == "")
            .count();
        if records == 0 && whole != 1 {
            silent.push(format!("{path}: {records} records, {whole} file entries"));
        }
    }
    assert!(silent.is_empty(), "{silent:#?}");
    // Each Tier 1 file of the fixtures: every heading is a record's source or an entry's line.
    for (name, security) in [("taskboard", false), ("ledger", false), ("vault", true)] {
        let facts = planning_facts(&fixture(name));
        let out = import(
            &CommandGraph::default(),
            &feed(
                facts.clone(),
                false,
                Some(&fixture_manifest(name, security)),
            ),
        );
        let report = payload(&out);
        let entries = report["entries"].as_array().expect("entries");
        for f in &facts {
            let path = f["path"].as_str().unwrap_or_default();
            let rel = path.trim_start_matches(".planning/");
            if !matches!(table::fate_of(rel), Some((_, Fate::Maps))) {
                continue;
            }
            let text = f["text"].as_str().unwrap_or_default();
            let (_, start) = frontmatter::split(text);
            let body = frontmatter::body(text, start);
            let lines: BTreeSet<u64> = entries
                .iter()
                .filter(|e| e["file"] == path)
                .filter_map(|e| e["line"].as_u64())
                .collect();
            for h in gsd_kit::markdown::headings(&body) {
                if !lines.contains(&(h.line as u64 + 1)) {
                    silent.push(format!("{name} {path}:{} {}", h.line + 1, h.text));
                }
            }
        }
    }
    assert!(silent.is_empty(), "headings with no entry: {silent:#?}");
}

/// The fixture `name`'s import through [`imported`], its served files in `order`.
fn imported_in(name: &str, security: bool, reversed: bool) -> (String, String, String) {
    let mut served: Vec<Served> = planning_facts(&fixture(name))
        .into_iter()
        .map(|f| Served {
            path: f["path"].as_str().unwrap_or_default().to_string(),
            text: f["text"].as_str().map(str::to_string),
            error: None,
        })
        .collect();
    if reversed {
        served.reverse();
    }
    let manifest = preflight::manifest(&[Served {
        path: "specforge.json".to_string(),
        text: Some(fixture_manifest(name, security)),
        error: None,
    }])
    .expect("a manifest");
    let plan = PlanBuilder::new(Default::default(), Default::default());
    let out = imported(served, &manifest, &CommandGraph::default(), plan).expect("imported");
    (
        serde_json::to_string(&out.plan.finish()).expect("JSON"),
        serde_json::to_string(&out.payload).expect("JSON"),
        out.text,
    )
}

#[test]
fn vault_owed_names_each_judged_entity() {
    let report = payload(&common::import_with("vault", true));
    let owed: Vec<String> = report["owed"]
        .as_array()
        .expect("owed")
        .iter()
        .map(|o| {
            format!(
                "{} {} {}",
                o["phase"].as_str().unwrap_or("-"),
                o["code"].as_str().unwrap_or_default(),
                o["entity"].as_str().unwrap_or_default()
            )
        })
        .collect();
    // The closed threats, the audit, and the must-haves of the two complete plans (02-01 has no summary).
    assert_eq!(
        owed,
        [
            "01 gsd.security/W302 v1.0/01/01-01/T-01-01",
            "01 gsd.security/W302 v1.0/01/01-02/T-01-03",
            "01 gsd.security/W303 v1.0/01/01-SECURITY",
            "01 gsd/E501 v1.0/01/01-01/AR1",
            "01 gsd/E501 v1.0/01/01-01/KL1",
            "01 gsd/E501 v1.0/01/01-01/TR1",
            "01 gsd/E501 v1.0/01/01-02/AR1",
            "01 gsd/E501 v1.0/01/01-02/KL1",
            "01 gsd/E501 v1.0/01/01-02/TR1",
        ]
    );
    let (_, _, text) = imported_in("vault", true, false);
    assert!(text.contains("\n  /gsd-secure-phase 01\n    specforge query 'v1.0/01/01-01/T-01-01' --inputs gsd.security/W302 --format json\n"), "{text}");
    assert!(text.contains("\n  /gsd-verify-work 01\n"), "{text}");
    assert!(
        text.contains(
            "\n  9 verdicts owed: `specforge check` exits 1 until these verdicts are recorded\n"
        ),
        "{text}"
    );
}

#[test]
fn an_import_is_deterministic() {
    for (name, security) in [("taskboard", false), ("ledger", false), ("vault", true)] {
        let (plan, json, text) = imported_in(name, security, false);
        let (plan2, json2, text2) = imported_in(name, security, true);
        assert!(plan == plan2, "{name}: the plans differ");
        assert!(json == json2, "{name}: the JSON reports differ");
        assert_eq!(text, text2, "{name}: the text reports differ");
        assert!(plan.len() > 1000 && text.contains("import: "), "{name}");
    }
}
