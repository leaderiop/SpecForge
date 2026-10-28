# Gating a workflow

An extension can teach an agent a workflow, and make SpecForge refuse a step the project is not ready for (ADR 0291).
Two things carry it: a **gate** on a command or prompt, and a **prompt** that lists the steps.

## A gate

A gate is a list of the extension's own diagnostic codes and the entity they are about. A gated command is refused
before its handler runs while any of those codes is an error in that entity's scope:

```rust
c.command("execute_phase", |cmd| {
    cmd.arg("phase", |a| a.required())
        .gate("E902")            // a blocked plan
        .gate("W904")            // a stale plan, once the profile raises it to an error
        .subject("phase")        // the arg whose entity scopes the gates
        .handler(|call| { /* ... */ });
});
```

The scope is the **subject** and what it scopes (a phase and its plans): an error placed on another phase does not
refuse this one. With no subject the scope is the project. The host resolves the subject like every other entity
argument (a short form is accepted) and sends the export its full path.

Where the diagnostic is placed decides the scope, so a pass reports a plan's problem on the plan (the diagnostic's
`entity`, or a span inside the plan's block). A diagnostic about neither counts for the whole project.

A refusal is `gate_failed`, kind `precondition_failed`. On the command line it is `error[gate_failed]` with each failing
diagnostic and exit 1; over MCP it is the tool's error with the same data:

```json
{"gates": [{"code": "t/E902", "off": false, "passes": false}],
 "failing": [{"code": "t/E902", "message": "the plan is blocked", "...": "..."}]}
```

A code that a setting gates off (`c.setting("workflow.gates", |s| { s.bool().gates(&["E901", "E902"]); })`) is
`off: true` and never fails the gate. A refusal calls no export and writes nothing.

### A check that only a gate asks

A pass in `phase: "guard"` reports codes at a gate as well as on a write. Declare the codes it answers with
`p.reports(&["E903"])`; the pass reads `PassInput.trigger`, `{kind: "write"}` (absent) or
`{kind: "gate", command, subject}`, and may stay silent on a write. A gate runs only the guard passes whose `reports`
meet its codes.

## A prompt

A prompt is the workflow: typed arguments, an entry gate, the steps and where the text came from.

```rust
c.prompt("plan_phase", |p| {
    p.title("Plan a phase").description("Plan the phase, then check the plans")
        .arg("phase", |a| { a.reference("phase").required(); })
        .gate("E901").subject("phase")
        .step("plan", |s| { s.title("Write the plans").command("record_plan"); })
        .step("execute", |s| { s.title("Execute the plans").command("execute_phase"); })
        .step("check", |s| { s.title("Check the plans"); })
        .attribution(Attribution { upstream: "gsd-core".into(), path: "workflows/plan-phase.md".into(),
            version: "1.18.0".into(), license: "MIT".into(), copyright: "Copyright (c) 2026 Open GSD".into() })
        .handler(|call| Ok(PromptOutput::user("...the workflow text...")));
});
```

Render it with `specforge prompt t.plan_phase 03`, `specforge t plan-phase 03` (when the extension has no command of
that name), MCP `prompts/get specforge.t.plan_phase` or the `specforge.prompt` tool. Your messages come first. The
host adds one message after them: the prompt as `@ext@version:id`, a digest of your messages, the attribution, the
subject's full path and each step with the tool and command line that run it and the state of **its** gate right now,
so the agent sees that `execute` is not allowed yet before it tries.

A prompt is not a contract. An agent may skip a step or run them out of order; only a command's gate refuses, and
only the host's edit pipeline refuses a bad write. Put the rule in a gate, and use the prompt to say what to do.

Attribution is rendered with the prompt, so ported text keeps its upstream, path, version, licence and copyright.

`p.when("workflow.research")` serves the prompt only while that bool setting is not false. A prompt whose name a command
of the extension already has is E039 and is not served; a prompt or gate the host cannot serve as declared is W021.
