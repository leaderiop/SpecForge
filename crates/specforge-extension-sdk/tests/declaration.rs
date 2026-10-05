//! The SDK builds one `ExtensionDeclaration` and serves exactly it: the
//! handshake carries the meta's short name, description and keywords; a
//! raw category is checked when the extension is built; and the generated
//! guest's routing is one function, `guest_call`, an in-process host
//! reuses.

use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::testing::MockHost;
use specforge_extension_sdk::{ExtensionDeclaration, HandshakeResponse, guest_call};
use specforge_protocol_types::{DescribeResponse, SUPPORTED_CATEGORIES};

fn reports() -> ContributionsBuilder {
    let mut meta = ExtensionMeta::new("@acme/reports", "0.1.0");
    meta.short = Some("rep".to_string());
    meta.description = Some("Reports over the graph".to_string());
    meta.keywords = vec!["reports".to_string(), "graph".to_string()];
    let mut b = ContributionsBuilder::new(meta);
    b.kind("report", |k| {
        k.description("A report").field("title", |f| {
            f.field_type(FieldType::String).required();
        });
    });
    b.command("list", |c| {
        c.title("List")
            .description("List reports")
            .handler(|_| CommandOutput {
                exit_code: 0,
                stdout: "hi".to_string(),
                stderr: String::new(),
            });
    });
    b.pass("audit", |p| {
        p.phase("check")
            .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
    });
    b
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "the SDK's short name reaches the handshake as ext_short"
)]
fn the_short_name_reaches_the_handshake() {
    let host = MockHost::new(reports());
    let handshake: serde_json::Value = serde_json::from_str(&host.handshake_json()).unwrap();
    assert_eq!(handshake["ext_short"], "rep");
    assert_eq!(handshake["description"], "Reports over the graph");
    assert_eq!(
        handshake["keywords"],
        serde_json::json!(["reports", "graph"])
    );
    let declaration = host.declaration();
    assert_eq!(declaration.short(), "rep");

    // Without them, the handshake carries none of the three keys.
    let plain = ContributionsBuilder::new(ExtensionMeta::new("@acme/plain", "1.0.0"));
    let handshake: serde_json::Value = serde_json::from_str(&plain.handshake_json()).unwrap();
    for key in ["ext_short", "description", "keywords"] {
        assert!(handshake.get(key).is_none(), "{key}: {handshake}");
    }
    assert_eq!(plain.declaration().short(), "plain");
}

#[specforge_test_macros::test(
    behavior = "load_extension_declaration",
    verify = "a raw category that does not parse panics when the extension is built"
)]
fn a_raw_category_that_does_not_parse_panics() {
    let mut b = ContributionsBuilder::new(ExtensionMeta::new("@acme/raw", "1.0.0"));
    b.raw_category("passes", serde_json::json!([{ "nam": "x" }]));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.declaration()))
        .expect_err("a malformed raw category panics");
    let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(message.contains("raw category 'passes'"), "{message}");
    assert!(message.contains("missing field `name`"), "{message}");
    // Serving any category builds the declaration, so the guest fails too.
    let served = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        b.describe_response_json("entities")
    }));
    assert!(served.is_err());
}

/// What the guest serves loads back as the declaration the builder built:
/// every category, the derived `fields` and flags included.
#[test]
fn the_guest_serves_its_declaration() {
    let b = reports();
    let declared = b.declaration();
    let handshake: HandshakeResponse = serde_json::from_str(&b.handshake_json()).unwrap();
    let loaded = ExtensionDeclaration::from_wire(
        handshake,
        |category| {
            Ok(serde_json::from_str::<DescribeResponse>(
                &b.describe_response_json(category).unwrap(),
            )
            .unwrap())
        },
        |key| panic!("unexpected key {key:?}"),
    )
    .unwrap();
    assert_eq!(loaded, declared);
    assert_eq!(declared.surfaces.commands[0].id, "list");
    assert_eq!(declared.passes[0].name, "audit");
    assert!(declared.handshake.contribution_flags.entities);
    for category in SUPPORTED_CATEGORIES {
        assert_eq!(
            b.describe_response_json(category),
            declared.describe_json(category),
            "{category}"
        );
    }
    let fields: serde_json::Value =
        serde_json::from_str(&b.describe_response_json("fields").unwrap()).unwrap();
    assert_eq!(fields["items"][0]["name"], "title");
}

fn dispatch(export: &str, _input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    (export == "scan__x").then(|| Ok(b"scanned".to_vec()))
}

/// `guest_call` routes as the component guest does: the protocol exports,
/// then the declared surfaces and operations, then the handler, else an
/// unknown export.
#[test]
fn guest_call_routes_every_export() {
    let b = reports();
    let handshake = guest_call(&b, dispatch, "__handshake", b"{}").unwrap();
    assert_eq!(handshake, b.handshake_json().into_bytes());
    let describe = guest_call(&b, dispatch, "__describe", br#"{"category":"entities"}"#).unwrap();
    assert_eq!(
        describe,
        b.describe_response_json("entities").unwrap().into_bytes()
    );
    let unsupported = guest_call(&b, dispatch, "__describe", br#"{"category":"nope"}"#);
    assert_eq!(unsupported, Err("unsupported category: nope".to_string()));
    let input = serde_json::to_vec(&CommandInput::default()).unwrap();
    let command = guest_call(&b, dispatch, "cmd__list", &input).unwrap();
    let output: serde_json::Value = serde_json::from_slice(&command).unwrap();
    assert_eq!(output["stdout"], "hi");
    let pass = guest_call(&b, dispatch, "__pass_audit", br#"{"entities":[]}"#).unwrap();
    assert_eq!(pass, b"[]");
    assert_eq!(
        guest_call(&b, dispatch, "scan__x", b""),
        Ok(b"scanned".to_vec())
    );
    assert_eq!(
        guest_call(&b, dispatch, "nope", b""),
        Err("unknown export 'nope'".to_string())
    );
}
