use super::*;

fn decision(id: &str, fields: &[(&str, &str)]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "decision".into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn supersedes(by: &str, earlier: &str) -> PassEdge {
    PassEdge {
        source: by.into(),
        target: earlier.into(),
        label: "supersedes".into(),
    }
}

/// `in_force` is the superseded set's complement: a decision superseded with force is out, the one that supersedes it
/// is in, and a settled decision an agent's decision names stays in, as E310 and W506 count them.
#[test]
fn in_force_follows_the_superseded_set() {
    let input = PassInput {
        entities: vec![
            decision("v/1/D-01", &[("origin", "context")]),
            decision("v/1/D-02", &[("origin", "context")]),
            decision("v/1/D-03", &[("origin", "context"), ("settled", "true")]),
            decision("v/1/D-04", &[("origin", "summary")]),
        ],
        edges: vec![
            supersedes("v/1/D-02", "v/1/D-01"),
            supersedes("v/1/D-04", "v/1/D-03"),
        ],
        ..Default::default()
    };
    let in_force: Vec<(String, bool)> = values(&input)
        .into_iter()
        .map(|v| (v.entity, v.value.as_bool().expect("a bool")))
        .collect();
    assert_eq!(
        in_force,
        [
            ("v/1/D-01".to_string(), false),
            ("v/1/D-02".to_string(), true),
            ("v/1/D-03".to_string(), true),
            ("v/1/D-04".to_string(), true),
        ]
    );
}
