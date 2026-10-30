use super::*;

fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn input(status: &str, one_liner: &str, commit: Option<&str>) -> PassInput {
    let mut entities = vec![
        entity("v/1", "phase", &[]),
        entity("v/1/1-01", "plan", &[]),
        entity(
            "v/1/1-01/1-01-SUMMARY",
            "summary",
            &[("one_liner", one_liner)],
        ),
        entity("v/1/1-VERIFICATION", "verification", &[("status", status)]),
        entity(
            "v/1/D-03",
            "decision",
            &[
                ("origin", "context"),
                ("text", "Log every retry with its reason"),
            ],
        ),
        entity("v/1/D-04", "decision", &[("origin", "summary")]),
    ];
    if let Some(subject) = commit {
        entities.push(entity("v/1/1-01/abc", "commit", &[("subject", subject)]));
    }
    PassInput {
        entities,
        edges: vec![PassEdge {
            source: "v/1/1-VERIFICATION".into(),
            target: "v/1".into(),
            label: "phase".into(),
        }],
        ..Default::default()
    }
}

#[test]
fn a_decision_nothing_shipped_is_w506_on_the_decision() {
    let found = check(&input("passed", "Adds the store", None));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "W506");
    assert_eq!(found[0].entity.as_deref(), Some("v/1/D-03"));
}

#[test]
fn w506_a_summary_or_a_commit_that_names_the_decision_covers_it() {
    assert!(check(&input("passed", "Implements D-03", None)).is_empty());
    assert!(
        check(&input(
            "passed",
            "Adds the store",
            Some("feat(1-01): log every retry with its reason")
        ))
        .is_empty()
    );
}

#[test]
fn a_phase_not_verified_is_not_judged() {
    assert!(check(&input("gaps_found", "Adds the store", None)).is_empty());
    assert!(check(&input("pending", "Adds the store", None)).is_empty());
}
