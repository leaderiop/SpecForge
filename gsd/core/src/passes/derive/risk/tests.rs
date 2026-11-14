use serde_json::json;

use super::*;

const NO_POST: Post = Post {
    severity: None,
    occurrence: None,
    detection: None,
};

fn sod(severity: i64, occurrence: i64, detection: i64) -> Sod {
    Sod {
        severity,
        occurrence,
        detection,
    }
}

/// A risk with its three ratings as written, and its `post` block's members when `post` is `Some`.
fn risk(id: &str, ratings: [&str; 3], post: Option<serde_json::Value>) -> PassEntity {
    let mut fields: std::collections::BTreeMap<String, String> =
        ["severity", "occurrence", "detection"]
            .into_iter()
            .zip(ratings)
            .filter(|(_, written)| !written.is_empty())
            .map(|(name, written)| (name.to_string(), written.to_string()))
            .collect();
    let mut values = std::collections::BTreeMap::new();
    if let Some(post) = post {
        let keys: Vec<&String> = post.as_object().expect("a block").keys().collect();
        fields.insert(
            "post".into(),
            keys.iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        values.insert("post".to_string(), post);
    }
    PassEntity {
        id: id.into(),
        kind: "risk".into(),
        fields,
        values,
        ..Default::default()
    }
}

fn found(values: &[PassValue], entity: &str, field: &str) -> Option<serde_json::Value> {
    values
        .iter()
        .find(|v| v.entity == entity && v.field == field)
        .map(|v| v.value.clone())
}

#[test]
fn rpn_is_the_product_of_severity_occurrence_and_detection() {
    assert_eq!(rate(sod(7, 5, 5), NO_POST).rpn, 175);
    assert_eq!(rate(sod(1, 1, 1), NO_POST).rpn, 1);
    assert_eq!(rate(sod(10, 10, 10), NO_POST).rpn, 1000);
    assert_eq!(rate(sod(7, 5, 5), NO_POST).post_rpn, 175, "no post block");
}

#[test]
fn a_post_rating_left_out_takes_its_pre_value() {
    let occurrence = Post {
        occurrence: Some(1),
        ..NO_POST
    };
    assert_eq!(rate(sod(7, 5, 5), occurrence).post_rpn, 35);
    let two = Post {
        severity: Some(3),
        detection: Some(2),
        ..NO_POST
    };
    assert_eq!(rate(sod(5, 5, 5), two).post_rpn, 30);
    assert_eq!(
        rate(sod(5, 5, 5), two).rpn,
        125,
        "the pre rpn is the pre ratings'"
    );
}

#[test]
fn a_risk_with_a_rating_missing_or_outside_one_to_ten_gets_no_value() {
    for (ratings, post) in [
        (["", "5", "5"], None),
        (["7", "5", "11"], None),
        (["0", "5", "5"], None),
        (["seven", "5", "5"], None),
        (["7", "5", "5"], Some(json!({ "occurrence": 0 }))),
        (["7", "5", "5"], Some(json!({ "detection": 11 }))),
    ] {
        let input = PassInput {
            entities: vec![risk("RK1", ratings, post.clone())],
            ..Default::default()
        };
        assert_eq!(values(&input), Vec::new(), "{ratings:?} {post:?}");
    }
}

#[test]
fn values_answers_rpn_and_post_rpn_for_each_rated_risk() {
    let input = PassInput {
        entities: vec![
            risk("RK1", ["7", "5", "5"], Some(json!({ "occurrence": 1 }))),
            risk("RK2", ["2", "3", "1"], None),
            risk("RK3", ["10", "", "7"], None),
        ],
        ..Default::default()
    };
    let out = values(&input);
    assert_eq!(found(&out, "RK1", "rpn"), Some(json!(175)));
    assert_eq!(found(&out, "RK1", "post_rpn"), Some(json!(35)));
    assert_eq!(found(&out, "RK2", "rpn"), Some(json!(6)));
    assert_eq!(found(&out, "RK2", "post_rpn"), Some(json!(6)));
    assert_eq!(
        found(&out, "RK3", "rpn"),
        None,
        "a required rating is missing"
    );
    assert_eq!(out.len(), 4);
}
