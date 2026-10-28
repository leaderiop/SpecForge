//! `specforge hook` (ADR 0292): one hook run for a host event. The event is filtered, the facts a sandboxed guest
//! cannot read are computed, the hook's export is called within its time limit, a failure is answered by the hook's
//! policy, a plan it answers goes through the edit-plan step as `Origin::Hook`, and the answer is neutral: the
//! adapter writes it as the host expects.

mod facts;
mod filter;

use specforge_protocol_types::{Decision, HookInput, HookOutput, HookTool, OnFailure, RawGraph};
use specforge_wasm::ExtensionCalls;

pub use facts::{commit, path_facts, path_names, relative_to, shell_words};
pub use filter::Filter;

use crate::Caller;
use crate::authoring::apply::{Origin, apply_plan};
use crate::host::render::{on, spec_root_spelled};
use crate::host::{HookAnswer, HostHookEvent};
use crate::view::ProjectView;

/// What running a hook came to.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub answer: HookAnswer,
    /// What the hook has to say to the one running it: a failure the policy let through, an unknown hook.
    pub stderr: String,
    /// Whether the hook's export was called.
    pub called: bool,
    /// The sources a plan the hook answered changed, from the project root.
    pub changed: Vec<String>,
}

/// The spec root, relative to the project root with `/` (empty: the root itself).
fn spec_root(view: &ProjectView<'_>) -> String {
    let rel =
        specforge_common::ProjectPath::of(view.env().root(), view.env().spec_root()).into_string();
    let rel = rel.trim_matches('/').trim_start_matches("./");
    if rel == "." {
        String::new()
    } else {
        rel.to_string()
    }
}

/// Run the hook routed `route` (`<short>.<id>`) for `event` over the project `view` reads.
pub fn run(
    view: &ProjectView<'_>,
    route: &str,
    event: &HostHookEvent,
    caller: Caller<'_>,
) -> Outcome {
    let build = view.registries();
    let Some(hook) = build.hook(route) else {
        return Outcome {
            stderr: format!(
                "unknown_hook: no hook '{route}' is declared by the project's extensions; run install again\n"
            ),
            ..Outcome::default()
        };
    };
    let (extension, declared) = (hook.extension, hook.item);
    let root = view.env().root();
    let cwd = event
        .cwd
        .as_deref()
        .map_or_else(|| root.to_path_buf(), Into::into);
    if !on(view, extension, declared.when.as_ref()) {
        return Outcome::default();
    }
    let filter = Filter {
        event: declared.event,
        tools: declared.tools.clone(),
        paths: declared
            .paths
            .iter()
            .map(|p| spec_root_spelled(view, p))
            .collect(),
    };
    if !filter.matches(event, root, &cwd) {
        return Outcome::default();
    }

    let spec = spec_root(view);
    let tool = event.tool.as_ref();
    let input = HookInput::<RawGraph> {
        event: declared.event,
        host: event.host.clone(),
        cwd: cwd.display().to_string(),
        today: crate::extension::command::today(),
        session: event.session.clone(),
        tool: tool.map(|t| HookTool {
            class: t.class.clone(),
            name: t.name.clone(),
            input: t.input.clone(),
        }),
        paths: tool
            .map(|t| facts::path_facts(root, &cwd, &spec, &t.input))
            .unwrap_or_default(),
        shell: tool
            .filter(|t| t.class == "shell")
            .and_then(|t| facts::shell_words(&t.input)),
        commit: (declared.event == specforge_protocol_types::HookEvent::PreCommit)
            .then(|| tool.and_then(|t| facts::commit(&t.input)))
            .flatten(),
        prompt: event.prompt.clone(),
        response: event.response.clone(),
        status: event.status.clone(),
        graph: declared.reads_graph.then(|| {
            RawGraph::new(crate::extension::command::graph_export(view))
                .expect("the graph export is one JSON value")
        }),
        revisions: if declared.writes {
            crate::revision::of_project(view.project())
        } else {
            Default::default()
        },
        state: None,
    };
    let called = view.runtime().map(|runtime| {
        ExtensionCalls::new(runtime.as_ref()).run_hook(
            extension,
            &declared.export,
            &input,
            declared.timeout_ms,
        )
    });
    let output = match called {
        Some(Ok(output)) => output,
        Some(Err(error)) => {
            return failed(
                declared.on_failure,
                &crate::host::render::call_failed(&error).message,
                route,
            );
        }
        None => {
            return failed(
                declared.on_failure,
                "E028: the project's extensions are not loaded",
                route,
            );
        }
    };
    let mut outcome = Outcome {
        called: true,
        ..Outcome::default()
    };
    apply(
        view,
        route,
        extension,
        declared.event,
        declared.writes,
        output,
        caller,
        &mut outcome,
    );
    outcome
}

/// The policy of a hook that did not answer.
fn failed(policy: OnFailure, message: &str, route: &str) -> Outcome {
    let text = if message.starts_with("E028") {
        message.to_string()
    } else {
        format!("E028: {message}")
    };
    let mut outcome = Outcome {
        called: true,
        stderr: format!("hook {route}: {text}\n"),
        ..Outcome::default()
    };
    if policy == OnFailure::Deny {
        outcome.answer.decision = Some(Decision::Deny);
        outcome.answer.reason = Some(format!("hook {route} failed: {text}"));
    }
    outcome
}

/// `output` as the neutral answer, its plan applied first.
#[allow(
    clippy::too_many_arguments,
    reason = "one call site; a struct would only rename them"
)]
fn apply(
    view: &ProjectView<'_>,
    route: &str,
    extension: &str,
    event: specforge_protocol_types::HookEvent,
    writes: bool,
    output: HookOutput,
    caller: Caller<'_>,
    outcome: &mut Outcome,
) {
    let answer = &mut outcome.answer;
    answer.decision = output.decision;
    answer.reason = output.reason;
    answer.context = output.context;
    answer.notice = output.notice;
    answer.statusline = output.statusline;
    let Some(plan) = output.plan else {
        return;
    };
    if !writes {
        outcome.stderr.push_str(&format!(
            "hook {route} answered a plan but declares no writes: ignored\n"
        ));
        return;
    }
    let origin = Origin::Hook {
        extension: extension.to_string(),
    };
    match apply_plan(view, &plan, &origin, false, caller) {
        Ok(edited) => {
            // The sources the plan changed; the ledger line a harness value adds is the host's, not the plan's.
            outcome.changed = edited
                .report
                .affected_files
                .into_iter()
                .filter(|f| f.ends_with(".spec"))
                .collect();
        }
        Err(refusal) => {
            let text = format!(
                "hook {route}: its plan was refused: {}",
                refusal.error.message
            );
            if matches!(
                event,
                specforge_protocol_types::HookEvent::PreTool
                    | specforge_protocol_types::HookEvent::PreCommit
            ) {
                answer.decision = Some(Decision::Deny);
                answer.reason = Some(text);
            } else {
                answer.notice = Some(text);
            }
        }
    }
}
