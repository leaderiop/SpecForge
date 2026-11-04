//! What one run of the values phase may spend (ADR 0336 D4): over the budget a value is not run, counts toward I206 as
//! `budget`, and nothing goes red for it. A judge call stays inside what its guest may hold.

use std::sync::Arc;
use std::time::Duration;

use specforge_effects::testing::{FakeHttp, MemoryConsent};
use specforge_effects::{EffectsMode, OutcomeClass};
use specforge_extension_sdk::prelude::*;
use specforge_project::CompiledProject;
use specforge_project::values::{CheckStatus, ValueBudget};
use specforge_protocol_types::SandboxPolicy;
use specforge_wasm::testing::InProcessRuntime;

use crate::value_probes::{EXT, Scene, coded, described, judged_by_class};

fn url(n: usize) -> String {
    format!("https://a.example/{n}")
}

/// `service`: `one` GETs its value, `many` GETs it 17 times over, `eight` 8 times, `file` reads a file.
fn extension(meta: ExtensionMeta) -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(meta);
    described(&mut c);
    c.capability(Capability::http("links").from_values().why("to ask"));
    c.capability(
        Capability::fs("files")
            .ops(&[FsAccess::Stat, FsAccess::Read])
            .why("to read"),
    );
    let get = |url: String, n: usize| Probe::HttpGet {
        url: format!("{url}?{n}"),
        range: None,
        max_bytes: 64 * 1024,
    };
    for (name, times) in [("one", 1), ("eight", 8), ("many", 17)] {
        c.checker(name, move |k| {
            k.applies_to(&["string"])
                .scope("links")
                .codes(&["E001"])
                .plan(move |item: &CheckItem| {
                    let probes = (0..times).map(|n| get(item.text.clone(), n)).collect();
                    PlannedItem::probes(probes)
                })
                .judge(judged_by_class);
        });
    }
    c.checker("file", |k| {
        k.applies_to(&["string"])
            .scope("files")
            .codes(&["E001"])
            .plan(|item: &CheckItem| {
                PlannedItem::probes(vec![Probe::FsRead {
                    root: "project".to_string(),
                    path: item.text.clone(),
                    bytes: 1024,
                }])
            })
            .judge(judged_by_class);
    });
    c.kind("service", |k| {
        k.keyword("service");
        for field in ["one", "eight", "many", "file"] {
            k.field(field, |f| {
                f.field_type(FieldType::String).check(field);
            });
        }
    });
    c
}

fn plain() -> ContributionsBuilder {
    extension(ExtensionMeta::new(EXT, "1.0.0"))
}

fn scene_of(http: FakeHttp, spec: &str) -> Scene {
    Scene::with(http, spec, &[EXT], InProcessRuntime::new().with(plain))
}

fn services(field: &str, values: impl IntoIterator<Item = String>) -> String {
    values
        .into_iter()
        .enumerate()
        .map(|(n, value)| format!("service s{n} \"S\" {{\n  {field} \"{value}\"\n}}\n"))
        .collect()
}

/// `n` URLs on one host answering `body`, each asked once.
fn network(n: usize, body: &[u8]) -> FakeHttp {
    (0..n).fold(FakeHttp::new(), |http, at| {
        http.route(&format!("{}?0", url(at)), 200, body)
    })
}

fn within(scene: &Scene, budget: ValueBudget) -> CompiledProject {
    scene.checked_within(EffectsMode::Online, &MemoryConsent::default(), budget)
}

fn statuses(project: &CompiledProject) -> Vec<CheckStatus> {
    project
        .value_checks()
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|row| row.status))
        .collect()
}

fn asks_for_budget(project: &CompiledProject) -> String {
    let found = project.diagnostics();
    let summary = coded(&found, "I206");
    assert_eq!(summary.len(), 1, "{found:?}");
    assert!(
        summary[0].message.contains("budget"),
        "{}",
        summary[0].message
    );
    summary[0].message.clone()
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "what is over a run's budget is not run and counts toward I206 as budget"
)]
fn a_value_asking_too_many_probes_is_not_run() {
    let scene = scene_of(FakeHttp::new(), &services("many", [url(0)]));
    let project = within(&scene, ValueBudget::RUN);
    assert!(scene.http.requests().is_empty(), "none of them was asked");
    assert_eq!(statuses(&project), [CheckStatus::NotRun]);
    assert!(asks_for_budget(&project).contains("budget 1"));
}

#[test]
fn past_the_runs_time_no_request_starts() {
    let scene = scene_of(network(3, b"x"), &services("one", (0..3).map(url)));
    let budget = ValueBudget {
        time: Duration::ZERO,
        ..ValueBudget::RUN
    };
    let project = within(&scene, budget);
    assert!(scene.http.requests().is_empty());
    assert!(statuses(&project).iter().all(|s| *s == CheckStatus::NotRun));
    assert!(asks_for_budget(&project).contains("budget 3"));
}

#[test]
fn past_the_runs_bytes_no_request_starts() {
    const VALUES: usize = 40;
    let scene = scene_of(
        network(VALUES, b"xx"),
        &services("one", (0..VALUES).map(url)),
    );
    let budget = ValueBudget {
        bytes: 1,
        ..ValueBudget::RUN
    };
    let project = within(&scene, budget);
    // One host holds 8 at once: what was on the wire when the first answer spent the budget.
    let asked = scene.http.requests().len();
    assert!((1..=8).contains(&asked), "{asked}");
    let statuses = statuses(&project);
    let passed = statuses.iter().filter(|s| **s == CheckStatus::Passed);
    assert_eq!(passed.count(), asked);
    let over = VALUES - asked;
    assert!(asks_for_budget(&project).contains(&format!("budget {over}")));
}

#[test]
fn the_local_probes_have_their_own_budget() {
    let names = ["a.md", "b.md", "c.md"];
    let scene = scene_of(FakeHttp::new(), &services("file", names.map(String::from)));
    for name in names {
        scene.dir.write(name, "x");
    }
    let budget = ValueBudget {
        local_probes: 1,
        ..ValueBudget::EDITOR
    };
    let project = scene.checked_within(EffectsMode::Cached, &MemoryConsent::default(), budget);
    let statuses = statuses(&project);
    assert_eq!(
        statuses
            .iter()
            .filter(|s| **s == CheckStatus::Passed)
            .count(),
        1,
        "{statuses:?}"
    );
    assert!(asks_for_budget(&project).contains("budget 2"));

    let spent = ValueBudget {
        bytes: 1,
        ..ValueBudget::EDITOR
    };
    let project = scene.checked_within(EffectsMode::Cached, &MemoryConsent::default(), spent);
    assert!(asks_for_budget(&project).contains("budget 2"));
}

/// [`plain`] in a guest of 1 MiB: a call carries at most 256 KiB.
fn small() -> ContributionsBuilder {
    let mut meta = ExtensionMeta::new(EXT, "1.0.0");
    meta.sandbox_policy = Some(SandboxPolicy {
        max_memory_mb: Some(1),
        max_execution_ms: None,
    });
    extension(meta)
}

/// What the guest was asked to check in one call each, in order.
fn check_calls(runtime: &InProcessRuntime) -> usize {
    runtime
        .calls()
        .iter()
        .filter(|call| call.export == "__check_values")
        .count()
}

#[specforge_test_macros::test(
    behavior = "check_field_values",
    verify = "a judge call stays inside its guest's memory limit"
)]
fn the_judge_call_is_batched_to_what_its_guest_can_hold() {
    const VALUES: usize = 20;
    let body = vec![b'x'; 16 * 1024];
    let http = network(VALUES, &body);
    let runtime = Arc::new(InProcessRuntime::new().with(small));
    let spec = services("one", (0..VALUES).map(url));
    let scene = Scene::over(http, &spec, &[EXT], Arc::clone(&runtime));
    runtime.clear_calls();
    let project = scene.online();
    assert!(statuses(&project).iter().all(|s| *s == CheckStatus::Passed));
    // Each item is about 48 KiB of JSON, so about 5 fit a call: one plan, and at least 4 judges.
    assert!(check_calls(&runtime) >= 5, "{}", check_calls(&runtime));
}

#[test]
fn an_item_no_call_could_hold_is_not_asked() {
    let body = vec![b'x'; 64 * 1024];
    let http = (0..8).fold(FakeHttp::new(), |http, at| {
        http.route(&format!("{}?{at}", url(0)), 200, body.clone())
    });
    let runtime = Arc::new(InProcessRuntime::new().with(small));
    let spec = services("eight", [url(0)]);
    let scene = Scene::over(http, &spec, &[EXT], Arc::clone(&runtime));
    let project = scene.online();
    assert_eq!(statuses(&project), [CheckStatus::NotRun]);
    let row = &project.value_checks().of("s0")[0];
    assert_eq!(row.outcome_class, None::<OutcomeClass>);
    assert!(asks_for_budget(&project).contains("budget 1"));
    let found = project.diagnostics();
    assert!(coded(&found, "E028").is_empty(), "{found:?}");
}
