//! `specforge.explain` explains a code the served project's extensions declare (ADR 0278).

use crate::support::*;
use serde_json::json;
use specforge_extension_sdk::prelude::{CheckKind, ValidationSeverity};
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
