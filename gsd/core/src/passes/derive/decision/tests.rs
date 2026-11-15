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

/// `in_force` is the superseded set's complement, as E310 and W506 count it: a decision superseded with force is
/// out and its successor in. An agent's decision retires another agent's (D-06 by D-05), but neither a decision taken
/// with the user, settled (D-03) or not (D-07), nor a settled one of its own kind (D-09).
#[test]
fn in_force_follows_the_superseded_set() {
    let input = PassInput {
        entities: vec![
            decision("v/1/D-01", &[("origin", "context")]),
            decision("v/1/D-02", &[("origin", "context")]),
            decision("v/1/D-03", &[("origin", "context"), ("settled", "true")]),
            decision("v/1/D-04", &[("origin", "summary")]),
            decision("v/1/D-05", &[("origin", "checkpoint")]),
            decision("v/1/D-06", &[("origin", "summary")]),
            decision("v/1/D-07", &[("origin", "context")]),
            decision("v/1/D-08", &[("origin", "summary")]),
            decision("v/1/D-09", &[("origin", "summary"), ("settled", "true")]),
        ],
        edges: vec![
            supersedes("v/1/D-02", "v/1/D-01"),
            supersedes("v/1/D-04", "v/1/D-03"),
            supersedes("v/1/D-05", "v/1/D-06"),
            supersedes("v/1/D-08", "v/1/D-07"),
            supersedes("v/1/D-08", "v/1/D-09"),
        ],
        ..Default::default()
    };
    let answered = values(&input);
    assert_eq!(answered.len(), 9);
    let out: Vec<&str> = answered
        .iter()
        .filter(|v| v.value.as_bool() == Some(false))
        .map(|v| v.entity.as_str())
        .collect();
    assert_eq!(out, ["v/1/D-01", "v/1/D-06"]);
}
