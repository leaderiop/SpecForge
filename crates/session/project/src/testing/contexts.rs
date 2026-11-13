//! A GSD-shaped project for context routes (ADR 0356): the taskboard twin's plan `v1.0/02/02-01` and the 45 entities
//! it reaches whose kinds the routed fixture declares, with the twin's ids and titles and only the fields that fixture
//! declares, so a budgeted read of the plan is the read an executor makes of the twin.

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::ContributionsBuilder;
use specforge_wasm::testing::InProcessRuntime;

use super::TestProject;
use super::judged::{CODE_CORE, GSD_JUDGED, PLAN_CHECK_INPUTS, gsd_judged_rules};
use super::routed::{
    gsd_routed_builder, gsd_routed_project_over, gsd_routed_serving, gsd_routed_wire,
};

/// The plan whose context the tests read.
pub const CONTEXT_PLAN: &str = "v1.0/02/02-01";

/// The taskboard twin's milestone, requirements, phases, plans and phase 02's decisions, context, deferred idea and
/// research, as the routed fixture's kinds take them.
pub const GSD_CONTEXT_SPEC: &str = r#"milestone v1.0 "Taskboard 1.0" {}

requirement LIST-01 "Filter the list" {}
requirement STORE-01 "Save after every change" {}
requirement STORE-02 "Titles survive a reload" {}
requirement TASK-01 "Add a task" {}
requirement TASK-02 "Complete a task" {}
requirement TASK-03 "List the open tasks" {}

phase 01 "Core" {
  milestone v1.0
  requirements [TASK-01, TASK-02]
}

phase 02 "Persistence" {
  milestone v1.0
  requirements [STORE-01, STORE-02]
}

phase 02.1 "Hotfix: titles survive a reload" {
  milestone v1.0
}

phase 03 "Listing" {
  milestone v1.0
  requirements [TASK-03, LIST-01]
}

plan 01-01 "The task model and add" {
  phase 01
  requirements [TASK-01]
  files_modified ["package.json", "src/tasks.js", "test/tasks.test.js"]
}

artifact AR1 "createBoard and addTask" {
  plan 01-01
  path "src/tasks.js"
}

key_link KL1 "The tests import addTask" {
  plan 01-01
  from "test/tasks.test.js"
  to "src/tasks.js"
}

task T1 "The task model" {
  plan 01-01
}

task T2 "Tests for add" {
  plan 01-01
}

truth TR1 "A task added to a board has the next integer id" {
  plan 01-01
  proof ["js:taskboard/test/tasks.test.js#a task is added with the next id"]
}

plan 01-02 "complete and the command line" {
  phase 01
  requirements [TASK-02]
  files_modified ["src/tasks.js", "src/cli.js", "test/tasks.test.js"]
}

artifact AR1 "taskboard add and taskboard done" {
  plan 01-02
  path "src/cli.js"
}

key_link KL1 "The command line imports the board" {
  plan 01-02
  from "src/cli.js"
  to "src/tasks.js"
}

task T1 "completeTask" {
  plan 01-02
}

task T2 "The command line" {
  plan 01-02
}

truth TR1 "A task is completed by its id" {
  plan 01-02
  proof ["js:taskboard/test/tasks.test.js#a task is completed by its id"]
}

truth TR2 "Completing a missing id fails with a message naming it" {
  plan 01-02
  proof ["js:taskboard/test/tasks.test.js#completing an unknown id throws"]
}

plan 02-01 "Save and load tasks.json" {
  phase 02
  requirements [STORE-01, STORE-02]
  files_modified ["src/store.js", "src/cli.js", "test/store.test.js"]
}

artifact AR1 "load and save of tasks.json" {
  plan 02-01
  path "src/store.js"
}

key_link KL1 "The command line imports the store" {
  plan 02-01
  from "src/cli.js"
  to "src/store.js"
}

task T1 "The store" {
  plan 02-01
}

task T2 "The command line saves" {
  plan 02-01
}

truth TR1 "Every change to the board is written to tasks.json" {
  plan 02-01
}

truth TR2 "A restarted board shows the tasks it had" {
  plan 02-01
}

truth TR3 "A title comes back exactly as it was written" {
  plan 02-01
}

context 02-CONTEXT "Persistence context" {
  phase 02
}

research 02-RESEARCH "Persistence research" {
  phase 02
}

dont_hand_roll DH1 "Locking" {
  research 02-RESEARCH
}

finding F1 "One JSON file, written then renamed" {
  research 02-RESEARCH
}

open_question OQ1 "Is one file enough?" {
  research 02-RESEARCH
}

pattern P1 "Write then rename" {
  research 02-RESEARCH
}

pattern P2 "Writing in place" {
  research 02-RESEARCH
}

pitfall PF1 "Torn writes" {
  research 02-RESEARCH
}

stack_choice SC1 "node:fs" {
  research 02-RESEARCH
}

decision D-01 "Tasks are kept in a single JSON file" {
  phase 02
  text "Tasks are kept in a single JSON file, `tasks.json`, in the working directory"
}

decision D-02 "The file is written then renamed" {
  phase 02
  text "The file is written to a temporary name and renamed over `tasks.json`, so a crash never leaves half a file"
}

decision D-03 "A title is stored as typed" {
  phase 02
  text "A title is stored exactly as typed, with no trimming"
}

deferred_idea DI1 "Keep two machines in step" {
  phase 02
}
"#;

/// The instructions file of [`gsd_context_project`].
pub const CONTEXT_INSTRUCTIONS: &str = "Write the store before the command line.\n";

/// A project over `@gsd/core` and [`CODE_CORE`] whose spec is [`GSD_CONTEXT_SPEC`], with the instructions file
/// `CLAUDE.md` and the files plan `02-01` modifies on disk.
pub fn gsd_context_project() -> TestProject {
    gsd_routed_project_over(GSD_CONTEXT_SPEC, &[GSD_JUDGED, CODE_CORE])
        .file("CLAUDE.md", CONTEXT_INSTRUCTIONS)
        .file("src/store.js", "")
        .file("src/cli.js", "")
        .file("test/store.test.js", "")
}

/// The plan's context record, as the wire carries a field written `context` (ADR 0356 D9).
pub const CONTEXT_PACK: &str = "context_pack";

/// The code a stale record of the executor's pack raises, as `@gsd/core` catalogues it.
pub const CONTEXT_STALE: &str = "W524";

/// [`gsd_routed_wire`] with the plan's context record [`CONTEXT_PACK`], checked while the plan is open, and the plan's
/// computed `status` an enum whose `complete` closes it, as the real `@gsd/core` declares them.
pub fn gsd_context_wire() -> Value {
    let mut kinds = gsd_routed_wire();
    let plan = kinds
        .as_array_mut()
        .expect("the kinds are a list")
        .iter_mut()
        .find(|kind| kind["name"] == "plan")
        .expect("the routed kinds declare a plan");
    plan["complete_when"] = json!({"field": "status", "values": ["complete"]});
    let fields = plan["fields"].as_array_mut().expect("a kind lists fields");
    let status = fields
        .iter_mut()
        .find(|field| field["name"] == "status")
        .expect("a plan has a status");
    *status = json!({"name": "status", "field_type": "enum", "writer": "computed",
        "enum_values": ["draft", "checked", "ready", "executing", "complete", "halted", "blocked"]});
    fields.push(
        json!({"name": CONTEXT_PACK, "field_type": "block", "writer": "context", "checks_while": "pending",
        "members": [
            {"name": "route", "field_type": "string"},
            {"name": "digest", "field_type": "string"},
            {"name": "max_tokens", "field_type": "integer"},
        ]}),
    );
    kinds
}

/// The executor's agent and two context routes of `@gsd/core`, as `c` contributes them, over the kinds of
/// [`gsd_context_wire`]: `executor`, what an executor
/// reads of a plan (core: the plan, its body, its phase and requirements; enrich: its phase's context, decisions, the
/// instructions, the research and the sibling plans; the `plan_check` setting), recorded in the plan's
/// [`CONTEXT_PACK`] and stale as [`CONTEXT_STALE`], and `plan-checker`, every input of the plan-check rules as a core
/// section.
pub fn declare_gsd_contexts(c: &mut ContributionsBuilder) {
    // A raw category replaces the one given before.
    c.raw_category(DeclaredCategory::Entities, gsd_context_wire());
    c.agent("gsd-executor", |a| {
        a.description("Executes a plan");
    });
    c.context("executor", |r| {
        r.subject("plan")
            .description("What an executor reads of a plan")
            .agent("gsd-executor")
            .budget(6000)
            .core("plan", "self")
            .core(
                "body",
                "self<task|checkpoint|truth|artifact|key_link|prohibition|user_setup.plan",
            )
            .core("phase", "self.phase")
            .core("requirements", "self.requirements")
            .enrich("context", "self.phase<context|deferred_idea.phase")
            .enrich("decisions", "self.phase<decision.phase")
            .enrich("instructions", "file(\"CLAUDE.md\", \"AGENTS.md\")")
            .enrich(
                "research_items",
                "self.phase<research.phase<finding|stack_choice|pattern|pitfall|dont_hand_roll|open_question.research",
            )
            .enrich("siblings", "self.phase<plan.phase")
            .settings(&["plan_check"])
            .record_suggesting(
                "plan.context_pack",
                "self",
                "run `specforge context {route} {subject} --record {entity}` again",
            )
            .stale(CONTEXT_STALE);
    });
    // The judged rules' catalogue is a raw category, which this one replaces: their codes, then the stale code.
    let mut catalog: Vec<Value> = gsd_judged_rules()
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|rule| rule["code"].as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|code| json!({"code": code, "title": "A test rule", "explanation": "A test rule's finding."}))
        .collect();
    catalog.push(json!({"code": CONTEXT_STALE, "title": "Stale context pack",
        "explanation": "The pack recorded on the entity is not the pack its route serves now."}));
    c.raw_category(DeclaredCategory::Codes, Value::Array(catalog));
    c.context("plan-checker", |r| {
        r.subject("plan")
            .description("What the plan checker reads of a plan")
            .agent("gsd-plan-checker")
            .budget(8000);
        for (name, selector) in PLAN_CHECK_INPUTS {
            r.core(name, selector);
        }
    });
}

/// [`gsd_routed_builder`] with [`declare_gsd_contexts`], under the short name `gsd`.
pub fn gsd_context_builder() -> ContributionsBuilder {
    let mut c = gsd_routed_builder();
    declare_gsd_contexts(&mut c);
    c
}

/// `@gsd/core` with its context routes, served in process.
pub fn gsd_context_runtime() -> InProcessRuntime {
    gsd_routed_serving(gsd_context_builder)
}
