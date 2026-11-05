//! A checked value on the hover (ADR 0336): one line per checker, and why one did not run.

use specforge_extension_sdk::prelude::*;
use specforge_test_macros::test as spec;
use tower_lsp::lsp_types::Position;

use crate::served::{Served, hover_text};

const EXT: &str = "@pin/hovered";
const SPEC: &str = "service a \"A\" {\n  home \"fine\"\n  site \"https://a.example/x\"\n}\n";

fn declare(c: &mut ContributionsBuilder) {
    c.code("E001", |d| {
        d.title("A bad value").explanation("A bad value.");
    });
    c.capability(Capability::http("links").from_values().why("to ask"));
    for name in ["plain", "second"] {
        c.checker(name, |k| {
            k.applies_to(&["string"])
                .plan(|_: &CheckItem| PlannedItem::findings(Vec::new()));
        });
    }
    c.checker("link", |k| {
        k.applies_to(&["string"])
            .scope("links")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::HttpHead {
                    url: item.text.clone(),
                    redirects: 5,
                }])
            })
            .judge(|_: &CheckItem| Vec::<CheckFinding>::new());
    });
    c.kind("service", |k| {
        k.keyword("service");
        k.field("home", |f| {
            f.field_type(FieldType::String)
                .check_with("plain", serde_json::json!({}))
                .check("second");
        });
        k.field("site", |f| {
            f.field_type(FieldType::String).check("link");
        });
    });
}

fn served() -> Served {
    Served::new(&[("a.spec", SPEC)])
        .extension(EXT, declare)
        .open(&["a.spec"])
}

fn hover(served: &Served, line: u32, character: u32) -> String {
    hover_text(
        &served.state(),
        &served.uri("a.spec"),
        Position::new(line, character),
    )
    .expect("a hover")
}

#[spec(
    behavior = "hover_information",
    verify = "hover on a checked value shows one line per checker"
)]
fn hover_on_a_checked_value_shows_one_line_per_checker() {
    let text = hover(&served(), 1, 10);
    assert!(text.contains("`plain` · passed"), "{text}");
    assert!(text.contains("`second` · passed"), "{text}");
}

#[spec(
    behavior = "hover_information",
    verify = "a check not run says why on the value's hover"
)]
fn a_not_run_check_says_why() {
    let text = hover(&served(), 2, 12);
    assert!(text.contains("`link` · not run ("), "{text}");
}

#[spec(
    behavior = "hover_information",
    verify = "a field's hover names its checks"
)]
fn a_fields_hover_names_its_checks() {
    let text = hover(&served(), 1, 3);
    assert!(text.contains("checks: plain, second"), "{text}");
}
