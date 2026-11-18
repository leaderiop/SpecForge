//! A name is unique within its scope (ADR 0284): what the read and edit operations take and write.

use std::collections::BTreeMap;

use specforge_ops::OpErrorKind;
use specforge_ops::authoring::edit::{self, Edit, Input};
use specforge_ops::authoring::rename;
use specforge_ops::read::inspect::inspect;
use specforge_ops::read::navigate::Navigator;
use specforge_ops::testing::Viewed;
use specforge_ops::view::{EntityField, ProjectView};
use specforge_project::testing::Assembled;
use specforge_test::prelude::*;

/// Phases `03` and `04` of milestone `v1.0`, a decision `D-01` in each and a plan `03-02` in phase 03 that links
/// the one of its phase.
const TWO_PHASES: &str = "\
milestone v1.0 \"One\" {
}

phase 03 \"Three\" {
  milestone v1.0
}

phase 04 \"Four\" {
  milestone v1.0
}

decision D-01 \"Choice\" {
  phase v1.0/03
}

decision D-01 \"Choice\" {
  phase v1.0/04
}

decision D-02 \"Other\" {
  phase v1.0/03
}

plan 03-02 \"Plan\" {
  phase v1.0/03
  decisions [D-01]
}
";

fn project(text: &str) -> Assembled {
    Assembled::scoped().sources(&[("a.spec", text)])
}

fn set(id: &str, field: &str, value: &str) -> Edit {
    Edit::Set {
        entity_id: id.into(),
        field: field.into(),
        value: Input::One(value.into()),
        annotations: None,
    }
}

fn create(kind: &str, id: &str, scope: (&str, &str)) -> Edit {
    Edit::Create {
        kind: kind.into(),
        id: id.into(),
        title: "New".into(),
        file: "a.spec".into(),
        fields: BTreeMap::from([(scope.0.to_string(), Input::One(scope.1.into()))]),
        verify: Vec::new(),
    }
}

/// The text of `a.spec` once `edits` are planned.
fn after(project: &Assembled, edits: &[Edit]) -> String {
    let planned = edit::plan(&project.view(), edits).unwrap();
    planned
        .change()
        .after()
        .remove("a.spec")
        .expect("a.spec changes")
}

#[specforge_test(requirement = "FEATURE-064")]
fn a_surface_takes_a_short_form_and_reports_the_path() {
    let project = project(TWO_PHASES);
    let view = project.view();
    let facts = inspect(&view, "03/D-01").unwrap();
    assert_eq!(facts.node.id.raw.as_str(), "v1.0/03/D-01");
    let facts = inspect(&view, "v1.0/04/D-01").unwrap();
    assert_eq!(facts.node.id.raw.as_str(), "v1.0/04/D-01");
    let refused = inspect(&view, "D-01").map(|_| ()).unwrap_err();
    assert_eq!(refused.subject, Some(EntityField::EntityId));
    assert_eq!(refused.error.kind, OpErrorKind::InvalidInput);
    for path in ["v1.0/03/D-01", "v1.0/04/D-01"] {
        assert!(
            refused.error.message.contains(path),
            "{}",
            refused.error.message
        );
    }
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_typed_link_writes_the_shortest_spelling() {
    let project = project(TWO_PHASES);
    let link = |target: &str| Edit::Link {
        entity_id: "v1.0/03/03-02".into(),
        field: "decisions".into(),
        target: target.into(),
    };
    assert!(after(&project, &[link("v1.0/03/D-02")]).contains("decisions [D-01, D-02]"));
    assert!(after(&project, &[link("v1.0/04/D-01")]).contains("decisions [D-01, 04/D-01]"));

    let text = after(&project, &[set("v1.0/03/D-02", "phase", "04")]);
    assert!(text.contains("phase v1.0/04"), "{text}");

    let taken = edit::plan(
        &project.view(),
        &[create("decision", "D-01", ("phase", "v1.0/03"))],
    )
    .unwrap_err();
    assert_eq!(taken.error.code, "entity_exists");
    let slash = edit::plan(
        &project.view(),
        &[create("decision", "a/b", ("phase", "v1.0/03"))],
    )
    .unwrap_err();
    assert_eq!(slash.error.code, "invalid_name");
    let created = after(&project, &[create("decision", "D-03", ("phase", "03"))]);
    assert!(
        created.contains("decision D-03") && created.contains("phase v1.0/03"),
        "{created}"
    );
}

#[specforge_test(requirement = "FEATURE-111")]
fn a_rename_edits_every_segment_naming_it() {
    let source = format!(
        "{TWO_PHASES}\nplan 04-01 \"Next\" {{\n  phase v1.0/04\n  decisions [03/D-01]\n}}\n"
    );
    let project = project(&source);
    let navigator = Navigator::new(ProjectView::of(project.compiled()));
    let plan = rename::plan(&navigator, "v1.0/03", "05").unwrap();
    let text = plan.change().after().remove("a.spec").unwrap();
    assert!(text.contains("phase 05 \"Three\""), "{text}");
    assert!(text.contains("phase v1.0/05\n}"), "{text}");
    assert!(text.contains("decisions [05/D-01]"), "{text}");
    assert_eq!(text.matches("phase v1.0/04").count(), 2, "{text}");
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_move_keeps_every_reference_target() {
    let source = "\
milestone v1.0 \"One\" {
}

phase 03 \"Three\" {
  milestone v1.0
}

phase 04 \"Four\" {
  milestone v1.0
}

decision D-02 \"Other\" {
  phase v1.0/03
}

plan 03-02 \"Plan\" {
  phase v1.0/03
  decisions [D-02]
}
";
    let project = project(source);
    let text = after(&project, &[set("v1.0/03/D-02", "phase", "v1.0/04")]);
    assert!(text.contains("decisions [04/D-02]"), "{text}");

    // A plan of phase 04 writes `D-02`, which names nothing there: the move would make it name the decision.
    let source =
        format!("{source}\nplan 04-01 \"Next\" {{\n  phase v1.0/04\n  decisions [D-02]\n}}\n");
    let captured = self::project(&source);
    let refused =
        edit::plan(&captured.view(), &[set("v1.0/03/D-02", "phase", "v1.0/04")]).unwrap_err();
    assert_eq!(refused.error.code, "name_captured");
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_create_keeps_every_reference_target() {
    // A project decision D-07, named from a plan of phase 05; a phase-05 D-07 would shadow it.
    let project = project(
        "\
decision D-07 \"Project\" {
}

milestone v1.0 \"One\" {
}

phase 05 \"Five\" {
  milestone v1.0
}

plan 05-01 \"Plan\" {
  phase v1.0/05
  decisions [D-07]
}
",
    );
    let text = after(
        &project,
        &[create("decision", "D-07", ("phase", "v1.0/05"))],
    );
    assert!(text.contains("decisions [/D-07]"), "{text}");

    // A hand-written short scope field is respelled when a second phase of the same name appears.
    let project = self::project(
        "\
milestone v1.0 \"One\" {
}

milestone v2.0 \"Two\" {
}

phase 03 \"Three\" {
  milestone v1.0
}

decision D-01 \"Choice\" {
  phase 03
}
",
    );
    let text = after(&project, &[create("phase", "03", ("milestone", "v2.0"))]);
    assert!(text.contains("phase v1.0/03"), "{text}");
}

/// Phases `03` and `04` of `v1.0`, a milestone `v2.0`, and a plan of phase 03 that links `D-02`.
const REFERRED: &str = "\
milestone v1.0 \"One\" {
}

milestone v2.0 \"Two\" {
}

phase 03 \"Three\" {
  milestone v1.0
}

phase 04 \"Four\" {
  milestone v1.0
}

decision D-02 \"Other\" {
  phase v1.0/03
}

plan 03-02 \"Plan\" {
  phase v1.0/03
  decisions [D-02]
}
";

fn retitle(id: &str, title: &str) -> Edit {
    Edit::Retitle {
        entity_id: id.into(),
        title: title.into(),
    }
}

#[specforge_test(requirement = "FEATURE-131")]
fn two_scope_edits_of_one_plan_respell_against_each_other() {
    let project = project(REFERRED);
    // The decision moves to phase 04, then phase 04 to milestone v2.0: the plan's link follows both.
    let text = after(
        &project,
        &[
            set("v1.0/03/D-02", "phase", "v1.0/04"),
            set("v1.0/04", "milestone", "v2.0"),
        ],
    );
    assert!(text.contains("decisions [v2.0/04/D-02]"), "{text}");
    assert!(text.contains("phase v2.0/04\n}"), "{text}");

    // Reverse order: the phase moves first, then the decision joins it.
    let text = after(
        &project,
        &[
            set("v1.0/04", "milestone", "v2.0"),
            set("v1.0/03/D-02", "phase", "v2.0/04"),
        ],
    );
    assert!(text.contains("decisions [v2.0/04/D-02]"), "{text}");
    assert!(!text.contains("[/v2.0"), "{text}");
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_later_edit_addresses_a_moved_entity_by_its_new_path() {
    let project = project(REFERRED);
    let text = after(
        &project,
        &[
            set("v1.0/03/D-02", "phase", "v1.0/04"),
            retitle("v1.0/04/D-02", "Renamed"),
        ],
    );
    assert!(
        text.contains("decision D-02 \"Renamed\" {\n  phase v1.0/04"),
        "{text}"
    );
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_create_spells_a_reference_from_its_own_scope_chain() {
    // A project decision D-01 beside phase 03's: from a plan of phase 03, `D-01` names the phase's.
    let project = project(&format!("decision D-01 \"Project\" {{\n}}\n\n{TWO_PHASES}"));
    let created = after(
        &project,
        &[Edit::Create {
            kind: "plan".into(),
            id: "03-09".into(),
            title: "New".into(),
            file: "a.spec".into(),
            fields: BTreeMap::from([
                ("phase".to_string(), Input::One("v1.0/03".into())),
                (
                    "decisions".to_string(),
                    Input::Many(vec!["/D-01".into(), "/v1.0/03/D-02".into()]),
                ),
            ]),
            verify: Vec::new(),
        }],
    );
    assert!(created.contains("decisions [/D-01, D-02]"), "{created}");
    let linked = after(
        &project,
        &[Edit::Link {
            entity_id: "v1.0/03/03-02".into(),
            field: "decisions".into(),
            target: "/D-01".into(),
        }],
    );
    assert!(linked.contains("decisions [D-01, /D-01]"), "{linked}");
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_comma_list_reference_value_is_spelled_per_item() {
    let project = project(&format!("decision D-01 \"Project\" {{\n}}\n\n{TWO_PHASES}"));
    let created = |decisions: Input| {
        after(
            &project,
            &[Edit::Create {
                kind: "plan".into(),
                id: "03-09".into(),
                title: "New".into(),
                file: "a.spec".into(),
                fields: BTreeMap::from([
                    ("phase".to_string(), Input::One("v1.0/03".into())),
                    ("decisions".to_string(), decisions),
                ]),
                verify: Vec::new(),
            }],
        )
    };
    let items = created(Input::Many(vec!["/D-01".into(), "/v1.0/03/D-02".into()]));
    assert!(items.contains("decisions [/D-01, D-02]"), "{items}");
    assert_eq!(created(Input::One("/D-01,/v1.0/03/D-02".into())), items);
    assert_eq!(created(Input::One("/D-01, /v1.0/03/D-02".into())), items);
    // A set of the list, one comma text or its items, writes the same file as well.
    let set_to = |value: Input| {
        after(
            &project,
            &[Edit::Set {
                entity_id: "v1.0/03/03-02".into(),
                field: "decisions".into(),
                value,
                annotations: None,
            }],
        )
    };
    let items = set_to(Input::Many(vec!["/v1.0/03/D-02".into(), "/D-01".into()]));
    assert!(items.contains("decisions [D-02, /D-01]"), "{items}");
    assert_eq!(set_to(Input::One("/v1.0/03/D-02,/D-01".into())), items);
}

#[specforge_test(requirement = "FEATURE-131")]
fn a_link_after_a_create_writes_the_shortest_spelling() {
    let project = project(REFERRED);
    let text = after(
        &project,
        &[
            create("decision", "D-09", ("phase", "v1.0/03")),
            Edit::Link {
                entity_id: "v1.0/03/03-02".into(),
                field: "decisions".into(),
                target: "v1.0/03/D-09".into(),
            },
        ],
    );
    assert!(text.contains("decisions [D-02, D-09]"), "{text}");
}

#[specforge_test(requirement = "FEATURE-131")]
fn an_e002_duplicate_does_not_shift_the_edited_block() {
    // The compile reports the duplicate (the fixture reports what it is given), as an edit's baseline.
    let duplicate = specforge_common::Diagnostic::new(
        specforge_common::codes::E002,
        "duplicate entity ID 'v1.0/03/D-01' (first declared at a.spec:12:1)",
    )
    .with_span(specforge_common::SourceSpan {
        file: specforge_common::Sym::new("a.spec"),
        start_line: 16,
        start_col: 1,
        end_line: 18,
        end_col: 2,
    });
    let project = project(
        "\
milestone v1.0 \"One\" {
}

phase 03 \"Three\" {
  milestone v1.0
}

phase 04 \"Four\" {
  milestone v1.0
}

decision D-01 \"First\" {
  phase v1.0/03
}

decision D-01 \"Duplicate\" {
  phase v1.0/03
}

decision D-01 \"Target\" {
  phase v1.0/04
}
",
    )
    .reporting(vec![duplicate]);
    let text = after(&project, &[retitle("v1.0/04/D-01", "Edited")]);
    assert!(
        text.contains("decision D-01 \"Edited\" {\n  phase v1.0/04"),
        "{text}"
    );
    assert!(text.contains("\"Duplicate\""), "{text}");
}
