//! The registry resolves every check to a loaded checker, and refuses a checker or check it cannot run: W019 for a
//! name that leads nowhere, W189 for a shape. What it refuses is dropped; the rest is kept.

use serde_json::json;
use specforge_extension_sdk::prelude::*;
use specforge_registry::compilation::CandidateVocabulary;
use specforge_registry::{FieldType, ResolvedCheck};

use crate::support::{build, build_with, coded, declare, diagnostics, optional_peer, peer};

/// A declared code, so a checker may list it.
fn with_code(c: &mut ContributionsBuilder, code: &str) {
    c.code(code, |k| {
        k.title("A check's finding").explanation("A finding.");
    });
}

/// `@test/checks`, with the pure checker `exists`.
fn checks() -> specforge_protocol_types::ExtensionDeclaration {
    declare("@test/checks", |c| {
        with_code(c, "W001");
        c.checker("exists", |k| {
            k.applies_to(&["string", "string_list"])
                .codes(&["W001"])
                .arg("root", |a| {
                    a.field_type(FieldType::String);
                })
                .plan(|_| PlannedItem::default());
        });
    })
}

fn string_field_with(
    extension: &str,
    peers: Vec<specforge_protocol_types::PeerDependency>,
    check: &str,
) -> specforge_protocol_types::ExtensionDeclaration {
    let check = check.to_string();
    declare(extension, move |c| {
        c.meta.peer_dependencies = peers;
        c.kind("Thing", |k| {
            k.keyword("thing");
            k.field("docs", |f| {
                f.field_type(FieldType::String).check(&check);
            });
        });
    })
}

#[test]
fn an_unknown_checker_is_w019_naming_it() {
    let build = build([string_field_with("@test/ext", vec![], "nope")]);
    let found = coded(&build, "W019");
    assert_eq!(found.len(), 1, "{:?}", diagnostics(&build));
    assert!(found[0].message.contains("'nope'"), "{}", found[0].message);
    assert!(build.checks().on("thing", "docs").is_empty());
}

#[test]
fn another_extensions_checker_needs_a_peer() {
    let refused = build([
        checks(),
        string_field_with("@test/ext", vec![], "@test/checks:exists"),
    ]);
    assert_eq!(
        coded(&refused, "W019").len(),
        1,
        "{:?}",
        diagnostics(&refused)
    );
    assert!(refused.checks().on("thing", "docs").is_empty());

    let kept = build([
        checks(),
        string_field_with(
            "@test/ext",
            vec![peer("@test/checks", "*")],
            "@test/checks:exists",
        ),
    ]);
    assert!(diagnostics(&kept).is_empty(), "{:?}", diagnostics(&kept));
    assert!(matches!(
        kept.checks().on("thing", "docs"),
        [ResolvedCheck::One(call)] if call.extension == "@test/checks" && call.checker == "exists"
    ));
}

#[test]
fn a_severity_argument_is_w189_citing_one_code_per_severity() {
    let build = build([declare("@test/ext", |c| {
        c.checker("url", |k| {
            k.applies_to(&["string"])
                .arg("severity", |a| {
                    a.field_type(FieldType::String);
                })
                .plan(|_| PlannedItem::default());
        });
    })]);
    let found = coded(&build, "W189");
    assert_eq!(found.len(), 1, "{:?}", diagnostics(&build));
    assert!(
        found[0].message.contains("one code each"),
        "{}",
        found[0].message
    );
    assert!(build.checks().checker("@test/ext", "url").is_none());
}

#[test]
fn a_check_of_an_optional_peer_not_loaded_is_inert() {
    let build = build([string_field_with(
        "@test/ext",
        vec![optional_peer("@specforge/checks", "^0.1")],
        "@specforge/checks:file",
    )]);
    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
    assert!(build.checks().on("thing", "docs").is_empty());
}

#[test]
fn a_checker_a_loaded_extension_lacks_is_w019_not_inert() {
    let build = build([
        checks(),
        string_field_with(
            "@test/ext",
            vec![optional_peer("@test/checks", "*")],
            "@test/checks:absent",
        ),
    ]);
    assert_eq!(coded(&build, "W019").len(), 1, "{:?}", diagnostics(&build));
}

#[test]
fn a_check_on_a_collected_kinds_field_is_w189() {
    let build = build([declare("@test/ext", |c| {
        with_code(c, "W001");
        c.checker("exists", |k| {
            k.applies_to(&["string"]).plan(|_| PlannedItem::default());
        });
        c.kind("Fact", |k| {
            k.keyword("fact").collected(&["t"]);
            k.field("docs", |f| {
                f.field_type(FieldType::String).check("exists");
            });
        });
    })]);
    let found = coded(&build, "W189");
    assert_eq!(found.len(), 1, "{:?}", diagnostics(&build));
    assert!(
        found[0].message.contains("collector"),
        "{}",
        found[0].message
    );
    assert!(build.checks().on("fact", "docs").is_empty());
}

#[test]
fn a_check_on_a_computed_or_verdict_field_is_w189() {
    for writer in ["computed", "verdict"] {
        let build = build([declare("@test/ext", |c| {
            c.checker("exists", |k| {
                k.applies_to(&["string"]).plan(|_| PlannedItem::default());
            });
            c.kind("Thing", |k| {
                k.keyword("thing");
                k.field("docs", |f| {
                    f.field_type(FieldType::String).check("exists");
                    if writer == "computed" {
                        f.computed();
                    } else {
                        f.verdict();
                    }
                });
            });
        })]);
        let found = coded(&build, "W189");
        assert!(
            found.iter().any(|d| d.message.contains(writer)),
            "{:?}",
            diagnostics(&build)
        );
    }
}

#[test]
fn checks_resolve_in_field_order_with_members() {
    let build = build([declare("@test/ext", |c| {
        c.checker("a", |k| {
            k.applies_to(&["string"]).plan(|_| PlannedItem::default());
        });
        c.checker("b", |k| {
            k.applies_to(&["string"]).plan(|_| PlannedItem::default());
        });
        c.kind("Thing", |k| {
            k.keyword("thing");
            k.field("docs", |f| {
                f.field_type(FieldType::String).check("b").check("a");
            });
            k.field("links", |f| {
                f.field_type(FieldType::BlockList);
                f.member("href", |m| {
                    m.field_type(FieldType::String).check("a");
                });
            });
        });
    })]);
    assert!(diagnostics(&build).is_empty(), "{:?}", diagnostics(&build));
    let order: Vec<(&str, Vec<&str>)> = build
        .checks()
        .of_kind("thing")
        .map(|(path, checks)| {
            let names = checks
                .iter()
                .flat_map(|c| c.calls())
                .map(|c| c.checker.as_str());
            (path, names.collect())
        })
        .collect();
    assert_eq!(order, [("docs", vec!["b", "a"]), ("links.href", vec!["a"])]);
}

/// `f` builds the only field, of type string, on a kind, and `k` the only checker, `the`.
fn refusing(
    k: impl FnOnce(&mut CheckerBuilder),
    f: impl FnOnce(&mut FieldBuilder),
) -> specforge_registry::RegistryBuild {
    build([declare("@test/ext", |c| {
        c.capability(Capability::http("links").from_values().why("to ask"));
        with_code(c, "W001");
        c.checker("the", |b| {
            b.applies_to(&["string"]).plan(|_| PlannedItem::default());
            k(b);
        });
        c.kind("Thing", |t| {
            t.keyword("thing");
            t.field("docs", |d| {
                d.field_type(FieldType::String);
                f(d);
            });
        });
    })])
}

#[test]
fn a_checker_the_host_cannot_run_is_w189_and_dropped() {
    type Change = Box<dyn FnOnce(&mut CheckerBuilder)>;
    let cases: [(&str, Change); 7] = [
        (
            "other than text",
            Box::new(|k| {
                k.value(ValueForm::Unknown);
            }),
        ),
        (
            "applies to 'block'",
            Box::new(|k| {
                k.applies_to(&["block"]);
            }),
        ),
        (
            "the scope 'nope'",
            Box::new(|k| {
                k.scope("nope");
            }),
        ),
        (
            "the code 'W002'",
            Box::new(|k| {
                k.codes(&["W002"]);
            }),
        ),
        (
            "root_arg 'root'",
            Box::new(|k| {
                k.root_arg("root");
            }),
        ),
        (
            "in a form",
            Box::new(|k| {
                k.arg("r", |a| {
                    a.field_type(FieldType::Reference);
                });
            }),
        ),
        (
            "root_arg 'r'",
            Box::new(|k| {
                k.value(ValueForm::Path).root_arg("r");
            }),
        ),
    ];
    for (says, change) in cases {
        let build = refusing(change, |f| {
            f.check("the");
        });
        let found = coded(&build, "W189");
        assert_eq!(found.len(), 1, "{says}: {:?}", diagnostics(&build));
        assert!(
            found[0].message.contains(says),
            "{says}: {}",
            found[0].message
        );
        assert!(build.checks().on("thing", "docs").is_empty(), "{says}");
        assert!(coded(&build, "W019").is_empty(), "{says}");
    }
}

#[test]
fn a_checker_named_outside_snake_case_or_twice_is_w189() {
    let plain = |k: &mut CheckerBuilder| {
        k.applies_to(&["string"]).plan(|_| PlannedItem::default());
    };
    let build = build([declare("@test/ext", |c| {
        c.checker("Bad-Name", plain);
        c.checker("twice", plain);
        c.checker("twice", plain);
    })]);
    let found = coded(&build, "W189");
    assert_eq!(found.len(), 2, "{:?}", diagnostics(&build));
    assert!(
        found[0].message.contains("snake_case"),
        "{}",
        found[0].message
    );
    assert!(found[1].message.contains("twice"), "{}", found[1].message);
    assert!(build.checks().checker("@test/ext", "twice").is_some());
}

#[test]
fn a_check_the_checker_refuses_is_w189_and_the_rest_are_kept() {
    let args = |value| {
        refusing(
            |k| {
                k.arg("root", |a| {
                    a.field_type(FieldType::String).required();
                });
            },
            move |f| {
                f.check_with("the", value);
            },
        )
    };
    for (says, value) in [
        ("does not take", json!({"root": "a", "other": 1})),
        ("does not admit", json!({"root": 1})),
        ("requires", json!({})),
    ] {
        let build = args(value);
        let found = coded(&build, "W189");
        assert_eq!(found.len(), 1, "{says}: {:?}", diagnostics(&build));
        assert!(found[0].message.contains(says), "{}", found[0].message);
    }
    let kept = args(json!({"root": "spec"}));
    assert!(diagnostics(&kept).is_empty(), "{:?}", diagnostics(&kept));
    assert_eq!(kept.checks().on("thing", "docs").len(), 1);
}

#[test]
fn a_check_on_a_type_its_checker_does_not_read_is_w189() {
    let build = build([declare("@test/ext", |c| {
        c.checker("exists", |k| {
            k.applies_to(&["string"]).plan(|_| PlannedItem::default());
        });
        c.kind("Thing", |k| {
            k.keyword("thing");
            k.field("count", |f| {
                f.field_type(FieldType::Integer).check("exists");
            });
        });
    })]);
    assert_eq!(coded(&build, "W189").len(), 1, "{:?}", diagnostics(&build));
}

#[test]
fn any_of_needs_two_branches_one_level_and_each_to_name_something() {
    let any_of = |branches: Vec<CheckRef>| {
        refusing(
            |_| {},
            move |f| {
                f.any_of(branches);
            },
        )
    };
    let the = || CheckRef::named("the");
    assert_eq!(coded(&any_of(vec![the()]), "W189").len(), 1);
    let nested = CheckRef::AnyOf {
        any_of: vec![the(), the()],
    };
    assert_eq!(coded(&any_of(vec![the(), nested]), "W189").len(), 1);
    assert_eq!(
        coded(&any_of(vec![the(), CheckRef::named("nope")]), "W019").len(),
        1
    );
    let kept = any_of(vec![the(), the()]);
    assert!(matches!(
        kept.checks().on("thing", "docs"),
        [ResolvedCheck::AnyOf(calls)] if calls.len() == 2
    ));
}

#[test]
fn candidates_do_not_make_an_undeclared_extension_inert() {
    let build = build_with(
        [string_field_with(
            "@test/ext",
            vec![],
            "@specforge/checks:file",
        )],
        CandidateVocabulary::default(),
    );
    assert_eq!(coded(&build, "W019").len(), 1, "{:?}", diagnostics(&build));
}
