//! The routes `specforge trace` follows (ADR 0330): checked once at the registry build, E118 for one that cannot be
//! followed, W195 for an ambiguous default, and a part of an unloaded optional peer skipped without a word.

use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{FieldType, Parent, RouteAnnotation, Skipped};
use specforge_test_macros::test as spec;

use crate::support::{build, build_with, candidates, coded, declare, optional_peer};

/// `@acme/work`: a `need`, the `plan`s that answer it, their `task`s and the `claim`s on a task (with a `status` and a
/// computed reference); a need names its `checks`, `@acme/proof`'s kind; the judged rule W901 judges a need. `f` adds
/// the routes.
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
            .field("status", |f| {
                f.field_type(FieldType::String);
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
        c.agent("acme-judge", |a| {
            a.description("Judges a need");
        });
        c.rule("W901", |r| {
            r.severity(ValidationSeverity::Warning)
                .message_template("{id} is not met")
                .judged("acme-judge", |j| {
                    j.outcomes(&["met", "unmet"])
                        .passing(&["met"])
                        .target("need", |t| {
                            t.input("need", "self");
                        });
                });
        });
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

/// The need's thread: its plans, their tasks, the claims on those, and the need's checks.
fn need_thread(c: &mut ContributionsBuilder) {
    c.route("need-thread", |r| {
        r.start("need")
            .field("title")
            .step("plan", "<plan.need", |s| {
                s.label("plan");
            })
            .step("task", "<task.plan", |_| {})
            .step("claim", "<claim.task", |s| {
                s.field("status");
            })
            .step("check", ".checks", |s| {
                s.from_start();
            });
    });
}

#[spec(
    behavior = "declare_trace_route",
    verify = "a route that cannot be followed is E118 and is not registered"
)]
fn a_route_over_loaded_kinds_is_registered_with_its_steps_in_order() {
    let build = build([
        proof(),
        work(|c| {
            c.route("need-thread", |r| {
                r.start("need")
                    .field("title")
                    .judgement("W901")
                    .step("plan", "<plan.need", |s| {
                        s.label("plan");
                    })
                    .step("task", "<task.plan", |_| {})
                    .step("claim", "<claim.task", |s| {
                        s.field("status");
                    })
                    .step("check", ".checks", |s| {
                        s.from_start().annotate(|a| {
                            a.field("result").absent("unchecked");
                        });
                    });
            });
        }),
    ]);
    assert!(
        coded(&build, "E118").is_empty(),
        "{:?}",
        coded(&build, "E118")
    );
    let route = build.routes().followed("need").expect("the need's route");
    assert_eq!(route.name, "work/need-thread");
    assert_eq!(route.extension, "@acme/work");
    let steps: Vec<(&str, Parent, &str)> = route
        .steps
        .iter()
        .map(|s| (s.name.as_str(), s.from, s.title()))
        .collect();
    assert_eq!(
        steps,
        [
            ("plan", Parent::Start, "plan"),
            ("task", Parent::Step(0), "task"),
            ("claim", Parent::Step(1), "claim"),
            ("check", Parent::Start, "check"),
        ]
    );
    assert_eq!(
        route.annotations,
        [
            RouteAnnotation::Field {
                of: None,
                field: "title".to_string(),
                absent: None
            },
            RouteAnnotation::Judgement {
                code: "acme.work/W901".to_string(),
                absent: None
            },
        ]
    );
    assert_eq!(route.steps[3].annotations[0].absent(), Some("unchecked"));
    assert!(route.skipped.is_empty());
    assert_eq!(build.routes().named("work/need-thread"), Some(route));
    assert_eq!(build.routes().of_kind("need").count(), 1);
}

#[spec(
    behavior = "declare_trace_route",
    verify = "a route that cannot be followed is E118 and is not registered"
)]
fn a_route_naming_an_unknown_kind_field_or_a_non_reference_is_e118_and_not_registered() {
    type Declare = fn(&mut ContributionsBuilder);
    let cases: &[(&str, Declare)] = &[
        ("start kind 'ghost' is not loaded", |c| {
            c.route("r", |r| {
                r.start("ghost").step("plan", "<plan.need", |_| {});
            });
        }),
        ("kind 'ghost' is not loaded", |c| {
            c.route("r", |r| {
                r.start("need").step("x", "<ghost.x", |_| {});
            });
        }),
        ("kind 'plan' declares no field 'nope'", |c| {
            c.route("r", |r| {
                r.start("need").step("x", "<plan.nope", |_| {});
            });
        }),
        ("'need.title' is not a reference", |c| {
            c.route("r", |r| {
                r.start("need").step("x", ".title", |_| {});
            });
        }),
        ("selector steps start with '.' or '<'", |c| {
            c.route("r", |r| {
                r.start("need").step("x", "plan", |_| {});
            });
        }),
        ("from 'task' names no earlier step", |c| {
            c.route("r", |r| {
                r.start("need")
                    .step("plan", "<plan.need", |s| {
                        s.from_step("task");
                    })
                    .step("task", "<task.plan", |_| {});
            });
        }),
        (
            "a step's name is a word, unique in the route, and not 'start'",
            |c| {
                c.route("r", |r| {
                    r.start("need").step("plan", "<plan.need", |_| {}).step(
                        "plan",
                        "<task.plan",
                        |_| {},
                    );
                });
            },
        ),
        (
            "a step's name is a word, unique in the route, and not 'start'",
            |c| {
                c.route("r", |r| {
                    r.start("need").step("start", "<plan.need", |_| {});
                });
            },
        ),
        ("a route has at least one step", |c| {
            c.route("r", |r| {
                r.start("need");
            });
        }),
        ("a route's name is kebab case", |c| {
            c.route("Need_Thread", |r| {
                r.start("need").step("plan", "<plan.need", |_| {});
            });
        }),
        ("exactly one of field and judgement", |c| {
            c.route("r", |r| {
                r.start("need").annotate(|a| {
                    a.field("title").judgement("W901");
                });
                r.step("plan", "<plan.need", |_| {});
            });
        }),
        ("kind 'plan' declares no field 'status'", |c| {
            c.route("r", |r| {
                r.start("need").step("plan", "<plan.need", |s| {
                    s.field("status");
                });
            });
        }),
        ("'acme.work/W999' names no judged rule", |c| {
            c.route("r", |r| {
                r.start("need").judgement("W999");
                r.step("plan", "<plan.need", |_| {});
            });
        }),
        (
            "judged rule 'acme.work/W901' does not judge kind 'plan'",
            |c| {
                c.route("r", |r| {
                    r.start("need").step("plan", "<plan.need", |s| {
                        s.judgement("W901");
                    });
                });
            },
        ),
        ("it makes no edge, so a route cannot follow it", |c| {
            c.route("r", |r| {
                r.start("task").step("claim", "<claim.derived", |_| {});
            });
        }),
    ];
    for (why, declare_route) in cases {
        let build = build([proof(), work(declare_route)]);
        let e118 = coded(&build, "E118");
        assert_eq!(e118.len(), 1, "{why}: {e118:#?}");
        assert!(e118[0].message.contains(why), "{why}: {}", e118[0].message);
        assert!(
            e118[0]
                .message
                .starts_with("extension '@acme/work': route '")
        );
        assert_eq!(
            e118[0].suggestion.as_deref(),
            Some("fix the route's declaration in @acme/work")
        );
        assert_eq!(build.routes().iter().count(), 0, "{why}: not registered");
    }
    // A second route of one name is refused, the first kept.
    let build = build([
        proof(),
        work(|c| {
            need_thread(c);
            need_thread(c);
        }),
    ]);
    let e118 = coded(&build, "E118");
    assert_eq!(e118.len(), 1, "{e118:#?}");
    assert!(e118[0].message.contains("unique in its extension"));
    assert_eq!(build.routes().iter().count(), 1);
}

#[spec(
    behavior = "declare_trace_route",
    verify = "a route step of an optional peer the project does not load is skipped, not reported"
)]
fn a_step_of_an_unloaded_optional_peer_is_skipped_silently_and_recorded() {
    let declare_routes = |c: &mut ContributionsBuilder| {
        c.route("need-thread", |r| {
            r.start("need")
                .step("plan", "<plan.need", |s| {
                    s.annotate(|a| {
                        a.field("result").of(".need.checks");
                    });
                })
                .step("check", ".checks", |s| {
                    s.from_start();
                })
                .step("again", "<plan.need", |s| {
                    s.from_step("check");
                })
                .step("task", "<task.plan", |s| {
                    s.from_step("plan");
                });
        });
        c.route("check-thread", |r| {
            r.start("check").step("need", "<need.checks", |_| {});
        });
    };
    // `@acme/proof`, which declares `check`, is a candidate the host could enable.
    let built = build_with(
        [work_with_proof_peer(declare_routes)],
        candidates(&[proof()]),
    );
    assert!(
        coded(&built, "E118").is_empty(),
        "{:?}",
        coded(&built, "E118")
    );
    let routes = built.routes();
    assert!(
        routes.followed("check").is_none(),
        "its start kind is the peer's: dropped"
    );
    let route = routes.followed("need").expect("registered");
    let steps: Vec<(&str, Parent)> = route
        .steps
        .iter()
        .map(|s| (s.name.as_str(), s.from))
        .collect();
    assert_eq!(steps, [("plan", Parent::Start), ("task", Parent::Step(0))]);
    assert!(route.steps[0].annotations.is_empty());
    let skipped = |step: &str, annotation: Option<&str>| Skipped {
        step: step.to_string(),
        annotation: annotation.map(str::to_string),
        kind: "check".to_string(),
    };
    assert_eq!(
        route.skipped,
        [
            skipped("plan", Some("result")),
            skipped("check", None),
            skipped("again", None)
        ]
    );
    let parts: Vec<String> = route.skipped.iter().map(ToString::to_string).collect();
    assert_eq!(
        parts,
        [
            "annotation 'result' of step 'plan'",
            "step 'check'",
            "step 'again'"
        ]
    );
    // Without the optional peer the same route is E118, and so it is with one the vocabulary does not credit
    // with `check` (a misspelt kind is no peer's).
    for built in [
        build_with([work(declare_routes)], candidates(&[proof()])),
        build([work_with_proof_peer(declare_routes)]),
    ] {
        assert_eq!(
            coded(&built, "E118").len(),
            2,
            "{:?}",
            coded(&built, "E118")
        );
    }
}

#[spec(
    behavior = "declare_trace_route",
    verify = "several routes from one kind without one default are W195, and the first in load order is followed"
)]
fn two_routes_from_one_kind_without_one_default_are_w195_and_the_first_is_followed() {
    let build = build([
        proof(),
        work(|c| {
            need_thread(c);
            c.route("need-plans", |r| {
                r.start("need").step("plan", "<plan.need", |_| {});
            });
        }),
    ]);
    let w195 = coded(&build, "W195");
    assert_eq!(w195.len(), 1, "{w195:#?}");
    assert_eq!(
        w195[0].message,
        "kind 'need' starts 2 trace routes (work/need-thread, work/need-plans) and none is marked default: \
         'work/need-thread' is followed"
    );
    assert_eq!(
        build.routes().followed("need").unwrap().name,
        "work/need-thread"
    );
    assert_eq!(build.routes().of_kind("need").count(), 2);
}

#[spec(
    behavior = "declare_trace_route",
    verify = "several routes from one kind without one default are W195, and the first in load order is followed"
)]
fn one_default_among_several_routes_is_followed_without_a_warning() {
    let build = build([
        proof(),
        work(|c| {
            need_thread(c);
            c.route("need-plans", |r| {
                r.start("need")
                    .default_route()
                    .step("plan", "<plan.need", |_| {});
            });
        }),
    ]);
    assert!(coded(&build, "W195").is_empty());
    assert_eq!(
        build.routes().followed("need").unwrap().name,
        "work/need-plans"
    );
}
