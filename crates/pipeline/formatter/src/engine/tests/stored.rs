//! Format keeps what the graph stores (ADR 0247): under real field forms, a value formatted stores the same
//! text and names the same references, and leaves no W159 that format can clear.

use super::value_forms::forms;
use super::*;
use proptest::prelude::*;
use proptest::test_runner::TestRunner;
use specforge_graph::field_form::{spell, write};
use specforge_parser::testing::property_config;
use specforge_test_macros::test as specforge_test;

const FIELDS: [(&str, &str); 8] = [
    ("behavior", "category"),
    ("behavior", "contract"),
    ("behavior", "points"),
    ("behavior", "urgent"),
    ("behavior", "due"),
    ("behavior", "types"),
    ("behavior", "tags"),
    ("type", "types"),
];

const SPECIAL: [&str; 7] = [
    "true",
    "false",
    "2026-03-01",
    "gh.issue:1",
    "+3",
    " 3",
    "007",
];

fn is_word(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `text` written the way `shape` says (0 bare when it lexes as one word, else quoted).
fn written(text: &str, shape: u8) -> String {
    match shape {
        0 if is_word(text) => text.to_string(),
        2 => spell::quoted(&format!("{text}\"")),
        3 => format!("\"\"\"{text}\"\"\""),
        4 => format!("\"\"\"\n    {text}\n  \"\"\""),
        5 => format!(
            "[{}, {}]",
            spell::quoted(text),
            if is_word(text) {
                text.to_string()
            } else {
                spell::quoted("b")
            }
        ),
        _ => spell::quoted(text),
    }
}

fn source(holder: &str, field: &str, value: &str) -> String {
    format!("{holder} h \"H\" {{\n  {field} {value}\n}}\n")
}

/// Every entity's (kind, id, field, stored text, reference ids), in source order.
fn stored_of(text: &str, forms: &FieldForms) -> Vec<String> {
    let file = specforge_parser::parse(text, "p.spec");
    let mut out = Vec::new();
    for e in &file.entities {
        for entry in e.fields.entries() {
            let kind = e.kind.raw.as_str();
            let stored = forms.stored(kind, entry);
            let refs: Vec<&str> = forms
                .reading(kind, entry.key.as_str())
                .references(entry)
                .iter()
                .map(|r| r.id)
                .collect();
            out.push(format!(
                "{kind} {} {:?} {refs:?}",
                entry.key,
                write(&stored)
            ));
        }
    }
    out
}

/// The rewrites W159 would name in `text` that format could clear (a multi-line list text it cannot split aside).
fn clearable(text: &str, forms: &FieldForms) -> Vec<String> {
    let file = specforge_parser::parse(text, "p.spec");
    let mut out = Vec::new();
    for e in &file.entities {
        for entry in e.fields.entries() {
            for r in forms.rewrites(e.kind.raw.as_str(), entry, &e.span) {
                if !(r.listed && r.text.contains('\n')) {
                    out.push(format!("{} {:?}: {}", entry.key, r.text, r.value));
                }
            }
        }
    }
    out
}

#[specforge_test(
    invariant = "formatting_semantic_preservation",
    verify = "format(spec) under the project's field forms stores the same values and references as spec"
)]
#[specforge_test(
    behavior = "apply_format_rules",
    verify = "format writes a value in another form exactly where W159's rule names one"
)]
#[ignore = "flipped by T4"]
fn formatting_keeps_every_stored_value_and_reference() {
    let forms = forms();
    let config = FormatConfig::default();
    let text = prop_oneof![
        "[ a-z0-9_:.+\\-]{0,8}".prop_map(|t| t),
        proptest::sample::select(SPECIAL.to_vec()).prop_map(str::to_string),
    ];
    let case = (0..FIELDS.len(), text, 0u8..6);
    let mut runner = TestRunner::new(property_config());
    let result = runner.run(&case, |(field, text, shape)| {
        let (holder, field) = FIELDS[field];
        let source = source(holder, field, &written(&text, shape));
        prop_assume!(specforge_parser::parse(&source, "p.spec").errors.is_empty());
        let once = format_source(&source, &config, &forms).formatted;
        prop_assert!(
            specforge_parser::parse(&once, "p.spec").errors.is_empty(),
            "format wrote text that does not parse: {once:?} from {source:?}"
        );
        prop_assert_eq!(stored_of(&source, &forms), stored_of(&once, &forms));
        prop_assert!(
            clearable(&once, &forms).is_empty(),
            "W159 left after format: {:?} in {once:?}",
            clearable(&once, &forms)
        );
        prop_assert_eq!(format_source(&once, &config, &forms).formatted, once);
        Ok(())
    });
    if let Err(e) = result {
        panic!("{e}");
    }
}
