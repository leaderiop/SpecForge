//! A value check may POST a bounded JSON query to a host its capability lists (ADR 0336): performed through the
//! grant, journaled by its body, and its answer read up to the capability's `max_bytes`.

use specforge_effects::EffectsMode;
use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_extension_sdk::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{EXT, Scene, coded, described};

const QUERY: &str = "https://api.osv.dev/v1/query";

/// The class and the length of each answer's body.
fn told(item: &CheckItem) -> Vec<CheckFinding> {
    item.outcomes
        .iter()
        .map(|outcome| {
            let text = match &outcome.data {
                OutcomeData::Http { body, .. } => format!("{} {}", outcome.class, body.len()),
                other => format!("{other:?}"),
            };
            CheckFinding::new("E001", text)
        })
        .collect()
}

/// `name` POSTs a query naming its value under `scope`.
fn querier(c: &mut ContributionsBuilder, name: &str, scope: &str) {
    c.checker(name, |k| {
        k.applies_to(&["string"])
            .scope(scope)
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::HttpPostQuery {
                    url: QUERY.to_string(),
                    json_body: serde_json::json!({ "package": item.text }).to_string(),
                }])
            })
            .judge(told);
    });
}

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    described(&mut c);
    c.capability(
        Capability::http("registries")
            .hosts(&["api.osv.dev"])
            .methods(&[HttpMethod::Post])
            .call_kinds(&[Operation::CheckValues])
            .max_bytes(4 << 20)
            .why("to ask advisories"),
    );
    c.capability(
        Capability::http("links")
            .from_values()
            .why("to check links"),
    );
    querier(&mut c, "listed", "registries");
    querier(&mut c, "named", "links");
    c.kind("service", |k| {
        k.keyword("service");
        for field in ["listed", "named"] {
            k.field(field, |f| {
                f.field_type(FieldType::String).check(field);
            });
        }
    });
    c
}

fn told_in(scene: &Scene, mode: EffectsMode) -> Vec<String> {
    let found = scene.checked(mode, &MemoryConsent::default()).diagnostics();
    coded(&found, "pin.probes/E001")
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a POST query to a listed host is performed, kept to its capability's max_bytes and journaled by body"
)]
fn a_post_query_to_a_listed_host_is_performed_and_journaled_by_body() {
    // Over the host's 1 MiB, within the capability's 4 MiB.
    let http = FakeHttp::new().route(QUERY, 200, vec![b'x'; 2 << 20]);
    let spec =
        "service a \"A\" {\n  listed \"serde\"\n}\nservice b \"B\" {\n  listed \"tokio\"\n}\n";
    let scene = Scene::with(http, spec, &[EXT], InProcessRuntime::new().with(extension));
    let live = told_in(&scene, EffectsMode::Online);
    assert_eq!(live, ["ok 2097152", "ok 2097152"]);
    let asked = scene.http.requests();
    let mut bodies: Vec<String> = asked
        .iter()
        .map(|request| {
            String::from_utf8_lossy(request.body.as_deref().unwrap_or_default()).into_owned()
        })
        .collect();
    // The two values' probes run at once, in either order.
    bodies.sort();
    assert_eq!(bodies, [r#"{"package":"serde"}"#, r#"{"package":"tokio"}"#]);
    assert!(asked.iter().all(|request| {
        request.method == "POST"
            && request.max_redirects == 0
            && request
                .headers
                .iter()
                .any(|h| h.name == "content-type" && h.value == "application/json")
    }));
    // One record per body: both are answered from the journal.
    assert_eq!(told_in(&scene, EffectsMode::Replay), live);
    assert_eq!(scene.http.requests().len(), 2);
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a POST query to a host only a value names is E090 and never sent"
)]
fn a_post_query_to_a_host_a_value_names_is_e090() {
    let http = FakeHttp::new().route(QUERY, 200, "{}");
    let spec = "service a \"A\" {\n  named \"serde\"\n}\n";
    let scene = Scene::with(http, spec, &[EXT], InProcessRuntime::new().with(extension));
    let found = scene.online().diagnostics();
    let refused = coded(&found, "E090");
    assert_eq!(refused.len(), 1, "{found:?}");
    assert!(
        refused[0]
            .message
            .contains("goes only to a host its capability lists"),
        "{}",
        refused[0].message
    );
    assert!(coded(&found, "pin.probes/E001").is_empty());
    assert!(scene.http.requests().is_empty());
}
