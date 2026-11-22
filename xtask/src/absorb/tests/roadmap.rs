//! A release becomes a version milestone and each milestone it lists a phase under it, keyed by its path, its
//! references written through the id map; a requirement no written phase lists is `v2` (ADR 0372 D3, D4, D7).

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

use super::super::ledger::Why;
use super::super::plan::{self, Planned};
use super::super::read::OldEntity;
use super::super::table;
use super::{create, field, field_of, ledgered, many, old, one, path_of, paths, planned, stops};

/// `e` written at `line` of `file`.
fn at(mut e: OldEntity, file: &str, line: usize) -> OldEntity {
    e.file = file.to_string();
    e.line = line;
    e
}

fn release(id: &str, version: &str, milestones: &[&str]) -> OldEntity {
    old(
        "release",
        id,
        json!({"version": version, "status": "planned", "milestones": milestones}),
    )
}

fn milestone(id: &str, line: usize, more: Value) -> OldEntity {
    let mut fields = json!({"description": format!("The {id} cut"), "status": "planned",
                            "exit_criteria": ["One", "Two"]});
    for (k, v) in more.as_object().unwrap() {
        fields[k] = v.clone();
    }
    at(old("milestone", id, fields), "spec/milestones.spec", line)
}

fn why_of(planned: &Planned, id: &str, field: &str) -> Vec<Why> {
    ledgered(planned, id)
        .into_iter()
        .filter(|(f, _)| f == field)
        .map(|(_, why)| why)
        .collect()
}

/// The title the create of `path` writes: a milestone's and a phase's one name (ADR 0387).
fn title_of<'a>(planned: &'a Planned, path: &str) -> &'a str {
    planned
        .creates
        .iter()
        .find_map(|edit| match edit {
            specforge_ops::authoring::edit::Edit::Create { title, .. } if path_of(edit) == path => {
                Some(title.as_str())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no create of {path}"))
}

/// The dropped entries of `id`'s `field`: (value, reason), a text value as itself.
fn dropped(planned: &Planned, id: &str, field: &str) -> Vec<(String, String)> {
    planned
        .ledger
        .entries()
        .filter(|e| e.id == id && e.field == field)
        .filter_map(|e| match &e.why {
            Why::Dropped(reason) => {
                let value = e
                    .value
                    .as_str()
                    .map_or_else(|| e.value.to_string(), String::from);
                Some((value, reason.clone()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_release_becomes_a_version_milestone_named_by_its_version() {
    let planned = planned(&[
        old(
            "release",
            "first",
            json!({
                "version": "0.1.0", "description": "Usable every day.", "changelog": "Entries are saved.",
                "status": "in_progress", "deliverables": ["store_cli"], "owner": "Alice Martin",
                "tags": ["core"], "target_date": "2026-12-01",
            }),
        ),
        old("release", "second", json!({"version": "v2.0"})),
    ]);
    assert_eq!(paths(&planned), ["v0.1.0", "v2.0"], "no second `v`");
    let (kind, fields) = create(&planned, "v0.1.0");
    assert_eq!(kind, "milestone");
    assert_eq!(
        fields,
        &BTreeMap::from([
            ("goal".to_string(), one("Usable every day.")),
            ("delivered".to_string(), one("Entries are saved.")),
        ])
    );
    assert_eq!(
        title_of(&planned, "v0.1.0"),
        "The first",
        "no field repeats the title"
    );
    let check = planned.checks.iter().find(|c| c.id == "first").unwrap();
    assert_eq!(
        (
            check.new_id.as_str(),
            check.new_field,
            check.expected.clone()
        ),
        ("v0.1.0", "state", Some(json!("in_progress")))
    );
    for name in ["deliverables", "owner", "tags", "target_date"] {
        let entries = dropped(&planned, "first", name);
        assert!(
            matches!(entries.as_slice(), [(_, reason)] if !reason.trim().is_empty()),
            "{name}: {entries:?}"
        );
    }
    assert!(
        ledgered(&planned, "first").iter().all(|(f, _)| ![
            "version",
            "description",
            "changelog",
            "status"
        ]
        .contains(&f.as_str())),
        "the version is the id, and the rest are written: {:?}",
        ledgered(&planned, "first")
    );
}

#[test]
fn a_listed_milestone_becomes_a_phase_numbered_in_source_order_under_its_release() {
    let mut third = milestone("third", 30, json!({}));
    third.title = Some("Phase 3: Third Cut".to_string());
    let planned = planned(&[
        release("one", "0.1.0", &["second", "first"]),
        release("two", "0.2.0", &["fourth", "third"]),
        milestone("first", 10, json!({})),
        milestone("second", 20, json!({})),
        third,
        milestone("fourth", 40, json!({})),
    ]);
    assert_eq!(
        paths(&planned),
        [
            "v0.1.0",
            "v0.2.0",
            "v0.1.0/01",
            "v0.1.0/02",
            "v0.2.0/01",
            "v0.2.0/02"
        ]
    );
    let ids: Vec<(&str, &str, &str)> = planned
        .ledger
        .ids()
        .map(|r| (r.kind.as_str(), r.old.as_str(), r.new.as_str()))
        .collect();
    assert!(
        ids.contains(&("milestone", "third", "v0.2.0/01")),
        "{ids:?}"
    );
    let Some(specforge_ops::authoring::edit::Edit::Create { id, title, .. }) = planned
        .creates
        .iter()
        .find(|edit| path_of(edit) == "v0.2.0/01")
    else {
        panic!("no create of v0.2.0/01")
    };
    assert_eq!(
        id, "01",
        "a name holds no `/`: the milestone field places it"
    );
    assert_eq!(
        title, "Third Cut",
        "a phase is titled by its name: GSD prints its number from its id"
    );
    assert_eq!(
        ledgered(&planned, "third")
            .into_iter()
            .filter(|(f, _)| f == "title")
            .collect::<Vec<_>>(),
        [(
            "title".to_string(),
            Why::Dropped(plan::NUMBERED.to_string())
        )],
        "the old title's `Phase 3:` is not written: GSD's label prints the number from the id"
    );
    assert!(
        why_of(&planned, "first", "title").is_empty(),
        "a title with no `Phase N:` is written as itself"
    );
    let (kind, fields) = create(&planned, "v0.2.0/01");
    assert_eq!(kind, "phase");
    assert_eq!(
        fields,
        &BTreeMap::from([
            ("milestone".to_string(), one("v0.2.0")),
            ("slug".to_string(), one("third-cut")),
            ("goal".to_string(), one("The third cut")),
            ("success_criteria".to_string(), many(&["One", "Two"])),
        ])
    );
    assert_eq!(
        why_of(&planned, "third", "slug"),
        [Why::Filled {
            field: "phase.slug".to_string(),
            value: json!("third-cut")
        }]
    );
    assert_eq!(
        title_of(&planned, "v0.1.0/01"),
        "The first",
        "a title with no `Phase N:` is written as itself"
    );
    assert_eq!(
        field(&planned, "v0.1.0/01", "name"),
        None,
        "no field repeats the title"
    );
}

#[test]
fn a_completed_milestone_checks_its_phase_status_by_its_path() {
    let planned = planned(&[
        release("one", "0.1.0", &["first"]),
        milestone("first", 10, json!({"status": "completed"})),
    ]);
    let check = planned.checks.iter().find(|c| c.id == "first").unwrap();
    assert_eq!(
        (
            check.new_id.as_str(),
            check.new_field,
            check.expected.clone()
        ),
        ("v0.1.0/01", "status", Some(json!("complete")))
    );
}

#[test]
fn a_milestone_no_converted_release_lists_is_dropped_whole_with_its_reason() {
    let planned = planned(&[
        release("one", "0.1.0", &["first"]),
        milestone("groundwork", 5, json!({"tags": ["h1"]})),
        milestone("first", 10, json!({"depends_on": ["groundwork"]})),
    ]);
    assert_eq!(paths(&planned), ["v0.1.0", "v0.1.0/01"]);
    let whole = Why::Dropped(table::NO_RELEASE.to_string());
    assert_eq!(
        ledgered(&planned, "groundwork"),
        ["description", "exit_criteria", "status", "tags", "title"]
            .map(|f| (f.to_string(), whole.clone()))
    );
    assert_eq!(field(&planned, "v0.1.0/01", "depends_on"), None);
    let entries = dropped(&planned, "first", "depends_on");
    assert!(
        matches!(entries.as_slice(), [(value, reason)]
            if value == "groundwork" && reason.contains("drops whole")),
        "{entries:?}"
    );
}

#[test]
fn a_phase_lists_its_requirements_and_depends_only_on_an_earlier_phase_of_its_milestone() {
    let planned = planned(&[
        release("one", "0.1.0", &["first", "second", "third"]),
        release("two", "0.2.0", &["fourth"]),
        old(
            "feature",
            "store",
            json!({"problem": "P", "status": "done"}),
        ),
        old("journey", "browse", json!({})),
        milestone("first", 10, json!({})),
        milestone(
            "second",
            20,
            json!({"features": ["store", "browse", "elsewhere"],
                   "depends_on": ["first", "third", "fourth", "ghost"]}),
        ),
        milestone("third", 30, json!({})),
        milestone("fourth", 40, json!({})),
    ]);
    assert_eq!(
        field(&planned, "v0.1.0/02", "requirements"),
        Some(&many(&["FEATURE-01"])),
        "the feature store, through the id map"
    );
    assert_eq!(
        field(&planned, "v0.1.0/02", "depends_on"),
        Some(&many(&["v0.1.0/01"]))
    );
    let reasons: BTreeMap<String, String> = dropped(&planned, "second", "depends_on")
        .into_iter()
        .collect();
    assert_eq!(
        reasons.keys().map(String::as_str).collect::<Vec<_>>(),
        ["fourth", "ghost", "third"]
    );
    assert!(reasons["third"].contains("not an earlier one"));
    assert!(reasons["fourth"].contains("another version milestone"));
    assert!(reasons["ghost"].contains("outside the selection"));
    let reasons: BTreeMap<String, String> = dropped(&planned, "second", "features")
        .into_iter()
        .collect();
    assert!(
        reasons["browse"].contains("journey the run drops whole"),
        "{reasons:?}"
    );
    assert!(reasons["elsewhere"].contains("outside the selection"));
    assert_eq!(reasons.len(), 2, "nothing is stubbed");
}

#[test]
fn a_milestone_with_no_exit_criteria_takes_its_description_as_its_one_criterion() {
    let mut bare = milestone("bare", 20, json!({}));
    bare.fields.remove("exit_criteria");
    bare.fields.remove("description");
    let mut told = milestone("told", 10, json!({}));
    told.fields.remove("exit_criteria");
    let planned = planned(&[release("one", "0.1.0", &["told", "bare"]), told, bare]);
    assert_eq!(
        field(&planned, "v0.1.0/01", "success_criteria"),
        Some(&many(&["The told cut"]))
    );
    assert_eq!(
        why_of(&planned, "told", "success_criteria"),
        [Why::Filled {
            field: "phase.success_criteria".to_string(),
            value: json!(["The told cut"])
        }]
    );
    let fields = create(&planned, "v0.1.0/02").1;
    assert!(
        !fields.contains_key("goal") && !fields.contains_key("success_criteria"),
        "nothing fills them: the run's missing-value stop names v0.1.0/02: {fields:?}"
    );
    assert_eq!(why_of(&planned, "bare", "success_criteria"), []);
}

/// A phase name with no ASCII letter or digit gives an empty slug: nothing is filled, so the run's missing-value stop
/// names the phase, never a phase written with an empty slug.
#[test]
fn a_name_with_no_ascii_letter_fills_no_slug() {
    let mut first = milestone("first", 10, json!({}));
    first.title = Some("第一段".to_string());
    let planned = planned(&[release("one", "0.1.0", &["first"]), first]);
    let fields = create(&planned, "v0.1.0/01").1;
    assert_eq!(title_of(&planned, "v0.1.0/01"), "第一段");
    assert!(!fields.contains_key("slug"), "{fields:?}");
    assert_eq!(why_of(&planned, "first", "slug"), []);
}

/// A phase's title drops every leading number `gsd/W109` flags (ADR 0387 D3), each ledgered `dropped`, so a converted
/// project is never W109; a title that held only a number is filled with the old id, as a missing one is; a number
/// inside a title, or one outside the label grammar, is kept.
#[test]
fn a_phase_title_drops_every_number_w109_flags() {
    let cases = [
        ("a", "Phase 2.1: Hotfix", "Hotfix", Some("Phase 2.1")),
        ("b", "Phase 3 - Listing", "Listing", Some("Phase 3")),
        ("c", "phase 4: Sharing", "Sharing", Some("phase 4")),
        ("d", "PHASE 12A: Split", "Split", Some("PHASE 12A")),
        ("e", "03: Core", "Core", Some("03")),
        ("f", "3. Core", "Core", Some("3")),
        ("g", "2) Search", "Search", Some("2")),
        ("h", "04 - Export", "Export", Some("04")),
        ("i", "Phase 5", "i", Some("Phase 5")),
        ("j", "HTTP 2 support", "HTTP 2 support", None),
        ("k", "3-way merge", "3-way merge", None),
    ];
    let mut old: Vec<OldEntity> = cases
        .iter()
        .enumerate()
        .map(|(n, (id, title, _, _))| {
            let mut e = milestone(id, 10 * (n + 1), json!({}));
            e.title = Some((*title).to_string());
            e
        })
        .collect();
    let ids: Vec<&str> = cases.iter().map(|(id, ..)| *id).collect();
    old.insert(0, release("one", "0.1.0", &ids));
    let planned = planned(&old);
    for (n, (id, written, title, number)) in cases.iter().enumerate() {
        let path = format!("v0.1.0/{:02}", n + 1);
        assert_eq!(title_of(&planned, &path), *title, "{written}");
        let numbers: Vec<String> = dropped(&planned, id, "title")
            .into_iter()
            .map(|(value, reason)| {
                assert_eq!(reason, plan::NUMBERED, "{written}");
                value
            })
            .collect();
        assert_eq!(
            numbers,
            number.map(String::from).into_iter().collect::<Vec<_>>(),
            "{written}"
        );
    }
    assert_eq!(
        why_of(&planned, "i", "title"),
        [
            Why::Dropped(plan::NUMBERED.to_string()),
            Why::Filled {
                field: "phase.title".to_string(),
                value: json!("i")
            }
        ],
        "a title that held only a number is filled with the old id"
    );
}

/// `how::numbered` and `how::slug` are copies of `@gsd/core`'s, which no crate outside `gsd/` may depend on: each
/// function's text is the original's, so the converter drops exactly what `gsd/W109` flags and names a directory as
/// `phase` does.
#[test]
fn the_copied_grammar_is_the_gsd_original() {
    let read = |path: &str| {
        std::fs::read_to_string(format!("{}/../{path}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|e| panic!("{path}: {e}"))
    };
    // the text of `fn name` up to its closing brace at the start of a line, its visibility dropped
    let body = |text: &str, name: &str| -> String {
        let start = text
            .find(&format!("fn {name}("))
            .unwrap_or_else(|| panic!("no fn {name}"));
        let end = start + text[start..].find("\n}\n").expect("a closing brace");
        text[start..end].to_string()
    };
    let how = read("xtask/src/absorb/how.rs");
    for (path, names) in [
        (
            "gsd/core/src/passes/roadmap/titles.rs",
            &["numbered", "worded", "bare", "grammar"][..],
        ),
        ("gsd/core/src/branch.rs", &["slug"][..]),
    ] {
        let original = read(path);
        for name in names {
            assert_eq!(
                body(&how, name),
                body(&original, name),
                "{name} drifted from {path}"
            );
        }
    }
}

#[test]
fn when_a_phase_is_written_a_requirement_no_phase_lists_is_v2() {
    let requirements = [
        old(
            "feature",
            "listed",
            json!({"problem": "P", "status": "done"}),
        ),
        old(
            "feature",
            "unlisted",
            json!({"problem": "P", "status": "done"}),
        ),
        old(
            "feature",
            "gone",
            json!({"problem": "P", "status": "deprecated", "reason": "R"}),
        ),
        old("constraint", "bound", json!({"description": "D"})),
    ];
    let mut entities = requirements.to_vec();
    entities.push(release("one", "0.1.0", &["first"]));
    entities.push(milestone("first", 10, json!({"features": ["listed"]})));
    let planned = planned(&entities);
    for (kind, id, tier) in [
        ("feature", "listed", "v1"),
        ("feature", "unlisted", "v2"),
        ("feature", "gone", "out_of_scope"),
        ("constraint", "bound", "v2"),
    ] {
        assert_eq!(
            field_of(&planned, kind, id, "tier"),
            Some(&one(tier)),
            "{id}"
        );
    }
    let v2 = Why::Filled {
        field: "requirement.tier".to_string(),
        value: json!("v2"),
    };
    for id in ["unlisted", "bound"] {
        assert_eq!(
            why_of(&planned, id, "tier"),
            std::slice::from_ref(&v2),
            "{id}"
        );
    }
    assert_eq!(why_of(&planned, "listed", "tier"), []);

    let alone = super::planned(&requirements);
    for (kind, id, tier) in [
        ("feature", "listed", "v1"),
        ("feature", "unlisted", "v1"),
        ("feature", "gone", "out_of_scope"),
        ("constraint", "bound", "v1"),
    ] {
        assert_eq!(field_of(&alone, kind, id, "tier"), Some(&one(tier)), "{id}");
    }
    assert!(
        alone.ledger.entries().all(|e| e.why != v2),
        "no phase, no tier by the rule"
    );
}

/// The ledger's (kind, why) entries of the old (`kind`, `id`)'s `field`.
fn why_of_kind(planned: &Planned, kind: &str, id: &str, field: &str) -> Vec<Why> {
    planned
        .ledger
        .entries()
        .filter(|e| e.kind == kind && e.id == id && e.field == field)
        .map(|e| e.why.clone())
        .collect()
}

/// A requirement several written phases list is in one phase, the first in source order that lists it; each later
/// listing is ledgered `dropped`, naming that phase, so `gsd/E101` holds and the requirement keeps its tier (ADR 0372
/// D7).
#[test]
fn a_requirement_several_phases_list_is_kept_by_the_first_in_source_order() {
    let planned = planned(&[
        release("one", "0.1.0", &["first", "second"]),
        release("two", "0.2.0", &["third"]),
        old(
            "feature",
            "store",
            json!({"problem": "P", "status": "done"}),
        ),
        milestone("third", 5, json!({"features": ["store"]})),
        milestone("first", 10, json!({"features": ["store"]})),
        milestone("second", 20, json!({"features": ["store"]})),
    ]);
    assert_eq!(
        field(&planned, "v0.2.0/01", "requirements"),
        Some(&many(&["FEATURE-01"])),
        "third is first in source order"
    );
    for phase in ["v0.1.0/01", "v0.1.0/02"] {
        assert_eq!(field(&planned, phase, "requirements"), None, "{phase}");
    }
    for id in ["first", "second"] {
        let entries = dropped(&planned, id, "features");
        assert!(
            matches!(entries.as_slice(), [(value, reason)]
                if value == "store" && reason.contains("v0.2.0/01") && reason.contains("gsd/E101")),
            "{id}: {entries:?}"
        );
    }
    assert_eq!(
        field_of(&planned, "feature", "store", "tier"),
        Some(&one("v1"))
    );
    assert_eq!(why_of(&planned, "store", "tier"), []);
}

/// Two old kinds sharing an id: a milestone's `features` names the feature, so its phase lists the feature's
/// requirement, and the invariant of the same id, which no phase lists, is `v2` (ADR 0372 D3, D7). A compiled source
/// with such a pair is refused before the plan (its graph drops the second declaration, W060), but the plan keys every
/// listing by the new path all the same.
#[test]
fn a_listing_names_the_kind_its_field_references_when_two_kinds_share_an_id() {
    let planned = planned(&[
        old("invariant", "x", json!({"guarantee": "G"})),
        old("feature", "x", json!({"problem": "P", "status": "done"})),
        release("one", "0.1.0", &["first"]),
        milestone("first", 10, json!({"features": ["x"]})),
    ]);
    assert_eq!(
        field(&planned, "v0.1.0/01", "requirements"),
        Some(&many(&["FEATURE-01"]))
    );
    assert_eq!(field_of(&planned, "feature", "x", "tier"), Some(&one("v1")));
    assert_eq!(
        field_of(&planned, "invariant", "x", "tier"),
        Some(&one("v2"))
    );
    assert_eq!(why_of_kind(&planned, "feature", "x", "tier"), []);
    assert_eq!(
        why_of_kind(&planned, "invariant", "x", "tier"),
        [Why::Filled {
            field: "requirement.tier".to_string(),
            value: json!("v2")
        }]
    );
}

#[test]
fn the_id_map_is_total_one_to_one_and_repeatable() {
    let entities = [
        release("one", "0.1.0", &["first", "second"]),
        release("two", "0.2.0", &["third"]),
        milestone("first", 10, json!({"features": ["store"]})),
        milestone("second", 20, json!({"depends_on": ["first"]})),
        milestone("third", 30, json!({})),
        milestone("groundwork", 40, json!({})),
        old(
            "feature",
            "store",
            json!({"problem": "P", "status": "done"}),
        ),
        old(
            "decision",
            "parser",
            json!({"status": "accepted", "decision": "D"}),
        ),
        old("journey", "browse", json!({})),
    ];
    let first = planned(&entities);
    let renamed: Vec<(String, String)> = first
        .ledger
        .ids()
        .map(|r| (r.kind.clone(), r.new.clone()))
        .collect();
    let created: BTreeSet<(String, String)> = first
        .creates
        .iter()
        .map(|edit| {
            let specforge_ops::authoring::edit::Edit::Create { kind, .. } = edit else {
                panic!("a create")
            };
            (kind.clone(), super::path_of(edit))
        })
        .collect();
    let map: Vec<(String, String)> = first
        .ids
        .entries()
        .map(|(_, _, new)| (new.kind.to_string(), new.path()))
        .collect();
    assert_eq!(renamed.len(), created.len(), "every create has one entry");
    assert_eq!(
        map.iter().cloned().collect::<BTreeSet<_>>(),
        created,
        "every entry one create, distinct within its kind"
    );
    let ids: Vec<(&str, &str, &str)> = first
        .ledger
        .ids()
        .map(|r| (r.kind.as_str(), r.old.as_str(), r.new.as_str()))
        .collect();
    assert_eq!(
        ids,
        [
            ("decision", "parser", "D-01"),
            ("feature", "store", "FEATURE-01"),
            ("milestone", "first", "v0.1.0/01"),
            ("milestone", "second", "v0.1.0/02"),
            ("milestone", "third", "v0.2.0/01"),
            ("release", "one", "v0.1.0"),
            ("release", "two", "v0.2.0"),
        ],
        "sorted by old kind and id; the dropped milestone and the journey are not in it"
    );
    let second = planned(&entities);
    assert_eq!(first.ids, second.ids);
    assert_eq!(first.ledger.to_json("p"), second.ledger.to_json("p"));
}

#[test]
fn two_releases_of_one_version_stop_the_run_naming_both() {
    let stops = stops(&[
        release("one", "0.1.0", &[]),
        release("again", "v0.1.0", &[]),
    ]);
    let named: Vec<String> = stops.iter().map(ToString::to_string).collect();
    assert_eq!(
        named,
        ["'v0.1.0': two entities given one milestone id, by release 'one' and release 'again'"]
    );
}

#[test]
fn a_milestone_two_releases_list_stops_the_run_naming_both() {
    let stops = stops(&[
        release("one", "0.1.0", &["shared"]),
        release("two", "0.2.0", &["shared"]),
        milestone("shared", 10, json!({})),
    ]);
    let named: Vec<String> = stops.iter().map(ToString::to_string).collect();
    assert_eq!(
        named,
        [
            "milestone 'shared': listed by 2 releases, 'one' and 'two': a phase sits under one version milestone"
        ]
    );
}

/// A version the target refuses as an id is no stop here: the run's refused-id check names it before any write.
#[test]
fn a_release_version_is_the_milestone_id_as_written() {
    let planned = planned(&[old("release", "odd", json!({"version": "1.0 beta"}))]);
    assert_eq!(paths(&planned), ["v1.0 beta"]);
    assert_eq!(title_of(&planned, "v1.0 beta"), "The odd");
}

/// A scheme reference in a written list cannot be written (bare, it names a `ref`, which a typed edit cannot create):
/// it is dropped with its reason, and the rest of the list is written (ADR 0372 D8).
#[test]
fn a_scheme_reference_in_a_written_list_is_dropped() {
    let planned = planned(&[
        release("one", "0.1.0", &["first"]),
        milestone(
            "first",
            10,
            json!({"exit_criteria": ["gh.issue:42", "Entries are saved."]}),
        ),
    ]);
    assert_eq!(
        field(&planned, "v0.1.0/01", "success_criteria"),
        Some(&many(&["Entries are saved."]))
    );
    assert_eq!(
        why_of(&planned, "first", "exit_criteria"),
        [Why::Dropped(super::super::how::SCHEME_REF.to_string())]
    );
    assert!(
        planned
            .ledger
            .entries()
            .all(|e| !matches!(e.why, Why::Unconverted(_))),
        "no entry is unconverted"
    );
}
