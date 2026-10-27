//! `specforge.explain` explains a code the served project's extensions declare (ADR 0278), and the
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
    verify = "specforge.explain explains a code the served project's extensions declare, as specforge explain does"
)]
fn explain_explains_a_code_the_served_projects_extension_declares() {
    let mut server = TestProject::new().serve(&[TestExtension::named("@acme/launch")
        .kind("campaign", false)
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
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(
        tool_json(&reply),
        json!({
            "code": "E903",
            "level": "error",
            "retired": false,
            "rules": [{
                "extension": "@acme/launch",
                "target_kind": "campaign",
                "check": "missing_required_field",
                "message": "campaign '{id}' has no end",
            }],
        })
    );
}

// Pins K10-D2: today a namespaced code is unknown_code; T6 un-ignores it.
#[specforge_test(
    behavior = "provide_mcp_explain_tool",
    verify = "specforge.explain explains a code the served project's extensions declare, as specforge explain does"
)]
#[ignore = "K10 T6"]
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

// Pins K10-D2: today a third-party code has no title; T6 un-ignores it.
#[specforge_test(
    behavior = "expose_diagnostics_as_mcp_resource",
    verify = "each catalogued diagnostic in the resource carries its title"
)]
#[ignore = "K10 T6"]
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
