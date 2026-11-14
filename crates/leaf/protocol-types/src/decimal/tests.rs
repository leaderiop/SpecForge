use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde_json::json;

use super::*;
use crate::{GraphNode, PassEntity};

fn d(text: &str) -> Decimal {
    Decimal::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

#[test]
fn a_decimal_word_reads_its_sign_fraction_exponent_and_leading_zeros() {
    for (word, value) in [
        ("0.75", "0.75"),
        ("-0.25", "-0.25"),
        ("100", "100"),
        ("1.5e3", "1500"),
        ("2E-4", "0.0002"),
        ("007.5", "7.5"),
        ("1.50", "1.5"),
        ("-0.0", "0"),
        ("+1.5", "1.5"),
        ("1e+3", "1000"),
    ] {
        assert_eq!(d(word).to_string(), value, "{word}");
    }
}

#[test]
fn a_text_that_is_no_decimal_is_refused() {
    for text in [
        ".5", "5.", "1_000", "inf", "NaN", "0x10", "1.2.3", "1e", "--1", "", " 1", "1 ", "+", "1e-",
    ] {
        assert_eq!(
            Decimal::parse(text),
            Err(DecimalError::NotADecimal(text.to_string())),
            "{text:?}"
        );
    }
    assert_eq!(
        DecimalError::NotADecimal("abc".into()).to_string(),
        "'abc' is no decimal number"
    );
}

#[test]
fn more_than_fifteen_significant_digits_is_refused() {
    assert_eq!(
        Decimal::parse("0.1234567890123456"),
        Err(DecimalError::TooManyDigits)
    );
    assert_eq!(
        DecimalError::TooManyDigits.to_string(),
        "a number keeps at most 15 significant digits"
    );
    assert_eq!(d("123456789012345e4").to_string(), "1234567890123450000");
    assert_eq!(d("1e20").to_string(), "100000000000000000000");
    assert_eq!(d("0.123456789012345000").to_string(), "0.123456789012345");
}

#[test]
fn a_value_outside_the_normal_range_is_refused() {
    for text in ["1e308", "1e-308", "-1e308", "1e99999999999999999999"] {
        assert_eq!(
            Decimal::parse(text),
            Err(DecimalError::OutOfRange(text.to_string())),
            "{text}"
        );
    }
    assert_eq!(
        DecimalError::OutOfRange("1e308".into()).to_string(),
        "'1e308' is outside the range a number holds"
    );
    assert_eq!(d("1e307").to_f64(), 1e307);
    assert_eq!(d("1e-307").to_f64(), 1e-307);
    assert_eq!(d("0e99999999999999999999"), Decimal::ZERO);
}

#[test]
fn the_canonical_text_is_plain_with_no_trailing_zero() {
    for (word, text) in [
        ("1.50", "1.5"),
        ("1.5e3", "1500"),
        ("-0.0", "0"),
        ("1e-7", "0.0000001"),
        ("007", "7"),
        ("-2.25", "-2.25"),
        ("123456789.012345", "123456789.012345"),
        ("9.99999999999999e307", &format!("{}", 9.99999999999999e307)),
    ] {
        let value = d(word);
        assert_eq!(value.to_string(), text, "{word}");
        assert_eq!(value.to_string(), format!("{}", value.to_f64()), "{word}");
    }
}

#[test]
fn equal_values_are_equal_and_order_by_value() {
    let hash = |v: Decimal| {
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    };
    assert_eq!(d("1.5"), d("1.50"));
    assert_eq!(d("1.5"), d("15e-1"));
    assert_eq!(hash(d("1.5")), hash(d("15e-1")));
    let ascending = ["-2", "-1.5", "0", "0.001", "1e20"].map(d);
    assert!(ascending.windows(2).all(|w| w[0] < w[1]), "{ascending:?}");
    assert!(d("999999999999999") < d("1e15"));
    assert!(d("0.94999999999999") < d("0.95"));
    assert!(d("-0.95") < d("-0.94999999999999"));
    assert_eq!(d("2").cmp(&d("2.0")), Ordering::Equal);
}

#[test]
fn json_round_trips_exactly() {
    for text in [
        "0.1",
        "0.3",
        "123456789.012345",
        "1e-300",
        "-2.5",
        "1e20",
        "-9e18",
    ] {
        let value = d(text);
        assert_eq!(Decimal::from_json(&value.to_json()), Ok(value), "{text}");
    }
    assert_eq!(d("100").to_json(), serde_json::Number::from(100));
    assert_eq!(d("1.5e3").to_json(), serde_json::Number::from(1500));
    assert!(d("1e19").to_json().is_u64());
    let drift = serde_json::Number::from_f64(0.1 + 0.2).expect("finite");
    assert_eq!(Decimal::from_json(&drift), Err(DecimalError::TooManyDigits));
    assert_eq!(
        Decimal::from_json(&serde_json::Number::from(1234567890123456i64)),
        Err(DecimalError::TooManyDigits)
    );
}

#[test]
fn json_text_round_trips_exactly() {
    for text in [
        "1.602176634e-19",
        "1.5e-29",
        "1.23e-23",
        "9.99999999999999e307",
        "1e-307",
    ] {
        let value = d(text);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            serde_json::from_str::<Decimal>(&json).ok(),
            Some(value),
            "{text}"
        );
        let node: GraphNode = serde_json::from_value(json!({
            "id": "m1",
            "kind": "measure",
            "fields": serde_json::from_str::<serde_json::Value>(&format!(r#"{{"v":{json}}}"#)).unwrap(),
        }))
        .unwrap();
        assert_eq!(node.number("v"), Some(value), "{text} in a graph node");
    }
    // Every adjusted exponent the cap allows, for a short, a long and a full-width coefficient.
    for coefficient in ["15", "1602176634", "123456789012345"] {
        let digits = coefficient.len() as i32;
        for exponent in (-307 - digits + 1)..=(307 - digits + 1) {
            let value = d(&format!("{coefficient}e{exponent}"));
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(
                serde_json::from_str::<Decimal>(&json).ok(),
                Some(value),
                "{coefficient}e{exponent} written {json}"
            );
        }
    }
}

#[test]
fn serde_writes_a_plain_json_number() {
    assert_eq!(serde_json::to_string(&d("1.5")).unwrap(), "1.5");
    assert_eq!(serde_json::to_string(&d("3")).unwrap(), "3");
    assert_eq!(serde_json::from_str::<Decimal>("1.50").unwrap(), d("1.5"));
    assert!(serde_json::from_str::<Decimal>("\"1.5\"").is_err());
    assert!(serde_json::from_str::<Decimal>("0.30000000000000004").is_err());
}

#[test]
fn try_from_f64_takes_the_shortest_text() {
    assert_eq!(Decimal::try_from(95.0), Ok(d("95")));
    assert_eq!(
        Decimal::try_from(0.1 + 0.2),
        Err(DecimalError::TooManyDigits)
    );
    assert_eq!(
        Decimal::try_from(f64::NAN),
        Err(DecimalError::NotADecimal("NaN".into()))
    );
    assert!(matches!(
        Decimal::try_from(f64::INFINITY),
        Err(DecimalError::NotADecimal(_))
    ));
    assert_eq!(Decimal::try_from(-0.0), Ok(Decimal::ZERO));
    assert_eq!(Decimal::try_from(1000000000000000000i64), Ok(d("1e18")));
    assert_eq!(Decimal::from(-42), d("-42"));
}

#[test]
fn a_graph_node_and_a_pass_entity_read_a_number_field() {
    let node = GraphNode {
        id: "m1".into(),
        kind: "measure".into(),
        fields: [
            ("value".to_string(), json!(0.75)),
            ("count".to_string(), json!(1500)),
            ("name".to_string(), json!("0.75")),
            ("drift".to_string(), json!(0.1 + 0.2)),
        ]
        .into_iter()
        .collect(),
        ..GraphNode::default()
    };
    assert_eq!(node.number("value"), Some(d("0.75")));
    assert_eq!(node.number("count"), Some(d("1.5e3")));
    assert_eq!(node.number("name"), None);
    assert_eq!(node.number("drift"), None);
    assert_eq!(node.number("absent"), None);
    let entity = PassEntity {
        id: "m1".into(),
        kind: "measure".into(),
        fields: [
            ("value".to_string(), "0.75".to_string()),
            ("title".to_string(), "M".to_string()),
        ]
        .into_iter()
        .collect(),
        ..PassEntity::default()
    };
    assert_eq!(entity.number("value"), Some(d("0.75")));
    assert_eq!(entity.number("title"), None);
    assert_eq!(entity.number("absent"), None);
}
