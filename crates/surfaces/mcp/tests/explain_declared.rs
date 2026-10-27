//! `specforge.explain` explains a code the served project's extensions catalog, and the
//! project's extensions' codes are titled from their catalogs (ADR 0286).

use crate::support::*;
use serde_json::Value;
use serde_json::json;
use specforge_extension_sdk::prelude::{
    CheckKind, PassDiagnostic, PassSeverity, ValidationSeverity,
};
use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "provide_mcp_explain_tool",
    verify = "specforge.explain explains a code the served project's extensions catalog, as specforge explain does"
)]
fn a_bare_code_an_extension_catalogs_is_refused_with_its_namespaced_form() {
    let mut server = TestProject::new().serve(&[TestExtension::named("@acme/launch")
        .kind("campaign", false)
        .coding("E903")
        .declaring(|c| {
            c.rule("E903", |r| {
                r.check(CheckKind::MissingRequiredField)
                    .target_kind("campaign")
                    .field("ends")
                    .severity(ValidationSeverity::Error)
                    .message_template("campaign '{id}' has no end");
            });
        })]);
    let reply = server.call_tool("specforge.explain", json!({"code": " e903 "}));
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("unknown diagnostic code: e903"), "{text}");
    assert!(text.contains("did you mean acme.launch/E903?"), "{text}");
    let reply = server.call_tool("specforge.explain", json!({"code": "acme.launch/e903"}));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(tool_json(&reply)["owner"], "@acme/launch");
}

#[specforge_test(
    behavior = "provide_mcp_explain_tool",
    verify = "specforge.explain explains a code the served project's extensions catalog, as specforge explain does"
)]
fn explain_answers_a_namespaced_code_from_the_project() {
    let mut server =
        TestProject::new().serve(&[TestExtension::named("@sdk/greet").declaring(|c| {
            c.code("E901", |d| {
                d.title("Greeting missing")
                    .explanation("Add a greeting.")
                    .docs("https://greet.dev/codes#e901");
            });
        })]);
    let reply = server.call_tool("specforge.explain", json!({"code": "SDK.GREET/e901"}));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let entry = tool_json(&reply);
    assert_eq!(entry["code"], "sdk.greet/E901", "{entry}");
    assert_eq!(entry["title"], "Greeting missing", "{entry}");
    assert_eq!(entry["docs"], "https://greet.dev/codes#e901", "{entry}");
}

#[specforge_test(
    behavior = "expose_diagnostics_as_mcp_resource",
    verify = "each catalogued diagnostic in the resource carries its title"
)]
fn a_third_party_code_is_titled_from_its_catalog() {
    let mut server = TestProject::new().serve(&[TestExtension::named("@sdk/greet")
        .reporting(PassDiagnostic::new("W901", PassSeverity::Warning, "m"))
        .declaring(|c| {
            c.code("W901", |d| {
                d.title("Greeting missing")
                    .explanation("Add a greeting.")
                    .docs("https://greet.dev/codes#w901");
            });
        })]);
    let bag: Value = serde_json::from_str(&resource_text(&read_resource(
        &mut server,
        "specforge://diagnostics",
    )))
    .unwrap();
    let ours = bag
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["origin"] == "@sdk/greet")
        .expect("the extension's finding");
    assert_eq!(ours["code"], "sdk.greet/W901", "{bag}");
    assert_eq!(ours["title"], "Greeting missing", "{bag}");
}
