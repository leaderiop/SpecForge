//! The `process` port: one program a capability declares, through the call's effects.

use std::sync::Arc;

use specforge_effects::process::{
    DEADLINE_CEILING, OUTPUT_CAP, Output, ProcessClass, same_variable,
};
use specforge_effects::{Denial as Refusal, Denied, EffectRequest, OutcomeClass, Served};
use specforge_protocol_types::{EnvVar, HostError, PortScope, ProcessOutput, ProcessRequest};

use super::{
    PortSession, answered, asked_cap, failed, held, lent_record, millis, not_in_journal,
    not_performed, now, out_of_scope,
};

impl PortSession {
    /// One program, its argv as a capability declares it, in the project's directory, through the call's effects.
    pub fn process_run(&mut self, request: ProcessRequest) -> Result<ProcessOutput, HostError> {
        let (kind, call) = self.lent()?;
        let asked = EffectRequest::Process(request.clone());
        let granted = Self::granted(kind, call, &asked)?;
        let PortScope::Process(scope) = &granted.declared.port else {
            return Err(out_of_scope("no process capability permits it"));
        };
        // A name it does not declare is one nobody approved: `BASH_ENV` chooses what an approved command runs.
        if let Some(var) = request
            .env
            .iter()
            .find(|var| !scope.env.iter().any(|name| same_variable(name, &var.name)))
        {
            let refusal = Refusal {
                extension: call.grant.extension().to_string(),
                request: specforge_effects::grant::one_line(&asked),
                scope: Some(granted.declared.name.clone()),
                why: Denied::Undeclared,
                detail: format!("its capability does not declare the variable {}", var.name),
            };
            return Err(Self::refused(call, refusal));
        }
        let mut env = request.env;
        // Removed after what the guest asked: a program cannot echo a secret into its answer.
        env.extend(call.secrets.iter().map(EnvVar::removed));
        // What the capability declares is the most it runs for, under the port's ceiling.
        let most = scope
            .deadline_ms
            .map_or(millis(DEADLINE_CEILING), |declared| {
                u64::from(declared).min(millis(DEADLINE_CEILING))
            });
        let request = ProcessRequest {
            env,
            output_cap: asked_cap(request.output_cap, self.answer_cap.min(OUTPUT_CAP)),
            deadline_ms: asked_cap(request.deadline_ms, most.min(self.budget_left())),
            inherit: Some(scope.env.clone()),
            ..request
        };
        let asked = EffectRequest::Process(request.clone());
        let lent = lent_record(call, granted);
        let mut live = None;
        let served = call.effects.serve_lent(lent, &asked, now(), |ports| {
            let granted = call.grant.process(kind, Arc::clone(&ports.process));
            let outcome = granted.run(&request, call.roots.project(), Output::Buffer);
            let answer = answered(outcome.class.outcome(), &outcome.stdout);
            live = Some(outcome);
            answer
        });
        let (class, stdout, stderr) = match (served, live) {
            (Served::Outcome { .. }, Some(outcome)) => {
                (outcome.class, outcome.stdout, outcome.stderr)
            }
            (Served::Outcome { answer, .. }, None) => {
                let class = match answer.outcome.class {
                    OutcomeClass::Ok => ProcessClass::Exited(0),
                    OutcomeClass::Exit(code) => ProcessClass::Exited(code),
                    OutcomeClass::Capped => ProcessClass::OutputCapped,
                    other => ProcessClass::CannotStart(other.to_string()),
                };
                (class, answer.payload.unwrap_or_default(), Vec::new())
            }
            (Served::Miss, _) => return Err(not_in_journal(&asked)),
            (_, _) => return Err(not_performed(&asked)),
        };
        let exit_code = match class {
            ProcessClass::Exited(code) => Some(code),
            ProcessClass::OutputCapped => None,
            other => {
                let program = request.argv.first().map_or("", String::as_str);
                return Err(failed(other.outcome(), format!("{program} {other}")));
            }
        };
        let (stdout, cut_out) = held(stdout, self.answer_cap);
        let (stderr, cut_err) = held(stderr, self.answer_cap - stdout.len() as u64);
        Ok(ProcessOutput {
            exit_code,
            stdout,
            stderr,
            truncated: cut_out || cut_err || exit_code.is_none(),
        })
    }
}
