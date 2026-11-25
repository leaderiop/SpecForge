//! `ROADMAP.md` in its milestone-grouped form, and hostile roadmaps, through the command in process.

mod common;

use serde_json::{Value, json};
use specforge_extension_sdk::PlanOp;
use specforge_extension_sdk::prelude::*;

use common::{fact, feed, import, manifest, payload};
use gsd_import::limits::MAX_OPS;
use gsd_import::read::roadmap;
use gsd_import::records::Records;
use gsd_import::report::Report;

/// GSD's milestone-grouped roadmap (gsd-core templates/roadmap.md, "Milestone-Grouped Roadmap").
const GROUPED: &str = "# Roadmap: Shop

## Milestones

- ✅ **v1.0 MVP** - Phases 1-2 (shipped 2026-01-10)
- 🚧 **v1.1 Search** - Phase 3 (in progress)
- 📋 **v2.0 Teams** - Phase 4 (planned)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-2) - SHIPPED 2026-01-10</summary>

### Phase 1: Catalog
**Goal**: Products can be listed
**Success Criteria** (what must be TRUE):
  1. A product shows its price
**Plans**: 1 plan

Plans:
- [x] 01-01: The `catalog` model

### Phase 2: Cart
**Goal**: A cart holds products
**Depends on**: Phase 1
**Success Criteria** (what must be TRUE):
  1. A cart sums its products
**Plans**: 1 plan

</details>

### 🚧 v1.1 Search (In Progress)

**Milestone Goal:** Products can be found by name

#### Phase 3: Search
**Goal**: A word finds its products
**Depends on**: Phase 2
**Success Criteria** (what must be TRUE):
  1. A search for `mug` lists the mugs

### 📋 v2.0 Teams (Planned)

**Milestone Goal:** A team shares one shop

#### Phase 4: Teams
**Goal**: A team has members
**Depends on**: Phase 3
**Success Criteria** (what must be TRUE):
  1. An owner invites a member

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Catalog | v1.0 | 1/1 | Complete | 2026-01-10 |
";

fn run(files: Vec<Value>) -> CommandOutput {
    import(
        &CommandGraph::default(),
        &feed(files, false, Some(&manifest(&["@gsd/core"]))),
    )
}

/// `(kind, id, file, fields)` of each create.
fn creates(out: &CommandOutput) -> Vec<(String, String, String, Value)> {
    out.plan
        .as_ref()
        .expect("a plan")
        .ops
        .iter()
        .map(|op| match op {
            PlanOp::Create {
                kind,
                id,
                file,
                fields,
                ..
            } => (kind.clone(), id.clone(), file.clone(), json!(fields)),
            other => panic!("{other:?}"),
        })
        .collect()
}

/// `(line, section, outcome, reason)` of each entry about `ROADMAP.md`.
fn entries(out: &CommandOutput) -> Vec<(u64, String, String, String)> {
    payload(out)["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter(|e| e["file"] == ".planning/ROADMAP.md")
        .filter_map(|e| {
            Some((
                e["line"].as_u64()?,
                e["section"].as_str()?.to_string(),
                e["outcome"].as_str()?.to_string(),
                e["reason"].as_str()?.to_string(),
            ))
        })
        .collect()
}

#[test]
fn roadmap_grouped_gives_one_milestone_per_version_and_scopes_each_phase() {
    let out = run(vec![fact(".planning/ROADMAP.md", GROUPED)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let creates = creates(&out);
    let milestones: Vec<(&str, &str, &Value)> = creates
        .iter()
        .filter(|(k, ..)| k == "milestone")
        .map(|(_, id, file, fields)| (id.as_str(), file.as_str(), fields))
        .collect();
    assert_eq!(
        milestones,
        [
            ("v1.0", "spec/gsd/v1.0.spec", &json!({})),
            (
                "v1.1",
                "spec/gsd/v1.1.spec",
                &json!({"goal": "Products can be found by name"})
            ),
            (
                "v2.0",
                "spec/gsd/v2.0.spec",
                &json!({"goal": "A team shares one shop"})
            ),
        ]
    );
    let phases: Vec<(&str, &str, Value, Value)> = creates
        .iter()
        .filter(|(k, ..)| k == "phase")
        .map(|(_, id, file, f)| {
            (
                id.as_str(),
                file.as_str(),
                f["milestone"].clone(),
                f["depends_on"].clone(),
            )
        })
        .collect();
    assert_eq!(
        phases,
        [
            ("01", "spec/gsd/v1.0.spec", json!("v1.0"), Value::Null),
            (
                "02",
                "spec/gsd/v1.0.spec",
                json!("v1.0"),
                json!(["v1.0/01"])
            ),
            (
                "03",
                "spec/gsd/v1.1.spec",
                json!("v1.1"),
                json!(["v1.0/02"])
            ),
            (
                "04",
                "spec/gsd/v2.0.spec",
                json!("v2.0"),
                json!(["v1.1/03"])
            ),
        ]
    );
    let search = creates
        .iter()
        .find(|(k, id, ..)| k == "phase" && id == "03")
        .map(|(.., f)| f.clone())
        .expect("phase 03");
    assert_eq!(search["slug"], "search");
    assert_eq!(
        search["success_criteria"],
        json!(["A search for `mug` lists the mugs"])
    );
    let entries = entries(&out);
    let status: Vec<&(u64, String, String, String)> = entries
        .iter()
        .filter(|(_, _, o, r)| o == "computed" && r.starts_with("milestone.state"))
        .collect();
    assert_eq!(status.len(), 1, "the status is reported once: {entries:#?}");
    assert_eq!(status[0].0, 5);
    assert!(
        !payload(&out)["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .any(|e| e["reason"]
                .as_str()
                .is_some_and(|r| r.contains("(no milestone)"))),
    );
}

#[test]
fn roadmap_grouped_gives_its_plan_titles_to_the_plan_reader() {
    let mut records = Records::default();
    let mut report = Report::default();
    let read = roadmap::read(
        ".planning/ROADMAP.md",
        GROUPED,
        &[],
        &mut records,
        &mut report,
    );
    assert_eq!(
        read.plan_titles.into_iter().collect::<Vec<_>>(),
        [("01-01".to_string(), "The catalog model".to_string())]
    );
    assert_eq!(read.phases.get("03").map(String::as_str), Some("v1.1/03"));
}

#[test]
fn roadmap_hostile_headings_and_references_are_reported_at_their_lines() {
    let text = "# Roadmap: X

### Phase 1: One
**Goal**: g
**Depends on**: Phase 99
**Requirements**: [NOPE-01]
**Success Criteria**:
  1. c

### Phase 1.2.3: Bad

### Phase: Nameless

### Phase 7 Seven

### Phase 8: No goal
**Success Criteria**:
  1. c
";
    let out = run(vec![fact(".planning/ROADMAP.md", text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let phases: Vec<(String, Value)> = creates(&out)
        .into_iter()
        .filter(|(k, ..)| k == "phase")
        .map(|(_, id, _, f)| (id, f))
        .collect();
    assert_eq!(
        phases,
        [(
            "01".to_string(),
            json!({"milestone": "v1.0", "slug": "one", "goal": "g", "success_criteria": ["c"]})
        )]
    );
    let entries = entries(&out);
    let has = |line: u64, says: &str| {
        assert!(
            entries
                .iter()
                .any(|(l, _, o, r)| *l == line && o == "unmapped" && r.contains(says)),
            "{line} {says}: {entries:#?}"
        );
    };
    has(5, "`Phase 99`, which is no phase of the roadmap");
    has(6, "NOPE-01, which REQUIREMENTS.md does not define");
    has(10, "`1.2.3` is not a phase number");
    has(12, "with no number");
    has(14, "neither form");
    has(16, "required goal has no source");
}

#[test]
fn roadmap_grouped_version_heading_naming_no_version_is_reported_at_its_line() {
    let text = GROUPED
        .replace("### 🚧 v1.1 Search", "### 🚧 v1.1.x Search")
        .replace("<summary>✅ v1.0 MVP", "<summary>✅ v1.0a MVP");
    let out = run(vec![fact(".planning/ROADMAP.md", &text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let phases: Vec<String> = creates(&out)
        .into_iter()
        .filter(|(k, ..)| k == "phase")
        .map(|(_, id, ..)| id)
        .collect();
    // Phases 1 to 3 stand in no milestone; phase 4 depends on phase 3, which the roadmap does not hold.
    assert_eq!(phases, ["04"]);
    let entries = entries(&out);
    for (line, says) in [
        (
            12,
            "`v1.0a` names no version vX.Y: the phases under it are not imported",
        ),
        (
            32,
            "`v1.1.x` names no version vX.Y: the phases under it are not imported",
        ),
        (36, "a phase outside any milestone's heading"),
    ] {
        assert!(
            entries
                .iter()
                .any(|(l, _, o, r)| *l == line && o == "unmapped" && r.contains(says)),
            "{line} {says}: {entries:#?}"
        );
    }
}

#[test]
fn roadmap_any_title_and_an_unknown_section_keep_every_phase() {
    let text = "# Project Roadmap

### Phase 1: One
**Goal**: g
**Success Criteria**:
  1. c

## Roadmap Notes

Some notes.

### Phase 2: Two
**Goal**: h
**Success Criteria**:
  1. d
";
    let out = run(vec![fact(".planning/ROADMAP.md", text)]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let phases: Vec<String> = creates(&out)
        .into_iter()
        .filter(|(k, ..)| k == "phase")
        .map(|(_, id, ..)| id)
        .collect();
    assert_eq!(phases, ["01", "02"]);
    let entries = entries(&out);
    assert!(
        entries
            .iter()
            .any(|(l, _, o, r)| *l == 1 && o == "redundant" && r.contains("# Name")),
        "{entries:#?}"
    );
    assert!(
        entries
            .iter()
            .any(|(l, _, o, r)| *l == 8 && o == "unmapped" && r.contains("no row")),
        "{entries:#?}"
    );
}

#[test]
fn roadmap_past_max_ops_is_truncated() {
    let mut text = "# Roadmap: Big\n\n".to_string();
    for n in 1..=9_000 {
        text.push_str(&format!(
            "### Phase {n}: P{n}\n**Goal**: g\n**Success Criteria**:\n  1. c\n\n"
        ));
    }
    let out = run(vec![fact(".planning/ROADMAP.md", &text)]);
    assert_eq!(out.exit_code, 1, "{}", out.stdout);
    assert!(out.plan.is_none());
    assert!(
        out.stderr.contains(&format!("truncated at {MAX_OPS}")),
        "{}",
        out.stderr
    );
}

#[test]
fn roadmap_wins_over_a_traceability_row_that_disagrees() {
    let requirements = "# Requirements\n\n## v1 Requirements\n\n### Core\n- [ ] **A-01**: one\n- [ ] **A-02**: two\n\n## Traceability\n\n| Requirement | Phase | Status |\n|---|---|---|\n| A-01 | Phase 2 | Pending |\n| A-02 | Phase 1 | Pending |\n";
    let road = "# Roadmap: X\n\n### Phase 1: One\n**Goal**: g\n**Requirements**: [A-01, A-02]\n**Success Criteria**:\n  1. c\n";
    let out = run(vec![
        fact(".planning/REQUIREMENTS.md", requirements),
        fact(".planning/ROADMAP.md", road),
    ]);
    assert_eq!(out.exit_code, 0, "{}", out.stderr);
    let phase = creates(&out)
        .into_iter()
        .find(|(k, ..)| k == "phase")
        .map(|(.., f)| f)
        .expect("phase 01");
    assert_eq!(phase["requirements"], json!(["A-01", "A-02"]));
    let mismatches: Vec<(Value, String)> = payload(&out)["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .filter(|e| e["section"] == "Traceability" && e["line"].is_u64() && e["line"] != 9)
        .map(|e| {
            (
                e["line"].clone(),
                e["reason"].as_str().expect("a reason").to_string(),
            )
        })
        .collect();
    assert_eq!(
        mismatches,
        [(
            json!(13),
            "Traceability puts A-01 in `Phase 2`, the roadmap's **Requirements** in phase 01: the roadmap wins"
                .to_string()
        )]
    );
}
