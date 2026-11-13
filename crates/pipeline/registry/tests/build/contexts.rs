//! The context routes a role reads (ADR 0356): checked once at the registry build, E123 for one that cannot be
//! served, E124 for an agent binding that cannot hold, and a section of an unloaded optional peer skipped and
//! recorded.

use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::{ContextTier, ExtensionDeclaration};
use specforge_registry::SkippedSection;
use specforge_test_macros::test as spec;

use crate::support::{build, build_with, candidates, coded, declare, optional_peer};

/// `@acme/work`: a `need`, the `plan`s that answer it, their `task`s and the `claim`s on a task (one computed
/// reference); a need names its `checks`, `@acme/proof`'s kind. It declares the agents `acme-runner` and
/// `acme-judge`, the setting `strict` and the secret `token`. `f` adds the context routes.
fn work(f: impl FnOnce(&mut ContributionsBuilder)) -> ExtensionDeclaration {
    declare("@acme/work", |c| {
        c.kind("need", |k| {
            k.field("title", |f| {
                f.field_type(FieldType::String);
            })
            .field("checks", |f| {
                f.field_type(FieldType::ReferenceList).target_kind("check");
            });
        });
        c.kind("plan", |k| {
            k.field("need", |f| {
                f.field_type(FieldType::Reference).target_kind("need");
            })
            .field("files", |f| {
                f.field_type(FieldType::StringList);
            });
        });
        c.kind("task", |k| {
            k.field("plan", |f| {
                f.field_type(FieldType::Reference).target_kind("plan");
            });
        });
        c.kind("claim", |k| {
            k.field("task", |f| {
                f.field_type(FieldType::Reference).target_kind("task");
            })
            .field("derived", |f| {
                f.field_type(FieldType::Reference)
                    .target_kind("task")
                    .computed();
            });
        });
        c.pass("derive", |p| {
            p.phase("compute")
                .computes(&["claim.derived"])
                .run(|_: &PassInput| Vec::<PassDiagnostic>::new());
        });
        for agent in ["acme-runner", "acme-judge"] {
            c.agent(agent, |a| {
                a.description("Works on a plan");
            });
        }
        c.setting("strict", |s| {
            s.bool().default(false).describe("Be strict");
        });
        c.secret("token", "ACME_TOKEN", "The API token");
        f(c);
    })
}

/// `work` that declares `@acme/proof` an optional peer.
fn work_with_proof_peer(f: impl FnOnce(&mut ContributionsBuilder)) -> ExtensionDeclaration {
    let mut declaration = work(f);
    declaration
        .handshake
        .peer_dependencies
        .push(optional_peer("@acme/proof", "^1.0.0"));
    declaration
}

/// `@acme/proof`: the `check` a need names.
fn proof() -> ExtensionDeclaration {
    declare("@acme/proof", |c| {
        c.kind("check", |k| {
            k.field("result", |f| {
                f.field_type(FieldType::String);
            });
        });
    })
}

/// What a runner reads of a plan.
fn runner(c: &mut ContributionsBuilder) {
    c.context("runner", |r| {
        r.subject("plan")
            .agent("acme-runner")
            .budget(4000)
            .core("plan", "self")
            .core("tasks", "self<task.plan")
            .enrich("need", "self.need")
            .enrich("checks", "self.need.checks")
            .enrich("code", "file(self.files)")
            .enrich("instructions", "file(\"CLAUDE.md\")")
            .settings(&["strict"]);
    });
}

#[spec(
    behavior = "declare_context_route",
    verify = "a context route that cannot be served is E123 and is not registered"
)]
fn a_context_route_over_loaded_kinds_is_registered_with_its_sections_in_order() {
    let build = build([proof(), work(runner)]);
    assert!(
        coded(&build, "E123").is_empty(),
        "{:?}",
        coded(&build, "E123")
    );
    assert!(
        coded(&build, "E124").is_empty(),
        "{:?}",
        coded(&build, "E124")
    );
    let route = build.contexts().named("work/runner").expect("registered");
    assert_eq!(route.extension, "@acme/work");
    assert_eq!(route.subject, "plan");
    assert_eq!(route.budget, 4000);
    assert_eq!(route.settings, ["strict"]);
    let sections: Vec<(&str, ContextTier, String)> = route
        .sections
        .iter()
        .map(|s| (s.name.as_str(), s.tier, s.selector.to_string()))
        .collect();
    assert_eq!(
        sections,
        [
            ("plan", ContextTier::Core, "self".to_string()),
            ("tasks", ContextTier::Core, "self<task.plan".to_string()),
            ("need", ContextTier::Enrich, "self.need".to_string()),
            (
                "checks",
                ContextTier::Enrich,
                "self.need.checks".to_string()
            ),
            ("code", ContextTier::Enrich, "file(self.files)".to_string()),
            (
                "instructions",
                ContextTier::Enrich,
                "file(\"CLAUDE.md\")".to_string()
            ),
        ]
    );
    assert!(route.skipped.is_empty());
    assert_eq!(
        build.contexts().of_agent("acme-runner").map(|r| &r.name),
        Some(&route.name)
    );
    assert!(build.contexts().of_agent("acme-judge").is_none());
    // A project whose extensions declare no context route has none.
    assert!(build_with([proof()], candidates(&[])).contexts().is_empty());
}

#[spec(
    behavior = "declare_context_route",
    verify = "a context route that cannot be served is E123 and is not registered"
)]
fn a_context_route_that_cannot_be_served_is_e123_and_not_registered() {
    type Declare = fn(&mut ContributionsBuilder);
    let cases: &[(&str, &str, Declare)] = &[
        ("Run_Plan", "a context route's name is kebab case", |c| {
            c.context("Run_Plan", |r| {
                r.subject("plan").budget(10).core("plan", "self");
            });
        }),
        ("r", "a context route's budget is a positive number", |c| {
            c.context("r", |r| {
                r.subject("plan").core("plan", "self");
            });
        }),
        ("r", "at least one core section", |c| {
            c.context("r", |r| {
                r.subject("plan").budget(10).enrich("plan", "self");
            });
        }),
        ("r", "at least one core section", |c| {
            c.context("r", |r| {
                r.subject("plan").budget(10);
            });
        }),
        ("r", "subject kind 'ghost' is not loaded", |c| {
            c.context("r", |r| {
                r.subject("ghost").budget(10).core("plan", "self");
            });
        }),
        ("r", "section 'the plan': a section's name is a word", |c| {
            c.context("r", |r| {
                r.subject("plan").budget(10).core("the plan", "self");
            });
        }),
        (
            "r",
            "section 'plan': a section's name is a word, unique",
            |c| {
                c.context("r", |r| {
                    r.subject("plan")
                        .budget(10)
                        .core("plan", "self")
                        .enrich("plan", "self.need");
                });
            },
        ),
        ("r", "not 'settings'", |c| {
            c.context("r", |r| {
                r.subject("plan").budget(10).core("settings", "self");
            });
        }),
        ("r", "section 'x': selector 'self..need'", |c| {
            c.context("r", |r| {
                r.subject("plan").budget(10).core("x", "self..need");
            });
        }),
        ("r", "path '../out.txt' leaves the project root", |c| {
            c.context("r", |r| {
                r.subject("plan")
                    .budget(10)
                    .core("x", "file(\"../out.txt\")");
            });
        }),
        ("r", "kind 'plan' declares no field 'nope'", |c| {
            c.context("r", |r| {
                r.subject("plan").budget(10).core("x", "self.nope");
            });
        }),
        ("r", "kind 'ghost' is not loaded", |c| {
            c.context("r", |r| {
                r.subject("plan").budget(10).core("x", "self<ghost.plan");
            });
        }),
        (
            "r",
            "it makes no edge, so a context section cannot follow it",
            |c| {
                c.context("r", |r| {
                    r.subject("task").budget(10).core("x", "self<claim.derived");
                });
            },
        ),
        ("r", "'nope' names no setting of @acme/work", |c| {
            c.context("r", |r| {
                r.subject("plan")
                    .budget(10)
                    .core("plan", "self")
                    .settings(&["nope"]);
            });
        }),
        ("r", "'token' names no setting of @acme/work", |c| {
            c.context("r", |r| {
                r.subject("plan")
                    .budget(10)
                    .core("plan", "self")
                    .settings(&["token"]);
            });
        }),
    ];
    for (route, why, declare_route) in cases {
        let built = build([proof(), work(declare_route)]);
        let e123 = coded(&built, "E123");
        assert_eq!(e123.len(), 1, "{why}: {e123:#?}");
        assert!(e123[0].message.contains(why), "{why}: {}", e123[0].message);
        assert!(
            e123[0].message.starts_with(&format!(
                "extension '@acme/work': context route '{route}': "
            )),
            "{}",
            e123[0].message
        );
        assert_eq!(
            e123[0].suggestion.as_deref(),
            Some("fix the context route's declaration in @acme/work")
        );
        assert!(built.contexts().is_empty(), "{why}: not registered");
    }
    // A second route of one name is refused, the first kept.
    let twice = build([
        proof(),
        work(|c| {
            runner(c);
            runner(c);
        }),
    ]);
    let e123 = coded(&twice, "E123");
    assert_eq!(e123.len(), 1, "{e123:#?}");
    assert!(e123[0].message.contains("unique in its extension"));
    assert_eq!(twice.contexts().iter().count(), 1);
    // An extension of another name under the same short name takes no route an earlier one registered.
    let mut other = work(runner);
    other.handshake.name = "@other/work".to_string();
    other.handshake.ext_short = Some("work".to_string());
    let mut first = work(runner);
    first.handshake.ext_short = Some("work".to_string());
    let shared = build([proof(), first, other]);
    let e123 = coded(&shared, "E123");
    assert!(
        e123.iter().any(|d| d
            .message
            .contains("'work/runner' is the name of an earlier extension's context route")),
        "{e123:#?}"
    );
    assert_eq!(shared.contexts().iter().count(), 1);
}

#[spec(
    behavior = "declare_context_route",
    verify = "a context section of an optional peer the project does not load is skipped and listed"
)]
fn a_section_of_an_unloaded_optional_peer_is_skipped_and_recorded() {
    let declare_routes = |c: &mut ContributionsBuilder| {
        runner(c);
        c.context("checker", |r| {
            r.subject("check").budget(10).core("check", "self");
        });
    };
    // `@acme/proof`, which declares `check`, is a candidate the host could enable.
    let built = build_with(
        [work_with_proof_peer(declare_routes)],
        candidates(&[proof()]),
    );
    assert!(
        coded(&built, "E123").is_empty(),
        "{:?}",
        coded(&built, "E123")
    );
    assert!(
        built.contexts().named("work/checker").is_none(),
        "its subject kind is the peer's: dropped"
    );
    let route = built.contexts().named("work/runner").expect("registered");
    let names: Vec<&str> = route.sections.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["plan", "tasks", "need", "code", "instructions"]);
    assert_eq!(
        route.skipped,
        [SkippedSection {
            section: "checks".to_string(),
            kind: "check".to_string(),
        }]
    );
}

#[spec(
    behavior = "declare_context_route",
    verify = "a context route's agent its extension does not declare, or another route binds, is E124 and the route stays"
)]
fn an_agent_its_extension_does_not_declare_or_another_route_binds_is_e124() {
    let build = build([
        proof(),
        work(|c| {
            runner(c);
            c.context("again", |r| {
                r.subject("plan")
                    .agent("acme-runner")
                    .budget(10)
                    .core("plan", "self");
            });
            c.context("stranger", |r| {
                r.subject("need")
                    .agent("someone-else")
                    .budget(10)
                    .core("need", "self");
            });
        }),
    ]);
    assert!(
        coded(&build, "E123").is_empty(),
        "{:?}",
        coded(&build, "E123")
    );
    let e124: Vec<&str> = coded(&build, "E124")
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(
        e124,
        [
            "extension '@acme/work': context route 'work/again': agent 'acme-runner' already reads the context \
             route 'work/runner' — the route was registered without its agent",
            "extension '@acme/work': context route 'work/stranger': the extension declares no agent \
             'someone-else' — the route was registered without its agent",
        ]
    );
    let contexts = build.contexts();
    assert_eq!(contexts.iter().count(), 3, "every route stays");
    assert_eq!(contexts.named("work/again").unwrap().agent, None);
    assert_eq!(contexts.named("work/stranger").unwrap().agent, None);
    assert_eq!(
        contexts.of_agent("acme-runner").map(|r| r.name.as_str()),
        Some("work/runner")
    );
}
