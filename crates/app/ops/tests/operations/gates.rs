//! A command's gate (ADR 0291): local codes of its extension, judged in its subject's scope before its export runs.
//! The fixture is `@t/core` ([`specforge_project::testing::GsdPrompts`]); its commands print the args they got.
//!
//! [`run_command`] is the one place a test calls the host: the tickets that change how a command is gated change it
//! (and [`refusal`], which reads a refusal).

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::extension::command::{CommandFormat, ExtensionCommands, RunError, run};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GsdPrompts, TestProject, gsd_prompts_sources};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_protocol_types::CommandOutput;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

use crate::writes::{changed_since, files_under};

/// A project of the fixture's extension: the sources for `goal` and `state`, and `settings` for `@t/core`.
pub fn project(goal: &str, state: &str, settings: Value) -> TestProject {
    let project = TestProject::new()
        .config(|c| {
            c["name"] = "p".into();
            c["settings"] = json!({ CORE: settings });
        })
        .enabling(&[CORE])
        .installing_enabled();
    for (path, text) in gsd_prompts_sources(goal, state) {
        project.write(path, text);
    }
    project
}

/// A project served in process and compiled.
pub struct Served {
    pub project: TestProject,
    pub runtime: Arc<InProcessRuntime>,
    pub compiled: CompiledProject,
}

pub fn served(project: TestProject) -> Served {
    served_with(project, GsdPrompts::new())
}

/// [`served`], the fixture answering as `fixture` says.
pub fn served_with(project: TestProject, fixture: GsdPrompts) -> Served {
    let runtime = Arc::new(fixture.runtime());
    let compiled = project.compile_in(Arc::clone(&runtime) as SharedRuntime);
    Served {
        project,
        runtime,
        compiled,
    }
}

impl Served {
    /// Whether the export `export` was called.
    pub fn called(&self, export: &str) -> bool {
        self.runtime
            .calls()
            .iter()
            .any(|call| call.export == export)
    }

    /// The input the last call of `export` was sent.
    pub fn input(&self, export: &str) -> Value {
        self.runtime
            .calls()
            .into_iter()
            .rev()
            .find(|call| call.export == export)
            .unwrap_or_else(|| panic!("{export} was not called"))
            .input
    }
}

/// Why a call was refused: the code an operation reports and its data.
#[derive(Debug)]
pub struct Refused {
    pub code: String,
    pub data: Value,
}

/// A refusal as a test reads it.
pub fn refusal(error: RunError) -> Refused {
    match error {
        RunError::Args(error) => Refused {
            code: "invalid_input".into(),
            data: json!({"message": error.to_string()}),
        },
        RunError::NoProject(error) => Refused {
            code: error.code.to_string(),
            data: error.data.map_or(Value::Null, |data| *data),
        },
        RunError::Call(error) => Refused {
            code: error.diagnostic().code,
            data: Value::Null,
        },
        RunError::Gate(error) => Refused {
            code: error.code.to_string(),
            data: error
                .data
                .map_or_else(|| json!({"message": error.message}), |data| *data),
        },
    }
}

/// Run command `id` of the fixture over `served`, with `args`.
pub fn run_command(served: &Served, id: &str, args: Value) -> Result<CommandOutput, Refused> {
    let view = ProjectView::of(&served.compiled);
    let commands = ExtensionCommands::build(view.registries());
    let command = commands
        .of("t")
        .find(|c| c.id() == id)
        .unwrap_or_else(|| panic!("no command t {id}"));
    run(
        &view,
        command,
        args.as_object().unwrap(),
        CommandFormat::Json,
    )
    .map_err(refusal)
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a command whose gate fails is refused before its export runs"
)]
fn a_command_whose_gate_fails_is_refused_before_its_export_runs() {
    let blocked = served(project("fine", "blocked", json!({})));
    let refused = run_command(&blocked, "execute_phase", json!({"phase": "03"}))
        .expect_err("a blocked plan in the phase refuses the step");
    assert_eq!(refused.code, "gate_failed");
    assert_eq!(refused.data["failing"][0]["code"], "t/E902", "{refused:?}");
    assert!(
        !blocked.called("cmd__execute_phase"),
        "the export is never called"
    );

    // The same step over a phase whose plan is not blocked runs: the refusal is the gate's.
    let fine = served(project("fine", "ok", json!({})));
    run_command(&fine, "execute_phase", json!({"phase": "03"})).expect("an unblocked phase runs");
    assert!(fine.called("cmd__execute_phase"));
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a gate is scoped to its subject"
)]
fn a_gate_is_scoped_to_its_subject() {
    let project = project("fine", "ok", json!({}));
    project.write(
        "m.spec",
        "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\nphase 05 \"Five\" {\n  milestone v1.0\n}\n",
    );
    project.write(
        "05-01.spec",
        "plan 05-01 \"Fifth\" {\n  phase 05\n  state \"blocked\"\n}\n",
    );
    let served = served(project);

    run_command(&served, "execute_phase", json!({"phase": "03"}))
        .expect("a blocked plan under phase 05 does not refuse phase 03");
    let refused = run_command(&served, "execute_phase", json!({"phase": "05"}))
        .expect_err("it refuses phase 05");
    assert_eq!(refused.code, "gate_failed");
    let failing = refused.data["failing"].as_array().unwrap();
    assert_eq!(failing.len(), 1, "{refused:?}");
    assert_eq!(failing[0]["data"]["entity"], "v1.0/05/05-01");
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a gate runs the guard passes it names"
)]
fn a_gate_runs_the_guard_passes_it_names() {
    let broken = served(project("fine", "broken", json!({})));
    assert!(
        broken
            .compiled
            .diagnostics()
            .iter()
            .all(|d| !d.code.ends_with("E903")),
        "a compile runs no guard pass"
    );
    let before = files_under(broken.project.root());

    let refused = run_command(&broken, "ship", json!({})).expect_err("the guard objects at ship");
    assert_eq!(refused.code, "gate_failed");
    assert_eq!(refused.data["failing"][0]["code"], "t/E903", "{refused:?}");
    let trigger = broken.input("__pass_ship_guard")["trigger"].clone();
    assert_eq!(trigger["kind"], "gate", "the pass is told why it runs");
    assert_eq!(trigger["command"], "ship");
    assert!(!broken.called("cmd__ship"));
    assert!(
        changed_since(broken.project.root(), &before).is_empty(),
        "a gate writes nothing"
    );

    // The same project with no broken window ships.
    let sound = served(project("fine", "ok", json!({})));
    run_command(&sound, "ship", json!({})).expect("nothing is broken");
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a gate on a code its setting turns off passes"
)]
fn a_gate_on_a_code_its_setting_turns_off_passes() {
    let args = json!({"phase": "03"});
    let on = served(project("fine", "blocked", json!({"workflow.gates": true})));
    let refused = run_command(&on, "execute_phase", args.clone()).expect_err("the gate is on");
    assert_eq!(refused.code, "gate_failed");

    let off = served(project("fine", "blocked", json!({"workflow.gates": false})));
    run_command(&off, "execute_phase", args).expect("a gated code that is off never fails a gate");
    assert!(off.called("cmd__execute_phase"));
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a warning a preset raises to an error fails a gate"
)]
fn a_raised_warning_fails_a_gate() {
    let args = json!({"phase": "03"});
    let compat = served(project("fine", "stale", json!({})));
    run_command(&compat, "execute_phase", args.clone()).expect("a stale plan is a warning");

    let strict = served(project("fine", "stale", json!({"profile": "strict"})));
    let refused = run_command(&strict, "execute_phase", args).expect_err("strict raises it");
    assert_eq!(refused.code, "gate_failed");
    assert_eq!(refused.data["failing"][0]["code"], "t/W904", "{refused:?}");
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a subject given as a short form is sent as its full path"
)]
fn a_subject_takes_a_short_form_and_is_sent_as_its_full_path() {
    let served = served(project("fine", "ok", json!({})));
    run_command(&served, "execute_phase", json!({"phase": "03"})).unwrap();
    assert_eq!(
        served.input("cmd__execute_phase")["args"]["phase"],
        "v1.0/03"
    );
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a subject that names no entity or several is refused before the gate runs"
)]
fn a_subject_that_names_no_entity_or_several_is_refused_before_the_gate() {
    let project = project("fine", "blocked", json!({}));
    project.write(
        "m.spec",
        "milestone v1.0 \"One\" {}\n\nmilestone v2.0 \"Two\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\nphase 03 \"Again\" {\n  milestone v2.0\n}\n",
    );
    let served = served(project);

    let several = run_command(&served, "execute_phase", json!({"phase": "03"}))
        .expect_err("`03` names a phase of each milestone");
    assert_eq!(several.code, "E003");
    let message = several.data["message"].as_str().unwrap();
    assert!(
        message.contains("v1.0/03") && message.contains("v2.0/03"),
        "the candidates are named: {message}"
    );
    let none = run_command(&served, "execute_phase", json!({"phase": "99"}))
        .expect_err("there is no phase 99");
    assert_eq!(none.code, "E003");
    assert!(!served.called("cmd__execute_phase"), "no export ran");
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a guard pass that fails at a gate fails the gate, and the export is not called"
)]
fn a_guard_pass_that_cannot_run_fails_its_gate() {
    use specforge_wasm::{WasmCallResult, WasmTrapInfo};
    let trap = WasmCallResult::Trap(WasmTrapInfo {
        kind: "fuel".into(),
        message: "out of fuel".into(),
        export_name: "__pass_ship_guard".into(),
    });
    let broken = served_with(
        project("fine", "ok", json!({})),
        GsdPrompts::new().answering(CORE, "__pass_ship_guard", trap),
    );
    let refused =
        run_command(&broken, "ship", json!({})).expect_err("an unasked gate is not satisfied");
    assert_eq!(refused.code, "gate_failed");
    assert_eq!(refused.data["failing"][0]["code"], "t/E903", "{refused:?}");
    assert!(!broken.called("cmd__ship"));
}

#[specforge_test(
    behavior = "gate_a_command",
    verify = "a finding that names no entity is in a gate's scope by the block that holds its span"
)]
fn a_finding_with_only_a_span_is_scoped_by_its_block() {
    use specforge_wasm::WasmCallResult;
    let at = |line: u32| {
        let finding = json!({"diagnostics": [{"code": "E902", "severity": "Error", "message": "blocked",
            "span": {"file": "m.spec", "start_line": line, "start_col": 1, "end_line": line, "end_col": 2}}],
            "summary": {}, "values": []});
        GsdPrompts::new().answering(
            CORE,
            "__pass_audit",
            WasmCallResult::Ok(finding.to_string().into_bytes()),
        )
    };
    let layout = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\nphase 04 \"Four\" {\n  milestone v1.0\n}\n";
    let build = |line: u32| {
        let project = project("fine", "ok", json!({}));
        project.write("m.spec", layout);
        served_with(project, at(line))
    };
    let args = json!({"phase": "03"});
    let inside = run_command(&build(4), "execute_phase", args.clone())
        .expect_err("line 4 is phase 03's block");
    assert_eq!(inside.code, "gate_failed");
    run_command(&build(8), "execute_phase", args).expect("line 8 is phase 04's block");
}
