//! The registry build owns what is checked of the declarations themselves:
//! identity and shape (E030), peers (E027) and the order of the declared
//! passes (W145), from `build_registries` alone, with no runtime.

use specforge_extension_sdk::prelude::*;
use specforge_extension_sdk::{ExtensionDeclaration, PeerDependency};
use specforge_registry::build_registries;

use crate::support::{codes, diagnostics};

fn declaration(name: &str, version: &str) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, version));
    c.kind("report", |k| {
        k.description("A report");
    });
    c
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a declaration with an empty name or version produces E030"
)]
fn an_empty_name_or_version_is_e030() {
    let build = build_registries(vec![
        declaration("", "1.0.0").declaration(),
        declaration("@acme/b", "").declaration(),
        declaration("@acme/c", "1.0.0").declaration(),
    ]);
    // The declarations' own, then the registries' (the kind collides).
    assert_eq!(codes(&build), ["E030", "E030", "E026", "E026"]);
    assert!(diagnostics(&build)[0].message.contains("name is empty"));
    assert!(
        diagnostics(&build)[1]
            .message
            .contains("'@acme/b': its version is empty")
    );
    // The declarations still build: the registries hold their kinds.
    assert!(build.kinds().contains("report"));
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a malformed ext_short produces E030"
)]
fn a_malformed_short_name_is_e030() {
    let mut bad = declaration("@acme/reports", "1.0.0").declaration();
    // A raw handshake can carry any short; the SDK refuses one at build.
    bad.handshake.ext_short = Some("Reports Tool".to_string());
    let mut good = declaration("@acme/tools", "1.0.0").declaration();
    good.entities.clear();
    good.handshake.ext_short = Some("my-tools2".to_string());
    let build = build_registries(vec![bad, good]);
    assert_eq!(codes(&build), ["E030"]);
    let message = &diagnostics(&build)[0].message;
    assert!(
        message.contains("ext_short 'Reports Tool' is not lowercase kebab case"),
        "{message}"
    );
    assert_eq!(
        build.declaration("@acme/tools").unwrap().short(),
        "my-tools2"
    );
}

fn with_peer(name: &str, peer: &str, range: &str, optional: bool) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    c.meta.peer_dependencies.push(PeerDependency {
        name: peer.to_string(),
        version: range.to_string(),
        optional,
    });
    c.declaration()
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "peer dependencies are checked in the registry build"
)]
fn peers_are_checked_by_the_build() {
    // A required peer that is not loaded, one loaded out of range, and an
    // optional one absent (fine).
    let build = build_registries(vec![
        with_peer("@acme/a", "@acme/missing", "^1", false),
        with_peer("@acme/b", "@acme/a", "^2", false),
        with_peer("@acme/c", "@acme/also-missing", "^1", true),
    ]);
    assert_eq!(codes(&build), ["E027", "E027"]);
    assert_eq!(
        diagnostics(&build)[0].message,
        "extension '@acme/a' requires peer dependency '@acme/missing' ^1 which is not installed"
    );
    assert_eq!(
        diagnostics(&build)[1].message,
        "extension '@acme/b' requires peer dependency '@acme/a' ^2 but version 1.0.0 is installed"
    );
}

fn passes(name: &str, declare: impl FnOnce(&mut ContributionsBuilder)) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(name, "1.0.0"));
    declare(&mut c);
    c.declaration()
}

fn order(build: &specforge_registry::RegistryBuild) -> Vec<String> {
    build.passes().iter().map(|p| p.full_name()).collect()
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "declared passes are ordered in the registry build"
)]
fn passes_are_ordered_in_the_registry_build() {
    let build = build_registries(vec![
        passes("@acme/formal", |c| {
            // Declared out of order; `after` gives the run order. `resolve`
            // is a host phase, ignored.
            c.pass("event_graph_analyze", |p| {
                p.after("layering_verify").run(no_findings);
            });
            c.pass("coverage_tracking", |p| {
                p.after("event_graph_analyze")
                    .phase("check")
                    .run(no_findings);
            });
            c.pass("condition_check", |p| {
                p.after("resolve").run(no_findings);
            });
            c.pass("layering_verify", |p| {
                p.after("condition_check").run(no_findings);
            });
        }),
        passes("@acme/other", |c| {
            c.pass("second", |p| {
                p.before("first").run(no_findings);
            });
            c.pass("first", |p| {
                p.run(no_findings);
            });
            c.pass("solo", |p| {
                p.run(no_findings);
            });
        }),
    ]);
    assert_eq!(
        order(&build),
        [
            "@acme/formal:condition_check",
            "@acme/formal:layering_verify",
            "@acme/formal:event_graph_analyze",
            "@acme/formal:coverage_tracking",
            // Ready passes run in declaration order (stable Kahn): `solo`
            // is ready before `first`, which waits for `second`.
            "@acme/other:second",
            "@acme/other:solo",
            "@acme/other:first",
        ]
    );
    let check: Vec<String> = build.check_passes().map(|p| p.full_name()).collect();
    assert_eq!(check, ["@acme/formal:coverage_tracking"]);
    assert_eq!(build.analyze_passes().count(), 6);
    assert!(build.diagnostics().next().is_none());
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a pass constraint cycle produces W145 and keeps declaration order"
)]
fn a_pass_cycle_is_w145_in_declaration_order() {
    let build = build_registries(vec![passes("@acme/loop", |c| {
        c.pass("a", |p| {
            p.after("b").run(no_findings);
        });
        c.pass("b", |p| {
            p.after("a").run(no_findings);
        });
        c.pass("c", |p| {
            p.run(no_findings);
        });
    })]);
    assert_eq!(
        order(&build),
        ["@acme/loop:a", "@acme/loop:b", "@acme/loop:c"]
    );
    assert_eq!(codes(&build), ["W145"]);
    let message = &diagnostics(&build)[0].message;
    assert!(message.contains("'@acme/loop'"), "{message}");
    assert!(message.contains("'a', 'b'"), "{message}");
}

/// The declarations' own diagnostics come in one fixed order: E030, then
/// W021, then the peers' E073 and E027, then the cycles' E027, then W145,
/// each extension by extension.
#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "the declarations' own diagnostics come in a fixed order: E030, W021, the peers' E073 and E027, the cycles' E027, W145"
)]
fn declaration_diagnostics_come_in_a_fixed_order() {
    let mut cyclic = passes("@acme/z", |c| {
        c.pass("a", |p| {
            p.after("b").run(no_findings);
        });
        c.pass("b", |p| {
            p.after("a").run(no_findings);
        });
    });
    cyclic.handshake.peer_dependencies.push(PeerDependency {
        name: "@acme/nowhere".to_string(),
        version: "^1".to_string(),
        optional: false,
    });
    let mut inconsistent = declaration("@acme/y", "1.0.0");
    inconsistent.kind("note", |k| {
        k.field("about", |f| {
            f.field_type(FieldType::Reference).target_kind("nowhere");
        });
    });
    cyclic.handshake.peer_dependencies.push(PeerDependency {
        name: "@acme/x".to_string(),
        version: "nope".to_string(),
        optional: false,
    });
    let unnamed = declaration("", "1.0.0").declaration();
    // `@acme/p` and `@acme/q` require each other.
    let mutual = |name: &str, peer: &str| {
        let mut c = declaration(name, "1.0.0");
        c.meta.peer_dependencies = vec![PeerDependency {
            name: peer.to_string(),
            version: "^1".to_string(),
            optional: false,
        }];
        c.declaration()
    };
    let build = build_registries(vec![
        cyclic,
        inconsistent.declaration(),
        unnamed,
        mutual("@acme/p", "@acme/q"),
        mutual("@acme/q", "@acme/p"),
    ]);
    assert_eq!(
        codes(&build),
        [
            "E030", "W021", "E027", "E073", "E027", "W145", "E026", "E026", "E026"
        ]
    );
}

/// What a field or kind declares reaches its registry entry whole: the
/// entries embed the descriptor, so nothing declared is dropped on the way.
#[specforge_test_macros::test(
    behavior = "registry_build_fields",
    verify = "a field's declared descriptor reaches its registry entry whole"
)]
fn a_declared_default_value_reaches_the_field_registry() {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@acme/tickets", "1.0.0"));
    c.kind("ticket", |k| {
        k.incremental(true)
            .inference_guide("Look for issue trackers")
            .field("status", |f| {
                f.vocabulary(&Vocabulary::new(&["open", "closed"]).absent_as("open"))
                    .description("Where the ticket stands");
            })
            .field("summary", |f| {
                f.field_type(FieldType::String);
            });
    });
    c.edge("blocks", |e| {
        e.description("A ticket blocks another")
            .edge_style("dashed");
    });
    let build = build_registries(vec![c.declaration()]);

    let status = build.fields().get("ticket", "status").expect("registered");
    assert_eq!(status.declared().default_value.as_deref(), Some("open"));
    assert_eq!(status.declared().enum_values, ["open", "closed"]);
    assert_eq!(
        status.declared().description.as_deref(),
        Some("Where the ticket stands")
    );
    let summary = build.fields().get("ticket", "summary").expect("registered");
    assert_eq!(summary.declared().description, None);
    assert_eq!(summary.declared().default_value, None);
    let ticket = build.kinds().get("ticket").expect("registered");
    assert_eq!(ticket.declared.incremental, Some(true));
    assert_eq!(
        ticket.declared.inference_guide.as_deref(),
        Some("Look for issue trackers")
    );
    let blocks = build.edges().get("blocks").expect("registered");
    assert_eq!(
        blocks.declared().unwrap().edge_style.as_deref(),
        Some("dashed")
    );
    assert_eq!(blocks.source_extension(), "@acme/tickets");
}

/// A pass's handler that finds nothing.
fn no_findings(_: &PassInput) -> Vec<PassDiagnostic> {
    Vec::new()
}

#[specforge_test_macros::test(
    behavior = "cc_kind_result",
    verify = "a result field must be a status enum of a collector-written kind"
)]
fn a_result_field_must_be_a_status_enum_of_a_collector_written_kind() {
    let statuses = Vocabulary::new(&["passed", "failed", "skipped"]);
    let build = build_registries(vec![crate::support::declare("@acme/code", |c| {
        c.kind("test", |k| {
            k.collected(&["js"]).result("status").field("status", |f| {
                f.vocabulary(&statuses);
            });
        });
        c.kind("run", |k| {
            k.collected(&["run"])
                .result("outcome")
                .field("outcome", |f| {
                    f.vocabulary(&Vocabulary::new(&["passed", "flaky"]));
                });
        });
        c.kind("check", |k| {
            k.collected(&["chk"]).result("note").field("note", |f| {
                f.field_type(FieldType::String);
            });
        });
        c.kind("lint", |k| {
            k.collected(&["lint"]).result("missing");
        });
        c.kind("case", |k| {
            k.result("status").field("status", |f| {
                f.vocabulary(&statuses);
            });
        });
    })]);

    assert_eq!(build.kinds().get("test").unwrap().result(), Some("status"));
    for kind in ["run", "check", "lint", "case"] {
        assert_eq!(build.kinds().get(kind).unwrap().result(), None, "{kind}");
    }
    let mut refused: Vec<&str> = crate::support::coded(&build, "W021")
        .into_iter()
        .map(|d| d.message.as_str())
        .collect();
    refused.sort();
    assert_eq!(
        refused,
        [
            "extension '@acme/code': kind 'case' declares result 'status': only a fact has a result of its own, and no collector writes the kind",
            "extension '@acme/code': kind 'check' declares result 'note': it is not an enum field",
            "extension '@acme/code': kind 'lint' declares result 'missing': it is not one of its fields",
            "extension '@acme/code': kind 'run' declares result 'outcome': its values [flaky] are not test statuses (passed, failed, skipped)",
        ]
    );
}

#[specforge_test_macros::test(
    behavior = "cc_owner_pending",
    verify = "complete_when names an enum value of its kind"
)]
fn complete_when_names_an_enum_value_of_its_kind() {
    let states = Vocabulary::new(&["draft", "complete"]);
    let build = build_registries(vec![crate::support::declare("@acme/plans", |c| {
        c.kind("plan", |k| {
            k.complete_when("status", &["complete"])
                .field("status", |f| {
                    f.vocabulary(&states);
                });
        });
        c.kind("goal", |k| {
            k.complete_when("status", &["done"]).field("status", |f| {
                f.vocabulary(&states);
            });
        });
        c.kind("task", |k| {
            k.complete_when("title", &["x"]).field("title", |f| {
                f.field_type(FieldType::String);
            });
        });
        c.kind("step", |k| {
            k.complete_when("state", &["complete"]);
        });
        c.kind("epic", |k| {
            k.complete_when("status", &[]).field("status", |f| {
                f.vocabulary(&states);
            });
        });
    })]);

    let plan = build.kinds().get("plan").unwrap().complete_when().unwrap();
    assert_eq!(
        (plan.field.as_str(), &plan.values[..]),
        ("status", &["complete".to_string()][..])
    );
    for kind in ["goal", "task", "step", "epic"] {
        assert!(
            build.kinds().get(kind).unwrap().complete_when().is_none(),
            "{kind}"
        );
    }
    let mut refused: Vec<&str> = crate::support::coded(&build, "W021")
        .into_iter()
        .map(|d| d.message.as_str())
        .collect();
    refused.sort();
    assert_eq!(
        refused,
        [
            "extension '@acme/plans': kind 'epic' declares complete_when 'status': it names no value, so no entity of the kind would ever be complete",
            "extension '@acme/plans': kind 'goal' declares complete_when 'status': [done] are not among its values [draft, complete]",
            "extension '@acme/plans': kind 'step' declares complete_when 'state': it is not one of its fields",
            "extension '@acme/plans': kind 'task' declares complete_when 'title': it is not an enum field",
        ]
    );
}

fn capability_problems_of(declare: impl FnOnce(&mut ContributionsBuilder)) -> Vec<String> {
    let mut c = declaration("@acme/caps", "1.0.0");
    declare(&mut c);
    let build = build_registries(vec![c.declaration()]);
    let found: Vec<String> = diagnostics(&build)
        .into_iter()
        .filter(|d| d.code == "W021")
        .map(|d| d.message.clone())
        .collect();
    found
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a capability that is not sound is W021 and grants nothing"
)]
fn capabilities_that_are_not_sound_are_w021_each() {
    let found = capability_problems_of(|c| {
        c.capability(Capability::git("Bad Name").why("read"));
        c.capability(Capability::git("twice").why("read"));
        c.capability(Capability::git("twice").why("read"));
        c.capability(Capability::git("silent"));
        c.capability(Capability::process("nothing").why("run"));
        c.capability(
            Capability::http("hosts")
                .hosts(&["not a host", "*.ok.example"])
                .why("fetch"),
        );
        c.capability(Capability::http("none").why("fetch"));
        c.capability(
            Capability::fs("writes")
                .ops(&[FsAccess::Write])
                .call_kinds(&[Operation::Pass])
                .why("write"),
        );
        c.capability(
            Capability::git("writes-any")
                .git_ops(&[GitOp::Write])
                .why("commit"),
        );
        c.capability(
            Capability::http("ftp")
                .hosts(&["a.example"])
                .schemes(&["ftp"])
                .why("fetch"),
        );
        c.capability(
            Capability::http("keyed")
                .from_values()
                .secret_header("Cookie", "missing", None)
                .why("fetch"),
        );
    });
    let expect = [
        "capability 'Bad Name' grants nothing: its name 'Bad Name' is not a scope name",
        "capability 'twice' grants nothing: its name 'twice' is declared twice",
        "capability 'silent' grants nothing: it says no `why`",
        "capability 'nothing' grants nothing: its `argv` is empty",
        "capability 'hosts' grants nothing: its host 'not a host' is not a host",
        "capability 'none' grants nothing: it lists no host",
        "capability 'writes' grants nothing: it writes, so its `call_kinds` must name only `collect` or `command`",
        "capability 'writes-any' grants nothing: it writes",
        "capability 'ftp' grants nothing: its scheme 'ftp' is neither `http` nor `https`",
        "capability 'keyed' grants nothing: its `secret_header` names the secret 'missing', which it does not declare",
        "capability 'keyed' grants nothing: its `secret_header` goes only to hosts it lists",
        "capability 'keyed' grants nothing: its `secret_header` needs `schemes` to be only `https`",
        "capability 'keyed' grants nothing: its `secret_header` header 'Cookie' is not a header",
    ];
    for expected in expect {
        assert!(
            found.iter().any(|m| m.contains(expected)),
            "{expected}\nin {found:#?}"
        );
    }
    assert_eq!(found.len(), expect.len(), "{found:#?}");
}

#[specforge_test_macros::test(
    behavior = "build_registries_from_declarations",
    verify = "a collector whose capability names no process capability it declares is W021"
)]
fn capabilities_a_collector_names_must_be_a_process_one() {
    let found = capability_problems_of(|c| {
        c.capability(Capability::git("history").why("read"));
        c.collector("runner", |k| {
            k.capability("history")
                .collect(|_| Ok(CollectOutput::default()));
        });
    });
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains(
        "collector 'runner' cannot run: its capability 'history' is no `process` capability"
    ));
}
