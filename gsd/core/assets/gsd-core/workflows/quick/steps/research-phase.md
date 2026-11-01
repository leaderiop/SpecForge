<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/quick/steps/research-phase.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Step 4.75: Research phase (only when `$RESEARCH_MODE`)**

Skip this step entirely if NOT `$RESEARCH_MODE`.

Display banner:
```
### GSD ► RESEARCHING QUICK TASK

◆ Investigating approaches for: ${DESCRIPTION} (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)
```

First make plan `${quick_id}`, which the research names, unless the discussion made it (the prompt's "The quick plan"): `specforge query "${quick_id}" --depth 0 >/dev/null 2>&1 || specforge new plan "${quick_id}" "${DESCRIPTION}" --in spec/quick.spec --type execute --files_modified "" --objective "${DESCRIPTION}"`. Then spawn a single focused researcher (not 4 parallel researchers like full phases — quick tasks need targeted research, not broad domain surveys):

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`planner_model`, `checker_model`, `executor_model`, `reviewer_model`, `verifier_model`, `researcher_model`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the role as named (the installed agent carries its role). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

```
Agent(
  prompt="
<research_context>

**Mode:** quick-task
**Task:** ${DESCRIPTION}
**Output:** the `research` entity `${quick_id}-RESEARCH`, with `plan ${quick_id}` (that plan exists: it is made before this research, so never `specforge new plan`)

<required_reading>
- ${STATE_PATH} (Project state — what's already built)
- ${PROJECT_PATH} (Project context)
- ./CLAUDE.md or ./.claude/CLAUDE.md (if exists — project-specific guidelines)
${DISCUSS_MODE ? '- `specforge query ' + quick_id + '-CONTEXT --depth 2 --kinds context,decision --format context` (User decisions — research should align with these. #3894: when workflow.research_before_questions is enabled research runs BEFORE discussion, so this file will not exist yet — read it only if present)' : ''}
</required_reading>

${AGENT_SKILLS_RESEARCHER}

</research_context>

<focus>
This is a quick task, not a full phase. Research should be concise and targeted:
1. Best libraries/patterns for this specific task
2. Common pitfalls and how to avoid them
3. Integration points with existing codebase
4. Any constraints or gotchas worth knowing before planning

Do NOT produce a full domain survey. Target 1-2 pages of actionable findings.
</focus>

<output>
Write research as the `research` entity `${quick_id}-RESEARCH`, after its plan (made before this research): `specforge new research "${quick_id}-RESEARCH" "<title>" --in spec/quick.spec --plan "${quick_id}" --researched ${date} --confidence <high|medium|low> --summary "<summary>"`, then its items (`specforge new finding`, `specforge new pitfall`, … with `--research "${quick_id}-RESEARCH"`)
Use standard research format but keep it lean — skip sections that don't apply.
Return: ## RESEARCH COMPLETE with file path
</output>
",
  subagent_type="gsd-phase-researcher",
  model="{researcher_model}",
  description="Research: ${DESCRIPTION}"
)
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

After researcher returns:
1. Verify the research exists: `specforge query "${quick_id}-RESEARCH" --depth 0 >/dev/null`
2. Report: "Research complete: research ${quick_id}-RESEARCH"

If research file not found, warn but continue: "Research agent did not produce output — proceeding to planning without research."
