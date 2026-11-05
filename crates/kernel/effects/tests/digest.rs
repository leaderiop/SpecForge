use specforge_effects::{
    FsOp, FsRequest, Header, HttpRequest, OutcomeClass, Redirected, Unreachable, canonical, digest,
};

#[test]
fn digest_is_canonical() {
    let request = HttpRequest {
        method: "GET".into(),
        url: "https://example.org/a\r\nb".into(),
        headers: vec![
            Header::sensitive("authorization", "Bearer hunter2"),
            Header::plain("accept", "text/html"),
        ],
        ..HttpRequest::default()
    };
    let text = String::from_utf8(canonical(&request)).unwrap();
    // Keys sorted at every level.
    assert!(text.starts_with("{\"body_cap\":"), "{text}");
    assert!(
        text.contains("{\"name\":\"accept\",\"value\":\"text/html\"}"),
        "{text}"
    );
    // A credential never reaches the canonical form, and is written redacted in Debug.
    assert!(!text.contains("hunter2"), "{text}");
    assert!(text.contains("\"value\":\"<redacted>\""), "{text}");
    assert!(!format!("{request:?}").contains("hunter2"));
    // CRLF is kept byte for byte.
    assert!(text.contains("a\\r\\nb"), "{text}");
    let digest_of = digest(&request);
    assert!(
        digest_of.starts_with("sha256:") && digest_of.len() == 7 + 64,
        "{digest_of}"
    );
    let mut other = request.clone();
    other.headers[0].value = "Bearer other".into();
    assert_eq!(
        digest(&other),
        digest_of,
        "a credential does not change the digest"
    );
    other.url.push('x');
    assert_ne!(digest(&other), digest_of);
    let fs = FsRequest {
        root: "spec".into(),
        path: "a.md".into(),
        op: FsOp::Read { max_bytes: 10 },
    };
    assert_eq!(
        String::from_utf8(canonical(&fs)).unwrap(),
        r#"{"op":{"max_bytes":10,"op":"read"},"path":"a.md","root":"spec"}"#
    );
}

#[test]
fn outcome_class_strings() {
    let classes = [
        (OutcomeClass::Ok, "ok"),
        (OutcomeClass::Status(404), "status:404"),
        (OutcomeClass::Exit(1), "exit:1"),
        (OutcomeClass::Exit(-2), "exit:-2"),
        (OutcomeClass::Signal, "signal"),
        (OutcomeClass::Deadline, "deadline"),
        (OutcomeClass::Capped, "capped"),
        (OutcomeClass::Missing, "missing"),
        (OutcomeClass::Denied, "denied"),
        (OutcomeClass::Outside, "outside"),
        (OutcomeClass::NotFound, "not_found"),
        (OutcomeClass::CannotStart, "cannot_start"),
        (
            OutcomeClass::Unreachable(Unreachable::Dns),
            "unreachable:dns",
        ),
        (
            OutcomeClass::Unreachable(Unreachable::Connect),
            "unreachable:connect",
        ),
        (
            OutcomeClass::Unreachable(Unreachable::Tls),
            "unreachable:tls",
        ),
        (
            OutcomeClass::Unreachable(Unreachable::Timeout),
            "unreachable:timeout",
        ),
        (
            OutcomeClass::Unreachable(Unreachable::Reset),
            "unreachable:reset",
        ),
        (
            OutcomeClass::Unreachable(Unreachable::Proxy),
            "unreachable:proxy",
        ),
        (
            OutcomeClass::Unreachable(Unreachable::RefusedAddress),
            "unreachable:refused_address",
        ),
        (OutcomeClass::Redirect(Redirected::Limit), "redirect:limit"),
        (
            OutcomeClass::Redirect(Redirected::Downgrade),
            "redirect:downgrade",
        ),
        (
            OutcomeClass::Redirect(Redirected::Refused),
            "redirect:refused",
        ),
    ];
    for (class, text) in classes {
        assert_eq!(class.to_string(), text);
        assert_eq!(text.parse::<OutcomeClass>(), Ok(class));
        assert_eq!(serde_json::to_value(class).unwrap(), text);
        assert_eq!(
            serde_json::from_value::<OutcomeClass>(text.into()).unwrap(),
            class
        );
    }
    for wrong in ["", "status", "status:x", "unreachable:moon", "fine"] {
        assert!(wrong.parse::<OutcomeClass>().is_err(), "{wrong}");
    }
}
