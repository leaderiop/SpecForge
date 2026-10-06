//! One extension vocabulary: what an SDK-built extension declares is what
//! the host's registry build reads. Every field type the SDK can name
//! registers (no W019), and the rules `specforge new --extension` and the
//! greet fixture write parse (no W112) and fire.

use std::collections::HashMap;

use specforge_common::{SourceSpan, Sym};
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta, prelude::*};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::build_registries;
use specforge_registry::validation_engine::{ValidationEntity, execute_pattern};
use specforge_wasm::protocol::load_declaration;
use specforge_wasm::testing::InProcessRuntime;

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@you/vocab", "0.1.0"));
    c.kind("thing", |k| {
        for t in FieldType::ALL {
            k.field(&format!("f_{t}"), |f| {
                f.field_type(*t);
                if *t == FieldType::Enum {
                    f.enum_values(&["warm", "formal"]);
                }
            });
        }
        k.field("description", |f| {
            f.field_type(FieldType::String);
        });
        k.field("style", |f| {
            f.field_type(FieldType::String);
        });
    });
    // The rule `specforge new --extension` scaffolds.
    c.rule("W900", |r| {
        r.check(CheckKind::MissingRequiredField);
        r.target_kind("thing");
        r.field("description");
        r.severity(ValidationSeverity::Warning);
        r.message_template("thing '{id}' is missing a description");
    });
    // The greet fixture's rule.
    c.rule("G101", |r| {
        r.check(CheckKind::FieldValueConstraint);
        r.target_kind("thing");
        r.field("style");
        r.constraint(|fc| {
            fc.kind(ConstraintKind::Matches);
            fc.pattern("^(warm|formal)$");
        });
        r.severity(ValidationSeverity::Error);
        r.message_template("thing '{id}' has unknown style");
    });
    c
}

/// The declaration `build` declares, as the host loads it: through the
/// one loader, from the extension served in process.
fn loaded(
    build: impl Fn() -> ContributionsBuilder + Send + Sync + 'static,
) -> ExtensionDeclaration {
    let name = build().meta.name.clone();
    let runtime = InProcessRuntime::new().with(build);
    let loaded = load_declaration(&runtime, &name).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    loaded.declaration
}

fn entity(id: &str, fields: &[(&str, &str)]) -> ValidationEntity {
    ValidationEntity {
        id: id.to_string(),
        kind: "thing".to_string(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        incoming_edge_count: 0,
        outgoing_edge_count: 0,
        span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        verify_kinds: Vec::new(),
        verify_texts: Vec::new(),
        outgoing_kinds: Default::default(),
        incoming_kinds: Default::default(),
        obligation_exempt: false,
    }
}

#[test]
fn sdk_vocabulary_round_trips_through_the_registry_build() {
    let build = build_registries(vec![loaded(extension)]);

    let unread: Vec<_> = build
        .registry_diagnostics
        .iter()
        .filter(|d| d.code == "W019" || d.code == "W112")
        .collect();
    assert!(unread.is_empty(), "host could not read: {unread:?}");
    for t in FieldType::ALL {
        assert!(
            build.fields.contains("thing", &format!("f_{t}")),
            "field type {t} did not register"
        );
    }

    let rule = |code: &str| {
        &build
            .rules
            .iter()
            .find(|(p, _)| p.code == code)
            .unwrap_or_else(|| panic!("rule {code} not registered"))
            .0
    };
    let bare = [entity("bare", &[("style", "loud")])];
    let fine = [entity(
        "fine",
        &[("description", "a thing"), ("style", "warm")],
    )];

    let fired = execute_pattern(rule("W900"), &bare, None);
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert_eq!(fired[0].message, "thing 'bare' is missing a description");
    let fired = execute_pattern(rule("G101"), &bare, None);
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert_eq!(fired[0].code, "G101");
    assert!(execute_pattern(rule("W900"), &fine, None).is_empty());
    assert!(execute_pattern(rule("G101"), &fine, None).is_empty());
}

/// Rules written by SDK releases before the vocabulary was shared still
/// load: the host reads their check names as aliases.
#[test]
fn older_sdk_check_names_still_load() {
    let mut ext = loaded(extension);
    for (rule, old) in ext
        .validation_rules
        .iter_mut()
        .zip(["missing_field", "field_constraint"])
    {
        rule.check = old.to_string();
    }
    ext.entities[0].fields[0].field_type = "boolean".to_string();

    let build = build_registries(vec![ext]);
    let unread: Vec<_> = build
        .registry_diagnostics
        .iter()
        .filter(|d| d.code == "W019" || d.code == "W112")
        .collect();
    assert!(unread.is_empty(), "host could not read: {unread:?}");
    assert!(build.rules.iter().any(|(p, _)| p.code == "W900"));
    assert!(build.rules.iter().any(|(p, _)| p.code == "G101"));
}

/// Every constraint kind the SDK can name loads on the check that reads it:
/// `non_empty`, `one_of` and `matches` on `field_value_constraint`,
/// `when_field_equals` on `conditional_field_required`, `one_of` on
/// `verify_kind_allowlist`.
#[test]
fn sdk_constraint_kinds_load_for_the_checks_that_read_them() {
    let uses = [
        (ConstraintKind::NonEmpty, CheckKind::FieldValueConstraint),
        (ConstraintKind::OneOf, CheckKind::FieldValueConstraint),
        (ConstraintKind::Matches, CheckKind::FieldValueConstraint),
        (
            ConstraintKind::WhenFieldEquals,
            CheckKind::ConditionalFieldRequired,
        ),
        (ConstraintKind::OneOf, CheckKind::VerifyKindAllowlist),
    ];
    for kind in ConstraintKind::ALL {
        assert!(
            uses.iter().any(|(k, _)| k == kind),
            "constraint kind {kind} has no check that reads it"
        );
    }
    let build = move || {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@you/constraints", "0.1.0"));
        c.kind("thing", |k| {
            k.field("status", |f| {
                f.field_type(FieldType::String);
            });
            k.field("reason", |f| {
                f.field_type(FieldType::String);
            });
        });
        for (i, (kind, check)) in uses.iter().enumerate() {
            c.rule(&format!("X{i:03}"), |r| {
                r.check(*check);
                r.target_kind("thing");
                if *check != CheckKind::VerifyKindAllowlist {
                    r.field("reason");
                }
                r.constraint(|fc| {
                    fc.kind(*kind);
                    match kind {
                        ConstraintKind::Matches => {
                            fc.pattern("^[a-z]+$");
                        }
                        ConstraintKind::WhenFieldEquals => {
                            fc.pattern("status").values(&["deferred"]);
                        }
                        _ => {
                            fc.values(&["unit", "a"]);
                        }
                    }
                });
                r.message_template("thing '{id}'");
            });
        }
        c
    };

    let build = build_registries(vec![loaded(build)]);
    let unread: Vec<_> = build
        .registry_diagnostics
        .iter()
        .filter(|d| d.code == "W019" || d.code == "W112")
        .collect();
    assert!(unread.is_empty(), "host could not read: {unread:?}");
    for (i, (kind, _)) in uses.iter().enumerate() {
        let code = format!("X{i:03}");
        let (pattern, _) = build
            .rules
            .iter()
            .find(|(p, _)| p.code == code)
            .unwrap_or_else(|| panic!("rule {code} not registered"));
        assert_eq!(
            pattern.constraint.as_ref().and_then(|c| c.kind),
            Some(*kind),
            "rule {code}"
        );
    }
}
