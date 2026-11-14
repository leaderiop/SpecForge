<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/discuss-phase-assumptions.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>
Extract implementation decisions that downstream agents need — using codebase-first analysis
and assumption surfacing instead of interview-style questioning.

You are a thinking partner, not an interviewer. Analyze the codebase deeply, surface what you
believe based on evidence, and ask the user only to correct what's wrong.
</purpose>

<available_agent_types>
Valid GSD subagent types (use exact names — do not fall back to 'general-purpose'):
- gsd-assumptions-analyzer — Analyzes codebase to surface implementation assumptions
</available_agent_types>

<downstream_awareness>
**CONTEXT.md feeds into:**

1. **gsd-phase-researcher** — Reads CONTEXT.md to know WHAT to research
2. **gsd-planner** — Reads CONTEXT.md to know WHAT decisions are locked

**Your job:** Capture decisions clearly enough that downstream agents can act on them
without asking the user again. Output is identical to discuss mode — same CONTEXT.md format.
</downstream_awareness>

<philosophy>
**Assumptions mode philosophy:**

The user is a visionary, not a codebase archaeologist. They need enough context to evaluate
whether your assumptions match their intent — not to answer questions you could figure out
by reading the code.

- Read the codebase FIRST, form opinions SECOND, ask ONLY about what's genuinely unclear
- Every assumption must cite evidence (file paths, patterns found)
- Every assumption must state consequences if wrong
- Minimize user interactions: ~2-4 corrections vs ~15-20 questions
</philosophy>

<scope_guardrail>
**CRITICAL: No scope creep.**

The phase boundary comes from ROADMAP.md and is FIXED. Discussion clarifies HOW to implement
what's scoped, never WHETHER to add new capabilities.

When user suggests scope creep:
"[Feature X] would be a new capability — that's its own phase.
Want me to note it for the roadmap backlog? For now, let's focus on [phase domain]."

Capture the idea in "Deferred Ideas". Don't lose it, don't act on it.
</scope_guardrail>

<answer_validation>
**IMPORTANT: Answer validation** — After every AskUserQuestion call, if the response is empty/whitespace-only:

- **"Other" with empty text** (the user wants to type freeform): output `"What would you like to discuss?"`, STOP generating, wait for the user's next message, then reflect it back and continue. Do NOT retry AskUserQuestion or call any tools.
- **Any other empty response:** retry once with the same parameters; if still empty, present options as a plain-text numbered list. Never proceed with empty input.

**Text mode** (`--text` or `workflow.text_mode: true`): follow `workflows/discuss-phase/modes/text.md` — do not use AskUserQuestion at all.
</answer_validation>

<process>

<step name="initialize" priority="first">
Phase number from argument (required).

```bash

# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)

INIT=$(specforge query "$PHASE_ID" --depth 1 --format json)

AGENT_SKILLS_ANALYZER=""   # the host install carries the skills of gsd-assumptions-analyzer
# #2072: resolve the routed model so model_overrides / models.discuss are honored
# (the resolver maps gsd-assumptions-analyzer → phaseType "discuss"); thread it below.
# the installed gsd-assumptions-analyzer carries its model: the Agent call needs no model parameter
```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

Read from the answer: the node whose `id` is `$PHASE_ID` is the phase (its number, two digits, is the last segment of that id; `name` and `slug` are its fields; a phase not in the graph is an E003 error),
and its `plan`, `research`, `context` and `verification` entities, when it has them, are among the other nodes (`plan_count` counts the plans). `commit_docs` is a setting:
`specforge settings --format json --extension @gsd/core` (the phase's spec is read in `check_spec`, from the graph).

**If `phase_found` is false:**
```
Phase [X] not found in roadmap.

Use /gsd:progress to see available phases.
```
Exit workflow.

**If `phase_found` is true:** Continue to check_spec.

**Auto mode** — If `--auto` is present in ARGUMENTS:
- In `check_existing`: auto-select "Update it" (if context exists) or continue without prompting
- In `present_assumptions`: no confirmation gate; go straight on to `write_context`
- In `correct_assumptions`: auto-select recommended option for each correction
- Log each auto-selected choice inline
- After completion, auto-advance to plan-phase
</step>

<step name="check_spec">
Check whether the phase has a spec (from the spec-phase workflow): it locks requirements before implementation decisions. The phase spec is the phase's `phase_spec` entity in the graph (an AI spec or a UI spec is never one); read it, never glob for a file: `SPEC_ID=$(specforge query "$PHASE_ID" --depth 1 --pick '.nodes[kind=phase_spec].id' --first 1); HAS_SPEC=$([ -n "$SPEC_ID" ] && echo true || echo false)`. Below, `has_spec` is `$HAS_SPEC`, and the spec is the entity `$SPEC_ID`.

**If `has_spec` is true:**
1. Read the spec and its locked requirements: `specforge query "$SPEC_ID" --depth 1 --format context`.
2. Count its `locked_req` entities: `specforge query "$SPEC_ID" --depth 1 --pick '.nodes[kind=locked_req].id' --count`.
3. Display: `{N} requirements are locked by the phase spec; this discussion covers implementation decisions only.`
4. Set `spec_loaded = true`.
5. Hold the requirements, boundaries and acceptance criteria as `<locked_requirements>`: the context names the spec, and they are not asked again.

**If `has_spec` is false:** Continue with `spec_loaded = false`.

**Note:** `*-AI-SPEC.md` (from the ai-integration-phase workflow) and `*-UI-SPEC.md` (from the ui-phase workflow) are different artifacts and never count as the phase SPEC.
</step>

<step name="check_existing">
Check if CONTEXT.md already exists using `has_context` from init.

```bash
specforge query "$PHASE_ID" --depth 1 --kinds context --format brief
```

**If exists:**

**If `--auto`:** Auto-select "Update it". Log: `[auto] Context exists — updating with assumption-based analysis.`

**Otherwise:** Use AskUserQuestion:
- header: "Context"
- question: "Phase [X] already has context. What do you want to do?"
- options:
  - "Update it" — Re-analyze codebase and refresh assumptions
  - "View it" — Show me what's there
  - "Skip" — Use existing context as-is

If "Update": Load existing, continue to load_prior_context
If "View": show it with `specforge query "$PHASE_ID" --depth 1 --format context`, then offer update/skip
If "Skip": Exit workflow

**If doesn't exist:**

Check `has_plans` and `plan_count` from init. **If `has_plans` is true:**

**If `--auto`:** Auto-select "Continue and replan after". Log: `[auto] Plans exist — continuing with assumption analysis, will replan after.`

**Otherwise:** Use AskUserQuestion:
- header: "Plans exist"
- question: "Phase [X] already has {plan_count} plan(s) created without user context. Your decisions here won't affect existing plans unless you replan."
- options:
  - "Continue and replan after"
  - "View existing plans"
  - "Cancel"

If "Continue and replan after": Continue to load_prior_context.
If "View existing plans": Display plan files, then offer "Continue" / "Cancel".
If "Cancel": Exit workflow.

**If `has_plans` is false:** Continue to load_prior_context.
</step>

<step name="load_prior_context">
Read project-level and prior phase context to avoid re-asking decided questions.

**Step 1: Read project-level files**
```bash
specforge query "$PROJECT" --depth 0 --format context 2>/dev/null || true
specforge query "$MILESTONE" --depth 2 --kinds requirement --format context 2>/dev/null || true
specforge query "$MILESTONE" --kinds milestone,phase --format context 2>/dev/null || true
```

Extract from these:
- **PROJECT.md** — Vision, principles, non-negotiables, user preferences
- **REQUIREMENTS.md** — Acceptance criteria, constraints
- **STATE.md** — Current progress, any flags

**Step 2: Read all prior CONTEXT.md files**
```bash
specforge query "$MILESTONE" --depth 2 --kinds context --format brief
```

For each CONTEXT.md where phase number < current phase:
- Read the `<decisions>` section — these are locked preferences
- Read `<specifics>` — particular references or "I want it like X" moments
- Note patterns (e.g., "user consistently prefers minimal UI")

**Step 3: Build internal `<prior_decisions>` context**

Structure the extracted information for use in assumption generation.

**If no prior context exists:** Continue without — expected for early phases.
</step>

<step name="cross_reference_todos">
Check if any pending todos are relevant to this phase's scope.

```bash
TODO_MATCHES=""   # todos are not part of @gsd/core: fill this only if the project keeps some
```

Parse JSON for: `todo_count`, `matches[]`.

**If `todo_count` is 0:** Skip silently.

**If matches found:** Present matched todos, use AskUserQuestion (multiSelect) to fold relevant ones into scope.

**For selected (folded) todos:** carry them along as `<folded_todos>` until the context of the phase is recorded, where each turns into one `decision`.
**For unselected:** carry them as `<reviewed_todos>` until then; the phase records each one as a `deferred_idea`, to be taken up later.

**Auto mode (`--auto`):** Fold all todos with score >= 0.4 automatically. Log the selection.
</step>

<step name="load_methodology">
Read the project-level methodology file if it exists. This must happen before assumption analysis
so that active lenses shape how assumptions are generated and evaluated.

```bash
specforge export --kinds methodology --format context 2>/dev/null || true
```

**If METHODOLOGY.md exists:**
- Parse each named lens: its diagnoses, recommendations, and triggering conditions
- Store as internal `<active_lenses>` for use in deep_codebase_analysis and present_assumptions
- When spawning the gsd-assumptions-analyzer, pass the lens list so it can flag which lenses apply
- When presenting assumptions, end with the lenses applied, under a "Methodology" heading
  and what they flagged (if anything)

**If METHODOLOGY.md does not exist:** Skip silently. This artifact is optional.
</step>

<step name="scout_codebase">
Lightweight scan of existing code to inform assumption generation.

**Step 1: Check for existing codebase maps**
```bash
# the codebase map is not part of @gsd/core: look for an existing map only if the project keeps one
```

**If codebase maps exist:** Read relevant ones (CONVENTIONS.md, STRUCTURE.md, STACK.md). Extract reusable components, patterns, integration points. Skip to Step 3.

**Step 2: If no codebase maps, do targeted grep**

Extract key terms from phase goal, search for related files.

```bash
specforge files search -l '{term1}|{term2}' src app --glob '*.ts' --glob '*.tsx' --max 10 --pick '.files[]'
```

Read the 3-5 most relevant files.

**Step 3: Build internal `<codebase_context>`**

Identify reusable assets, established patterns, integration points, and creative options. Store internally for use in deep_codebase_analysis.
</step>

<step name="deep_codebase_analysis">
Spawn a `gsd-assumptions-analyzer` agent to deeply analyze the codebase for this phase. This
keeps raw file contents out of the main context window, protecting token budget.

**Resolve calibration tier (if USER-PROFILE.md exists):**

```bash
PROFILE_PATH="$HOME/.claude/gsd-core/USER-PROFILE.md"
```

If file exists at PROFILE_PATH:
- Priority 1: a project override of `preferences.vendor_philosophy` (not a `@gsd/core` setting: skip to priority 2)
- Priority 2: Read USER-PROFILE.md Vendor Choices/Philosophy rating (global)
- Priority 3: Default to "standard"

Map to calibration tier:
- conservative OR thorough-evaluator → full_maturity (more alternatives, detailed evidence)
- opinionated → minimal_decisive (fewer alternatives, decisive recommendations)
- pragmatic-fast OR any other value → standard

If no USER-PROFILE.md: calibration_tier = "standard"

**Spawn Explore subagent** (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)**:**

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the role name as it is installed. On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`ANALYZER_MODEL`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

```
Agent(subagent_type="gsd-assumptions-analyzer", model="{ANALYZER_MODEL}", prompt="""
Analyze the codebase for Phase {PHASE}: {phase_name} (PHASE_ID {PHASE_ID}).

<required_reading>
- specforge context gsd/assumptions-analyzer {PHASE_ID} (the analyzer pack: the phase, its requirements, the earlier phases' contexts and decisions)
</required_reading>

Phase goal: {roadmap_description}
Prior decisions: {prior_decisions_summary}
Codebase scout hints: {codebase_context_summary}
Calibration: {calibration_tier}
Settled by the phase spec (leave this line out when spec_loaded = false): {locked_requirements}

Your job:
1. Read the analyzer pack (`<required_reading>`)
2. Find related work: `specforge search` for the goal terms, then `specforge anchors` for the source places an entity names (`--file <path>`: which entity claims a file)
3. Glob/Grep for files related to: {phase_relevant_terms} that no entity is anchored to
4. Read 5-15 most relevant source files
5. Return structured assumptions
6. When locked requirements are given, treat them as decided: form assumptions about HOW to implement them only, never about WHAT to build or WHY

## Output Format

Return EXACTLY this structure:

## Assumptions

### [Area Name] (e.g., "Technical Approach")
- **Assumption:** [Decision statement]
  - **Why this way:** [Evidence from codebase — cite file paths]
  - **If wrong:** [Concrete consequence of this being wrong]
  - **Confidence:** Confident | Likely | Unclear

(3-5 areas, calibrated by tier:
- full_maturity: 3-5 areas, 2-3 alternatives per Likely/Unclear item
- standard: 3-4 areas, 2 alternatives per Likely/Unclear item
- minimal_decisive: 2-3 areas, decisive single recommendation per item)

## Needs External Research
[Topics where codebase alone is insufficient — library version compatibility,
ecosystem best practices, etc. Leave empty if codebase provides enough evidence.]

${AGENT_SKILLS_ANALYZER}
""")
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, analyze the codebase, or process assumptions while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

Parse the subagent's response. Extract:
- `assumptions[]` — each with area, statement, evidence, consequence, confidence
- `needs_research[]` — topics requiring external research (may be empty)

**Initialize canonical refs accumulator:**
- Source 1: Copy `Canonical refs:` from ROADMAP.md for this phase, expand to full paths
- Source 2: Check REQUIREMENTS.md and PROJECT.md for specs/ADRs referenced
- Source 3: Add any docs referenced in codebase scout results
</step>

<step name="external_research">
**Skip if:** `needs_research` from deep_codebase_analysis is empty.

If research topics were flagged, spawn a general-purpose research agent (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze):

```
Agent(subagent_type="general-purpose", prompt="""
Research the following topics for Phase {PHASE}: {phase_name}.

Topics needing research:
{needs_research_content}

For each topic, return:
- **Finding:** [What you learned]
- **Source:** [URL or library docs reference]
- **Confidence impact:** [Which assumption this resolves and to what confidence level]

Use Context7 (resolve-library-id then query-docs) for library-specific questions.
Use WebSearch for ecosystem/best-practice questions.
""")

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not independently research any of these topics while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work and wasted context. Only resume when the subagent result is available.
```

Merge findings back into assumptions:
- Update confidence levels where research resolves ambiguity
- Add source attribution to affected assumptions
- Keep the research findings for the phase's discussion log

**If no gaps flagged:** Skip entirely. Most phases will skip this step.
</step>

<step name="present_assumptions">
Display all assumptions grouped by area with confidence badges.

**Format for display:**

```
## Phase {PHASE}: {phase_name} — Assumptions

Based on codebase analysis, here's what I'd go with:

### {Area Name}
{Confidence badge} **{Assumption statement}**
↳ Evidence: {file paths cited}
↳ If wrong: {consequence}

### {Area Name 2}
...

[If external research was done:]
### External Research Applied
- {Topic}: {Finding} (Source: {URL})
```

**If `--auto`:**
- If all assumptions are Confident or Likely: log assumptions, skip to write_context.
  Log: `[auto] All assumptions Confident/Likely — proceeding to context capture.`
- If any assumptions are Unclear: log a warning, auto-select recommended alternative for
  each Unclear item. Log: `[auto] {N} Unclear assumptions auto-resolved with recommended defaults.`
  Proceed to write_context.

**Otherwise:** Use AskUserQuestion:
- header: "Assumptions"
- question: "These all look right?"
- options:
  - "Yes, proceed" — Record the context with these assumptions as decisions
  - "Let me correct some" — Select which assumptions to change

**If "Yes, proceed":** Skip to write_context.
**If "Let me correct some":** Continue to correct_assumptions.
</step>

<step name="correct_assumptions">
The assumptions are already displayed above from present_assumptions.

Present a multiSelect where each option's label is the assumption statement and description
is the "If wrong" consequence:

Use AskUserQuestion (multiSelect):
- header: "Corrections"
- question: "Which assumptions need correcting?"
- options: [one per assumption, label = assumption statement, description = "If wrong: {consequence}"]

For each selected correction, ask ONE focused question:

Use AskUserQuestion:
- header: "{Area Name}"
- question: "What should we do instead for: {assumption statement}?"
- options: [2-3 concrete alternatives describing user-visible outcomes, recommended option first]

Record each correction:
- Original assumption
- User's chosen alternative
- Reason (if provided via "Other" free text)

After all corrections processed, continue to write_context with updated assumptions.

**Auto mode:** Should not reach this step (--auto skips from present_assumptions).
</step>

<step name="write_context">
Record the context: read the context template with `specforge gsd reference gsd-core/templates/context.md` and run
its fence, the same entities discuss mode records.

Map assumptions to the context's entities:
- Each assumption becomes a `decision` (D-01, D-02, …), its `area` the assumption's area
- A correction overrides the original assumption: the decision holds the user's chosen alternative
- Areas where all assumptions were Confident are decisions like any other
- When `spec_loaded = true`: one `canonical_refs` row names the spec entity with the note "Locked requirements — MUST
  read before planning", and its requirements are not copied into decisions (agents read the spec itself); the
  decisions hold only the HOW decisions from this discussion
- Folded todos are the todo capability's, not the context's
</step>

<step name="write_discussion_log">
Record the audit trail of assumptions and corrections: read the log template with `specforge gsd reference
workflows/discuss-phase/templates/discussion-log.md` and run its fence, every entry's `mode` `assumptions`:

- each assumption presented is an entry: its area, the assumption as `question` and `chosen`, its `confidence`
  (`confident`, `likely` or `unclear`) and its `evidence` (the file paths);
- each correction is an entry: the original assumption as `question`, the alternatives as `options`, the user's choice
  as `chosen`, and the user's rationale, if provided, as `notes`;
- with `--auto`, an Unclear item auto-resolved is an entry whose `notes` say which recommended option was auto-selected;
- external research findings go in the `notes` of the entry they informed, with their source URL.
</step>

<step name="git_commit">
Commit phase context and discussion log:

```bash
specforge git commit --add spec -m "docs(${padded_phase}): capture phase context (assumptions mode)" && specforge collect --runner gsd-git --yes
```

Confirm: "Committed: docs(${padded_phase}): capture phase context (assumptions mode)"
</step>

<step name="confirm_creation">
Present summary and next steps:

```
Created: the `context` entity of phase ${PADDED_PHASE}-${SLUG}

## Decisions Captured (Assumptions Mode)

### {Area Name}
- {Key decision} (from assumption / corrected)

{Repeat per area}

[If corrections were made:]
## Corrections Applied
- {Area}: {original} → {corrected}

[If deferred ideas exist:]
## Noted for Later
- {Deferred idea} — future phase

---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Phase ${PHASE}: {phase_name}** — {Goal from ROADMAP.md}

`/clear` then:

`/gsd:plan-phase ${PHASE}`

---

**Also available:**
- `/gsd:plan-phase ${PHASE} --skip-research` — plan without research
- `/gsd:ui-phase ${PHASE}` — generate UI design contract (if frontend work)
- Read the context back with `specforge query "$PHASE_ID" --depth 1 --format context` before continuing

---
```
</step>

<step name="auto_advance">
Check for auto-advance trigger:

1. Parse `--auto` flag from $ARGUMENTS
2. Sync chain flag:
   ```bash

   ```
3. Read consolidated auto-mode (`active` = chain flag OR user preference):
   ```bash
    AUTO_MODE="${AUTO_MODE:-false}"   # check auto-mode: the chain flag OR the user preference, from the settings block
   AUTO_MODE="${AUTO_MODE:-false}"
   ```

**If `--auto` flag present AND `AUTO_MODE` is not true:**
```bash
: # workflow._auto_chain_active is not a setting here: the chain lasts as long as the conversation, so there is nothing to set or clear
```

<!-- gsd:section id="auto-advance-dispatch" when="state:auto-advance-active" -->
If `section_manifest` is `null` or `"auto-advance-dispatch"` is in its `included` list: read and execute `gsd-core/workflows/discuss-phase-assumptions/steps/auto-advance-dispatch.md`. Otherwise skip — do not read the file.
<!-- /gsd:section -->

**If neither `--auto` nor config enabled:**
End here — `confirm_creation` already ran; do not route back to it.
</step>

</process>

<success_criteria>
- Phase validated against roadmap
- Prior context loaded (no re-asking decided questions)
- Codebase deeply analyzed via Explore subagent (5-15 files read)
- Assumptions surfaced with evidence and confidence levels
- User confirmed or corrected assumptions (~2-4 interactions max)
- Scope creep redirected to deferred ideas
- The phase context records actual decisions, as the same entities discuss mode records
- The context's `canonical_refs` give full file paths (MANDATORY)
- The context's `reusable_assets`, `patterns` and `integration_points` hold what the codebase analysis found
- The discussion log's entries keep the assumptions and corrections as the audit trail
- User knows next steps
</success_criteria>
