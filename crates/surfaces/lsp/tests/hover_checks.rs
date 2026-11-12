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

const OWNED: &str = "plan p \"P\" {\n  state draft\n  files [\"new.js\"]\n}\n\nplan q \"Q\" {\n  state complete\n  gone [\"old.js\"]\n}\n";

/// A `plan` complete on `state ∈ [complete]`: its `files` are read by `plain` once it is, its `gone` while it is not.
fn owned(c: &mut ContributionsBuilder) {
    c.checker("plain", |k| {
        k.applies_to(&["string", "string_list"])
            .plan(|_: &CheckItem| PlannedItem::findings(Vec::new()));
    });
    c.kind("plan", |k| {
        k.complete_when("state", &["complete"])
            .field("state", |f| {
                f.vocabulary(&Vocabulary::new(&["draft", "complete"]));
            })
            .field("files", |f| {
                f.field_type(FieldType::StringList).check("plain").planned();
            })
            .field("gone", |f| {
                f.field_type(FieldType::StringList)
                    .check("plain")
                    .while_open();
            });
    });
}

fn owned_hover(line: u32, character: u32) -> String {
    let served = Served::new(&[("a.spec", OWNED)])
        .extension("@pin/owned", owned)
        .open(&["a.spec"]);
    hover(&served, line, character)
}

#[spec(
    behavior = "hover_information",
    verify = "a planned value's hover says it waits for its owner, and a closed one's that its owner is complete"
)]
fn a_planned_value_says_it_waits_for_its_owner() {
    let planned = owned_hover(2, 11);
    assert!(
        planned.contains("`plain` · planned: checked once its owner is complete"),
        "{planned}"
    );
    let closed = owned_hover(7, 10);
    assert!(
        closed.contains("`plain` · closed: its owner is complete"),
        "{closed}"
    );
}

#[spec(
    behavior = "hover_information",
    verify = "a field's hover names when its owner gate lets its checks run"
)]
fn a_fields_hover_names_its_owner_gate() {
    let files = owned_hover(2, 3);
    assert!(
        files.contains("checks: plain, once its owner is complete"),
        "{files}"
    );
    let gone = owned_hover(7, 3);
    assert!(
        gone.contains("checks: plain, while its owner is pending"),
        "{gone}"
    );
}
