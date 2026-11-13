//! `graduate_lessons`: what `/gsd-graduation` chose for a cluster of lessons, recorded on each lesson.

use super::commands_support::*;

/// Two lessons of phase 02, as `/gsd-extract-learnings` writes them.
fn lessons() -> crate::harness::Project {
    let p = twin();
    plant(
        &p,
        "spec/lessons.spec",
        "lesson L-02-1 \"Pin the clock\" {\n  text \"Pin the clock in tests\"\n  kind lesson\n}\n\nlesson L-02-2 \"Name the store\" {\n  text \"Name the store module\"\n  kind pattern\n}\n",
    );
    p
}

const BOTH: &str = r#"["L-02-1", "L-02-2"]"#;

#[test]
fn each_disposition_is_recorded() {
    let p = lessons();
    assert_eq!(p.fields_of("L-02-1")["status"], "open");
    let run = gsd(
        &p,
        "graduate-lessons",
        &[BOTH, "--disposition", "deferred", "--until", "v1.0/03"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    for id in ["L-02-1", "L-02-2"] {
        let lesson = p.fields_of(id);
        assert_eq!(lesson["graduated"]["disposition"], "deferred", "{lesson}");
        assert_eq!(lesson["graduated"]["until"], "v1.0/03");
        assert!(
            lesson["graduated"]["at"]
                .as_str()
                .is_some_and(|t| t.ends_with('Z')),
            "{lesson}"
        );
        assert_eq!(lesson["status"], "deferred");
    }
    // a deferral to a complete phase is open again
    let run = gsd(
        &p,
        "graduate-lessons",
        &[
            r#"["L-02-2"]"#,
            "--disposition",
            "deferred",
            "--until",
            "v1.0/01",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of("L-02-2")["status"], "open");
    // a deferred lesson may be graduated again
    let run = gsd(
        &p,
        "graduate-lessons",
        &[
            r#"["L-02-1"]"#,
            "--disposition",
            "promoted",
            "--target",
            "D-01",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let lesson = p.fields_of("L-02-1");
    assert_eq!(lesson["status"], "promoted");
    assert!(
        lesson["graduated"]["target"]
            .as_str()
            .is_some_and(|t| t.ends_with("D-01")),
        "{lesson}"
    );
    assert!(lesson["graduated"].get("until").is_none(), "{lesson}");
    let run = gsd(
        &p,
        "graduate-lessons",
        &[r#"["L-02-2"]"#, "--disposition", "dismissed"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of("L-02-2")["status"], "dismissed");
    assert!(
        !p.codes().iter().any(|c| c.starts_with('E')),
        "{:?}",
        p.codes()
    );
}

#[test]
fn a_promotion_names_a_target_and_a_deferral_a_phase() {
    let p = lessons();
    let before = bytes(&p);
    let refuse = |args: &[&str], words: &[&str]| {
        refused(&gsd(&p, "graduate-lessons", args), "invalid_input", words)
    };
    refuse(&[BOTH, "--disposition", "promoted"], &["--target"]);
    refuse(&[BOTH, "--disposition", "deferred"], &["--until"]);
    refuse(
        &[BOTH, "--disposition", "dismissed", "--until", "v1.0/03"],
        &["--until", "dismissed"],
    );
    refuse(
        &[
            BOTH,
            "--disposition",
            "deferred",
            "--until",
            "v1.0/03",
            "--target",
            "D-01",
        ],
        &["--target", "deferred"],
    );
    refuse(&["[]", "--disposition", "dismissed"], &["lessons"]);
    refuse(
        &[r#"["L-02-1", "L-02-1"]"#, "--disposition", "dismissed"],
        &["twice"],
    );
    unchanged(&p, &before);
}

#[test]
fn a_promoted_or_dismissed_lesson_is_refused() {
    let p = lessons();
    assert_eq!(
        gsd(
            &p,
            "graduate-lessons",
            &[
                r#"["L-02-1"]"#,
                "--disposition",
                "promoted",
                "--target",
                "D-01"
            ]
        )
        .code,
        0
    );
    assert_eq!(
        gsd(
            &p,
            "graduate-lessons",
            &[r#"["L-02-2"]"#, "--disposition", "dismissed"]
        )
        .code,
        0
    );
    let before = bytes(&p);
    refused(
        &gsd(
            &p,
            "graduate-lessons",
            &[r#"["L-02-1"]"#, "--disposition", "dismissed"],
        ),
        "invalid_input",
        &["L-02-1", "promoted"],
    );
    refused(
        &gsd(
            &p,
            "graduate-lessons",
            &[
                r#"["L-02-2"]"#,
                "--disposition",
                "deferred",
                "--until",
                "v1.0/03",
            ],
        ),
        "invalid_input",
        &["L-02-2", "dismissed"],
    );
    unchanged(&p, &before);
}
