# Writing a hook

A hook is a function of your extension that a host calls on one of its events: before a tool runs, after it ran, before
a commit, when a session starts, and so on. `specforge install <host>` wires it
([installing into a host](installing-into-a-host.md)); the host then runs `specforge hook <short>.<id>` on the event,
which loads your extension and calls the hook's export, `hook__<id>`.

```rust
c.hook("write_guard", HookEvent::PreTool, |h| {
    h.description("Refuses a write that shrinks a spec file a lot")
        .tools(&["write"])
        .paths(&["{spec_root}/**/*.spec"])
        .timeout_ms(2_000)
        .on_failure_deny()
        .handler(|call| {
            let facts = call.facts();
            let shrinks = facts.paths.iter().any(|p| {
                p.in_spec_root && p.exists && p.new_bytes.zip(p.bytes).is_some_and(|(new, old)| new * 2 < old)
            });
            Ok(if shrinks {
                HookOutput::deny("this write halves the file; edit it instead")
            } else {
                HookOutput::allow()
            })
        });
});
```

## Events

`pre_tool`, `post_tool` and `pre_commit` fire for a tool and take `tools` (host-neutral classes; empty means every tool)
and `paths` (project-relative globs a path of the tool's input must match; `{spec_root}` is the spec root). The others
are `user_prompt`, `session_start`, `stop`, `subagent_start`, `subagent_stop`, `pre_compact` and `statusline`. A host
that has no such event reports it as `not_rendered` at install. `pre_commit` fires only for a shell tool running
`git commit`, with the commit's message (`-m`, `--message`; a message read from a file with `-F` is not read).

A hook that does not match its event, tool class or paths costs a process start and nothing else: the extension is not
loaded. A hook whose `when` setting is false is a no-op.

## Facts

A sandboxed guest cannot read files or run commands, so the host computes what a hook usually needs:

- `paths`: for each path the tool names, `in_project`, `in_spec_root`, `exists`, `bytes` and, for a write, `new_bytes`;
- `shell`: a shell tool's command as words (POSIX quoting; a command the host cannot split gives none);
- `commit`: the message and whether it amends;
- `prompt`, `response`, `status`: what the host sent for those events;
- `graph` (`reads_graph`), `revisions` (`writes`) and `state` (`reads_state`), only when declared.

## Answers

A hook answers a `HookOutput`: a `decision` (`allow`, `deny`, `ask`) with a `reason`, `context` for the agent, a
`notice` for the user, the `statusline` text, an edit `plan` (declare `writes`), and a `state` patch. The host's adapter
maps it to the host's own protocol; a host that cannot express an answer (Cursor has no `ask`) takes the strictest
reading and says so.

- **Timeouts.** `timeout_ms` defaults to 5 s and may not exceed the sandbox ceiling. The host is told to wait five
  seconds more, so it never kills the process before your failure policy applies.
- **Failure.** A trap, a timeout or an answer that is no `HookOutput` applies `on_failure`: `allow` (the default) is
  the host's no-op answer with the E028 line on stderr; `deny` denies with the E028 text. A guard should say `deny`;
  everything else should not stop an agent.
- **Plans.** A hook that declares `writes` may answer an edit plan. It is checked and applied exactly as a command's
  plan is, with the hook as its origin, so a computed value, a harness value you do not own and a person's own value are
  refused. A refused plan becomes a deny on a pre event and a notice otherwise, naming the refusal.

## State

A hook that declares `reads_state` is sent `state.project` and, when the host names a session, `state.session`: small
JSON objects the host keeps under `.specforge/state/<extension>/` (git-ignored). Answer `state` as merge patches
(RFC 7396; `null` removes a key) and the host applies them after the plan, under the write lock. Each document holds at
most 64 KiB; a patch over it is refused whole and reported as a notice, and nothing changes. Sessions untouched for 30
days are removed. Keep in state what is about a session or a moment (a cursor, a context estimate), never project
truth: no rule reads it, a compile never depends on it and `check` never sees it.
