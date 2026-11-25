use std::collections::BTreeSet;

use gsd_kit::files::PLANNING_FILES;

use super::*;

#[test]
fn fates_cover_planning_files() {
    let mut fated: Vec<&str> = FATES.iter().chain(EXTRAS).map(|(name, _)| *name).collect();
    let mut known: Vec<&str> = PLANNING_FILES.iter().map(|(name, _)| *name).collect();
    known.extend(["config.json", "state.json", ".gitkeep"]);
    // In the kit's order, each once.
    assert_eq!(fated, known);
    fated.sort_unstable();
    fated.dedup();
    known.sort_unstable();
    known.dedup();
    assert_eq!(
        fated.len(),
        FATES.len() + EXTRAS.len(),
        "a name given twice"
    );
    assert_eq!(fated, known);
}

#[test]
fn the_kits_reasons_are_reused() {
    let mut none = 0;
    for (name, kinds) in PLANNING_FILES {
        let (_, fate) = FATES.iter().find(|(n, _)| n == name).expect("a fate");
        if kinds.starts_with("none:") {
            none += 1;
            assert_eq!(*fate, Fate::Unmapped(KIT), "{name}");
            assert_eq!(reason(name, *fate), *kinds, "{name}");
        } else {
            assert_ne!(
                *fate,
                Fate::Unmapped(KIT),
                "{name}: the kit gives kinds, not a reason"
            );
        }
    }
    assert!(none > 0);
    // A Tier 2 name says which kinds exist.
    assert_eq!(
        reason("SKELETON.md", Fate::Unmapped(TIER_2)),
        format!("kinds {} exist; {TIER_2}", kit_kinds("SKELETON.md"))
    );
}

#[test]
fn a_section_has_one_home() {
    let mut seen = BTreeSet::new();
    for row in rows() {
        assert!(
            seen.insert((row.file, row.key)),
            "{} {} has two rows",
            row.file,
            row.key
        );
        assert!(
            !row.homes.is_empty(),
            "{} {} has no home",
            row.file,
            row.key
        );
        // STATE.md maps nothing, but its reader reports it section by section.
        assert!(
            FATES.iter().any(|(name, fate)| *name == row.file
                && (*fate == Fate::Maps
                    || (*name == "STATE.md" && matches!(fate, Fate::Runtime(_))))),
            "{}: rows of a file no reader reads",
            row.file
        );
        assert_eq!(section(row.file, row.key), Some(row));
    }
    // Every Tier 1 file has rows.
    for (name, fate) in FATES {
        if *fate == Fate::Maps {
            assert!(rows().any(|row| row.file == *name), "{name} has no rows");
        }
    }
}

#[test]
fn fate_of_classifies_a_path_under_planning() {
    assert_eq!(
        fate_of("phases/01-core/01-01-PLAN.md"),
        Some(("NN-MM-PLAN.md", Fate::Maps))
    );
    assert_eq!(
        fate_of("quick/261105-tcl-x/261105-tcl-PLAN.md"),
        Some(("quick/*", Fate::Maps))
    );
    assert!(matches!(
        fate_of("state.json"),
        Some(("state.json", Fate::Runtime(_)))
    ));
    assert!(matches!(
        fate_of("phases/03-listing/.gitkeep"),
        Some((".gitkeep", Fate::Ignored(_)))
    ));
    assert_eq!(
        fate_of("config.json"),
        Some(("config.json", Fate::Settings))
    );
    assert_eq!(
        fate_of("phases/01-core/01-SECURITY.md"),
        Some(("SECURITY.md", Fate::Maps))
    );
    assert_eq!(fate_of("notes.txt"), None);
    // GSD keeps config.json and state.json at the folder's root only.
    assert_eq!(fate_of("phases/01-core/config.json"), None);
    assert!(maps("REQUIREMENTS.md", false));
    assert!(!maps("STATE.md", true));
    // A SECURITY.md maps only when the project loads @gsd/security.
    assert!(maps("phases/01-core/01-SECURITY.md", true));
    assert!(!maps("phases/01-core/01-SECURITY.md", false));
}
