//! A GSD-shaped extension that declares the route `requirement-thread` (ADR 0330), the one plan 04 gives `@gsd/core`:
//! from a requirement, its plans, their tasks, their commits, the tests its truths cite (with their `status`) and its
//! truths (with their E501 judgement). It is [`declare_gsd_judged`] and the kinds the route reaches beyond it, served
//! in process with [`CODE_CORE`]'s `test` facts and written as the wire carries them.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{Annotates, ContributionsBuilder, ExtensionMeta};
use specforge_wasm::testing::InProcessRuntime;

use super::judged::{
    CODE_CORE, GSD_JUDGED, code, code_core_builder, declare_gsd_judged, gsd_judged_passes,
    gsd_judged_wire,
};
use super::{TestProject, stored};

/// The id of the test the truth of [`GSD_ROUTED_SPEC`] cites, a fact of the scheme `js`.
macro_rules! routed_test {
    () => {
        "js:app/test/auth.test.js#keeps a session"
    };
}

/// The id of the test the truth of [`GSD_ROUTED_SPEC`] cites, a fact of the scheme `js`.
pub const ROUTED_TEST: &str = routed_test!();

/// The key of the `node` store [`gsd_routed_project`] writes.
const ROUTED_STORE: &str = "0123456789abcdef0123456789abcdef01234567";

/// [`gsd_judged_wire`], with a truth's `requirement` and the kind `commit`.
pub fn gsd_routed_wire() -> Value {
    let mut kinds = gsd_judged_wire();
    let list = kinds.as_array_mut().expect("the kinds are a list");
    let truth = list
        .iter_mut()
        .find(|kind| kind["name"] == "truth")
        .expect("the judged kinds declare a truth");
    truth["fields"]
        .as_array_mut()
        .expect("a kind lists fields")
        .push(
            json!({"name": "requirement", "field_type": "reference", "target_kind": "requirement"}),
        );
    list.push(json!({"name": "commit", "keyword": "commit", "fields": [
        {"name": "task", "field_type": "reference", "target_kind": "task"},
        {"name": "linked_by", "field_type": "string"},
    ]}));
    kinds
}

/// The facts a collect of [`GSD_ROUTED_SPEC`]'s project stores (as the wire carries them): the test the truth cites,
/// passed.
pub fn gsd_routed_facts() -> Value {
    json!([{"kind": "test", "id": ROUTED_TEST,
            "fields": {"file": "app/test/auth.test.js", "name": "keeps a session", "status": "passed"}}])
}

/// The route `requirement-thread`, as plan 04 declares it.
pub fn declare_requirement_thread(c: &mut ContributionsBuilder) {
    c.route("requirement-thread", |r| {
        r.start("requirement")
            .description("A requirement's thread: its plans, tasks, commits, tests and truths")
            .step("plan", "<plan.requirements", |s| {
                s.label("plan");
            })
            .step("task", "<task.plan", |_| {})
            .step("commit", "<commit.task", |s| {
                s.field("linked_by");
            })
            .step("test", "<truth.requirement.proof", |s| {
                s.from_start().annotate(|a| {
                    a.field("status").absent("not run");
                });
            })
            .step("truth", "<truth.requirement", |s| {
                s.from_start().judgement(&code('E', 501));
            });
    });
}

/// The kinds, the rules, the agents, the setting and the route of `@gsd/core`, as `c` contributes them (no pass).
pub fn declare_gsd_routed(c: &mut ContributionsBuilder) {
    declare_gsd_judged(c);
    // A raw category replaces the one given before: the judged kinds, with what the route reaches beyond them.
    c.raw_category(DeclaredCategory::Entities, gsd_routed_wire());
    declare_requirement_thread(c);
}

/// The extension: [`declare_gsd_routed`] and the compute pass `plan_status`, under the short name `gsd` the real
/// `@gsd/core` routes its commands by (so its route is `gsd/requirement-thread`).
pub fn gsd_routed_builder() -> ContributionsBuilder {
    let mut meta = ExtensionMeta::new(GSD_JUDGED, "0.1.0");
    meta.short = Some("gsd".to_string());
    let mut c = ContributionsBuilder::new(meta);
    declare_gsd_routed(&mut c);
    c.raw_category(DeclaredCategory::Passes, gsd_judged_passes());
    c
}

/// `build` (the extension, [`gsd_routed_builder`] or one made from it) and [`CODE_CORE`] served in process, with the
/// answer of its compute pass `plan_status`.
pub fn gsd_routed_serving(
    build: impl Fn() -> ContributionsBuilder + Send + Sync + 'static,
) -> InProcessRuntime {
    InProcessRuntime::new()
        .with(build)
        .with(code_core_builder)
        .answer_raw(
            GSD_JUDGED,
            "__pass_plan_status",
            specforge_wasm::WasmCallResult::Ok(
                json!({"diagnostics": [], "summary": {}, "values": []})
                    .to_string()
                    .into_bytes(),
            ),
        )
}

/// `@gsd/core` with the route, served in process.
pub fn gsd_routed_runtime() -> InProcessRuntime {
    gsd_routed_serving(gsd_routed_builder)
}

/// One requirement's whole thread: a plan, its task, the task's commit, and the truth that proves the requirement and
/// cites [`ROUTED_TEST`]. The truth has no verdict.
pub const GSD_ROUTED_SPEC: &str = concat!(
    r#"milestone v1.0 "One" {}

requirement AUTH-07 "Sessions" {}

phase 03 "Three" {
  milestone v1.0
  requirements [AUTH-07]
}

plan 03-02 "Session tokens" {
  phase 03
  requirements [AUTH-07]
  files_modified ["src/auth/session.ts"]
}

task T1 "Task" {
  plan 03-02
}

commit 5e2a91c0d3b4 "Keep the session" {
  task v1.0/03/03-02/T1
  linked_by "Gsd-Task"
}

truth TR1 "Tokens survive a reload" {
  plan 03-02
  requirement AUTH-07
  proof [""#,
    routed_test!(),
    "\"]\n}\n"
);

/// A project over `@gsd/core` with the route and [`CODE_CORE`], whose spec is `spec`, with the store of
/// [`gsd_routed_facts`] a collect leaves.
pub fn gsd_routed_project(spec: &str) -> TestProject {
    let project = gsd_routed_project_over(spec, &[GSD_JUDGED, CODE_CORE]);
    stored(project.root(), "node", ROUTED_STORE, gsd_routed_facts());
    project
}

/// A project over `extensions` (`@gsd/core` with the route, and [`CODE_CORE`] when it is named), whose spec is
/// `spec`, with the instructions file `CLAUDE.md` and the source file `src/auth/session.ts` the judged rules read, and
/// no fact store.
pub fn gsd_routed_project_over(spec: &str, extensions: &[&str]) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(extensions)
        .installing_enabled()
        .file("a.spec", spec)
        .file("CLAUDE.md", "# Instructions\n")
        .file("src/auth/session.ts", "export const session = 1;\n")
}
