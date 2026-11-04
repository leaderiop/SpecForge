//! Value checks on the wire (ADR 0336): a checker's declaration and a field's checks read back as written, and the
//! `checkers` describe category is read with the rest.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

#[test]
fn a_checker_and_a_field_check_round_trip() {
    use specforge_protocol_types::{CheckRef, CheckerDescriptor, FieldDescriptor, ValueForm};
    let checker: CheckerDescriptor = serde_json::from_value(serde_json::json!({
        "name": "file",
        "applies_to": ["string", "string_list"],
        "args": [{"name": "root", "field_type": "string"}],
        "scopes": ["files"],
        "codes": ["E092"],
        "value": "path",
        "root_arg": "root"
    }))
    .unwrap();
    assert_eq!(checker.value, ValueForm::Path);
    let again = serde_json::to_value(&checker).unwrap();
    assert_eq!(
        serde_json::from_value::<CheckerDescriptor>(again).unwrap(),
        checker
    );
    let text: CheckerDescriptor = serde_json::from_value(serde_json::json!({"name": "t"})).unwrap();
    assert_eq!(text.value, ValueForm::Text);
    assert_eq!(
        serde_json::to_value(&text).unwrap(),
        serde_json::json!({"name": "t", "applies_to": []})
    );

    let field = FieldDescriptor {
        name: "docs".into(),
        checks: vec![
            CheckRef::named("a"),
            CheckRef::AnyOf {
                any_of: vec![CheckRef::named("b"), CheckRef::named("c")],
            },
        ],
        ..Default::default()
    };
    let wire = serde_json::to_value(&field).unwrap();
    assert_eq!(
        serde_json::from_value::<FieldDescriptor>(wire).unwrap(),
        field
    );
}

#[test]
fn checkers_are_the_last_describe_category_and_an_unknown_key_in_one_is_reported() {
    use specforge_protocol_types::{DeclaredCategory, DescribeResponse, ExtensionDeclaration};
    assert_eq!(
        DeclaredCategory::ALL.last(),
        Some(&DeclaredCategory::Checkers)
    );
    let mut unknown = Vec::new();
    let handshake = specforge_protocol_types::HandshakeResponse {
        protocol_version: specforge_protocol_types::PROTOCOL_VERSION.to_string(),
        ..Default::default()
    };
    let declaration = ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            let items = match category {
                DeclaredCategory::Checkers => {
                    serde_json::json!([{"name": "a", "aplies_to": [], "args": [{"name": "x", "field_type": "string", "oops": 1}]}])
                }
                _ => serde_json::json!([]),
            };
            Ok(DescribeResponse {
                category: category.name().to_string(),
                items,
            })
        },
        |key| unknown.push(key.key),
    )
    .unwrap();
    assert_eq!(declaration.checkers[0].name, "a");
    assert_eq!(unknown, ["aplies_to", "oops"]);
}

#[test]
fn a_byte_range_round_trips_on_the_wire() {
    use specforge_protocol_types::{ByteRange, Probe, Root};
    let read = Probe::FsRead {
        root: Root::Project,
        path: "a.pdf".into(),
        range: ByteRange::Suffix { len: 1024 },
    };
    let wire = serde_json::to_value(&read).unwrap();
    assert_eq!(
        wire,
        serde_json::json!({
            "probe": "fs.read", "root": "project", "path": "a.pdf",
            "range": { "kind": "suffix", "len": 1024 }
        })
    );
    assert_eq!(serde_json::from_value::<Probe>(wire).unwrap(), read);
    for range in [
        ByteRange::Whole { max: 9 },
        ByteRange::Prefix { len: 4 },
        ByteRange::Span { offset: 7, len: 3 },
    ] {
        let get = Probe::HttpGet {
            url: "https://a.example/".into(),
            range,
        };
        let back: Probe = serde_json::from_value(serde_json::to_value(&get).unwrap()).unwrap();
        assert_eq!(back, get);
    }
    assert_eq!(
        ByteRange::Suffix { len: 4 }.header().as_deref(),
        Some("bytes=-4")
    );
    assert_eq!(
        ByteRange::Prefix { len: 4 }.header().as_deref(),
        Some("bytes=0-3")
    );
    let span = ByteRange::Span { offset: 7, len: 3 };
    assert_eq!(span.header().as_deref(), Some("bytes=7-9"));
    assert_eq!(ByteRange::Whole { max: 9 }.header(), None);
    assert_eq!(ByteRange::Suffix { len: 4 }.within(10), 6..10);
    assert_eq!(ByteRange::Suffix { len: 40 }.within(10), 0..10);
    assert_eq!(span.within(8), 7..8);
    assert_eq!(span.within(5), 5..5);
}

#[test]
fn a_post_query_body_is_json_of_at_most_16_kib() {
    use specforge_protocol_types::{MAX_QUERY_BYTES, Probe};
    let probe = |body: &str| {
        serde_json::from_value::<Probe>(serde_json::json!({
            "probe": "http.post_query", "url": "https://api.osv.dev/v1/query", "json_body": body
        }))
    };
    assert!(probe(r#"{"package":{"name":"serde"}}"#).is_ok());
    let large = format!("\"{}\"", "x".repeat(MAX_QUERY_BYTES));
    let over = probe(&large).unwrap_err().to_string();
    assert!(over.contains("is over 16384"), "{over}");
    let not_json = probe("package=serde").unwrap_err().to_string();
    assert!(not_json.contains("is not JSON"), "{not_json}");
}
