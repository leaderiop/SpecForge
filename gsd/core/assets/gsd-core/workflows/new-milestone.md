<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/new-milestone.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>

Start a new milestone cycle for an existing project. Loads project context, gathers milestone goals from the conversation (SpecForge keeps no milestone context file), records the milestone and the project's changes, optionally runs parallel research, defines scoped requirements with REQ-IDs, spawns the roadmapper to create phased execution plan, and commits all artifacts. Brownfield equivalent of new-project.

</purpose>

<required_reading>

Read all files referenced by the invoking prompt's execution_context before starting.

</required_reading>

<available_agent_types>
Valid GSD subagent types (use exact names — do not fall back to 'general-purpose'):
- gsd-project-researcher — Researches project-level technical decisions
- gsd-research-synthesizer — Synthesizes findings from parallel research agents
- gsd-roadmapper — Creates phased execution roadmaps
</available_agent_types>

<process>

## 1. Load Context

Parse `$ARGUMENTS` before doing anything else:

- `--reset-phase-numbers` flag → opt into restarting roadmap phase numbering at `1`. If absent, keep the current behavior of continuing phase numbering from the previous milestone.
- `--ws <name>` flag → active workstream scope, bound to `WS`
- the words that stand without an option → the milestone name if present, bound to `MILESTONE_NAME`

Bind `WS` and `MILESTONE_NAME` from the typed values:

```bash

# the commands below are specforge commands: there is no launcher to define
WS="{{arg:ws|}}"

MILESTONE_NAME="{{arg:name|}}"
# #4456: no file carries the workstream between fences: a later block that
# needs it sets WS from the ws argument again, as Step 1's WS= line does;
# the render fills that argument in every block of this prompt.

RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
# #2994: EARLY, section-manifest-only init.new-milestone call — needed here
# (before Step 4) to gate the project-md-milestone-write section. This is
# DELIBERATELY separate from Step 7's full init.new-milestone call below,
# which must stay AFTER Step 6's phase archival/phases.clear so its
# phase_dir_count / roadmap_exists / latest_completed_milestone fields
# reflect POST-archival state — moving that call here would compute those
# fields too early and corrupt the roadmapper's phase-numbering context.
# init.new-milestone is a pure read (no mutation), so calling it twice is
# safe; only `section_manifest` is consumed from this early call.
# #4456: the section manifest is the milestone's own: this query takes no
# workstream, so the ws argument is forwarded to none of these commands.

INIT_EARLY=$(specforge query "$MILESTONE" --depth 1 --format json)

```

`--ws` is forwarded nowhere: no command and no routing hint in this workflow takes a workstream. Step 4's shared-file guard reads `WS`, which Step 1 sets from the `ws` argument.

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations (including the "What do you want to build next?" prompt and seed-selection questions below) — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

- Read the project and its decisions (`specforge export --kinds project,decision --format context`): the existing project, its validated requirements
- Read the milestones (`specforge export --kinds milestone --format context`): what shipped previously
- The current position is computed (`specforge gsd progress-report`); pending todos and blockers are their capabilities' entities

## 2. Gather Milestone Goals

The goals come from the conversation: SpecForge keeps no milestone context file.

- Present what shipped in last milestone

**Text mode (`workflow.text_mode: true` in config or `--text` flag):** Set `TEXT_MODE=true` if `--text` is present in `$ARGUMENTS` OR `text_mode` from init JSON is `true`. When TEXT_MODE is active, replace every `AskUserQuestion` call with a plain-text numbered list and ask the user to type their choice number. This is required for non-Claude runtimes (OpenAI Codex, Antigravity, etc.) where `AskUserQuestion` is not available.
- Ask inline (freeform, NOT AskUserQuestion): "What do you want to build next?"
- Wait for their response, then use AskUserQuestion to probe specifics
- If user selects "Other" at any point to provide freeform input, ask follow-up as plain text — not another AskUserQuestion

## 2.5. Scan Planted Seeds

Check the seeds (if the project keeps any) for seed files that match the milestone goals gathered in step 2.

```bash
# seeds are not part of @gsd/core: list them only if the project keeps some
```

**If no seed files exist:** Skip this step silently — do not print any message or prompt.

**If seed files exist:** Read each `SEED-*.md` file and extract from its frontmatter and body:
- **Idea** — the seed title (heading after frontmatter, e.g. `# SEED-001: <idea>`)
- **Trigger conditions** — the `trigger_when` frontmatter field and the "When to Surface" section's bullet list
- **Planted during** — the `planted_during` frontmatter field (for context)

Compare each seed's trigger conditions against the milestone goals from step 2. A seed matches when its trigger conditions are relevant to any of the milestone's target features or goals.

**If no seeds match:** Skip silently — do not prompt the user.

**If matching seeds found:**

**`--auto` mode:** Auto-select ALL matching seeds. Log: `[auto] Selected N matching seed(s): [list seed names]`

**Text mode (`TEXT_MODE=true`):** Present matching seeds as a plain-text numbered list:
```
Seeds that match your milestone goals:
1. SEED-001: <idea> (trigger: <trigger_when>)
2. SEED-003: <idea> (trigger: <trigger_when>)

Enter numbers to include (comma-separated), or "none" to skip:
```

**Normal mode:** Present via AskUserQuestion:
```
AskUserQuestion(
  header: "Seeds",
  question: "These planted seeds match your milestone goals. Include any in this milestone's scope?",
  multiSelect: true,
  options: [
    { label: "SEED-001: <idea>", description: "Trigger: <trigger_when> | Planted during: <planted_during>" },
    ...
  ]
)
```

**After selection:**
- Selected seeds become additional context for requirement definition in step 9. Store them in an accumulator (e.g. `$SELECTED_SEEDS`) so step 9 can reference the ideas and their "Why This Matters" sections when defining requirements.
- Unselected seeds remain untouched — never delete or modify seed files during this workflow.

## 3. Determine Milestone Version

- Parse last version from MILESTONES.md
- Suggest next version (v1.0 → v1.1, or v2.0 for major)
- Confirm with user

## 3.5. Verify Milestone Understanding

Before writing any files, present a summary of what was gathered and ask for confirmation.

```
### GSD ► MILESTONE SUMMARY

**Milestone v[X.Y]: [Name]**

**Goal:** [One sentence]

**Target features:**
- [Feature 1]
- [Feature 2]
- [Feature 3]

**Key context:** [Any important constraints, decisions, or notes from questioning]
```

AskUserQuestion:
- header: "Confirm?"
- question: "Does this capture what you want to build in this milestone?"
- options:
  - "Looks good" — Proceed to record the milestone
  - "Adjust" — Let me correct or add details

**If "Adjust":** Ask what needs changing (plain text, NOT AskUserQuestion). Incorporate changes, re-present the summary. Loop until "Looks good" is selected.

**If "Looks good":** Proceed to Step 4.

## 4. Record the Milestone and the Project's Changes

The project is shared across workstreams (`gsd-core/references/workstream-flag.md` marks it `# Shared`). This step has two independently-scoped parts — only Part A is workstream-guarded.

<!-- gsd:section id="project-md-milestone-write" when="state:flat-mode" -->
If `section_manifest` (from `INIT_EARLY`) is `null` or `"project-md-milestone-write"` is in its `included` list: read and execute `gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md`. Otherwise (a workstream is active) skip — do not read the file; Part B below still runs regardless of the workstream.
<!-- /gsd:section -->

**Part B — the project's changes (always runs, regardless of the workstream).** Record what this milestone changes in the `project` entity with typed edits: `specforge set <project> what "…"` (likewise `context`, `core_value`) when its description drifted, and per new key decision the `new decision` and `link <project> key_decisions` lines of the project fence (`specforge gsd reference templates/project.md`). Leave the project as it is when nothing drifted. Nothing is backfilled: upstream's Evolution section describes what `/gsd-transition` and `/gsd:complete-milestone` do to the entities.

## 5. The Current Milestone

The current milestone is computed from the milestones and their phases' statuses: there is no state to reset, and the previous milestone's phases stay with it.

## 6. Commit

Nothing is cleared or archived from the previous milestone: its phases stay with it, and phase ids are milestone-scoped, so numbering may restart at 1 without a collision. The commit carries the milestone and the project's changes:

```bash
specforge git commit --add spec -m "docs: start milestone v[X.Y] [Name]" && specforge collect --runner gsd-git --yes
```

## 7. Load Context and Resolve Models

```bash

INIT=$(specforge query "$MILESTONE" --depth 1 --format json)

AGENT_SKILLS_RESEARCHER=""   # the host install carries the skills of gsd-project-researcher
AGENT_SKILLS_SYNTHESIZER=""   # the host install carries the skills of gsd-research-synthesizer
AGENT_SKILLS_ROADMAPPER=""   # the host install carries the skills of gsd-roadmapper
```
<!-- #4456: the workstream is not carried between steps: a block that
needs it sets WS from the ws argument itself, so no file holds it and
none is removed in Step 10. -->

Read from the answer: the node whose `id` is `$MILESTONE` is the milestone (`name` and `state` are its fields), and its phases are among the other nodes. `commit_docs` and `workflow.research` are settings: `specforge settings --format json --extension @gsd/core`; an installed agent carries its own model; the project, its roadmap and its requirements are entities, read with `specforge query`, so no path is read.

**If `agents_installed` is false:** Display a warning before proceeding:
```
⚠ GSD agents not installed. The following agents are missing from your agents directory:
  {missing_agents joined with newline}

Subagent spawns (gsd-project-researcher, gsd-research-synthesizer, gsd-roadmapper) will fail
with "agent type not found". Install them for this host (they are project files) to make them available:

  specforge install <host>   (claude-code, codex or cursor)

Proceeding without research subagents — roadmap will be generated inline.
```
Skip the parallel research spawn step and generate the roadmap inline.

<!-- gsd:section id="reset-phase-safety" when="flag:--reset-phase-numbers" -->
If `section_manifest` is `null` or `"reset-phase-safety"` is in its `included` list: read and execute `gsd-core/workflows/new-milestone/steps/reset-phase-safety.md`. Otherwise skip — do not read the file.
<!-- /gsd:section -->

## 8. Research Decision

Check `research_enabled` from init JSON (loaded from config).

**If `research_enabled` is `true`:**

AskUserQuestion: "Research the domain ecosystem for new features before defining requirements?"
- "Research first (Recommended)" — Discover patterns, features, architecture for NEW capabilities
- "Skip research for this milestone" — Go straight to requirements (does not change your default)

**If `research_enabled` is `false`:**

AskUserQuestion: "Research the domain ecosystem for new features before defining requirements?"
- "Skip research (current default)" — Go straight to requirements
- "Research first" — Discover patterns, features, architecture for NEW capabilities

**IMPORTANT:** Do NOT persist this choice as a setting (`specforge settings set`). The `workflow.research` setting is a persistent user preference that controls plan-phase behavior across the project. Changing it here would silently alter future `/gsd:plan-phase` behavior. To change the default, use `/gsd:settings`.

**If user chose "Research first":**

```
### GSD ► RESEARCHING

◆ Spawning 4 researchers in parallel... (each runs in a subagent — no output until they return, ~1–5 min; expected, not a freeze)
  → Stack, Features, Architecture, Pitfalls
```

**Record the project research:** the four researchers record their items in the project research `RESEARCH`, and the synthesizer sets its summary. Record it first; a later milestone's research takes the same entity back, its summary pending again:

```bash
IFS= read -rd '' DOMAIN <<'TEXT' || true; DOMAIN=${DOMAIN%$'\n'}
{the domain researched}
TEXT
RESEARCHED="{today, YYYY-MM-DD}"
PENDING="Pending: the research synthesizer sets the summary"
if specforge query RESEARCH --depth 0 --format json >/dev/null 2>&1; then
  specforge set RESEARCH summary "$PENDING" || exit 1
else
  specforge new research RESEARCH "Project research" --in spec/research.spec --domain "$DOMAIN" --researched "$RESEARCHED" --confidence low --summary "$PENDING" || exit 1
fi
```

Spawn 4 parallel gsd-project-researcher agents. Each uses this template with dimension-specific fields:

**Common structure for all 4 researchers:**
<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`researcher_model`, `synthesizer_model`, `roadmapper_model`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

```text
Agent(prompt="
<research_type>Project Research — {DIMENSION} for [new features].</research_type>

<milestone_context>
SUBSEQUENT MILESTONE — Adding [target features] to existing app.
{EXISTING_CONTEXT}
Focus ONLY on what's needed for the NEW features.
</milestone_context>

<question>{QUESTION}</question>

<required_reading>
- specforge context gsd/project-researcher {project} (the researcher pack: the project, its requirements, decisions and milestones)
</required_reading>

${AGENT_SKILLS_RESEARCHER}

<downstream_consumer>{CONSUMER}</downstream_consumer>

<quality_gate>{GATES}</quality_gate>

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the role name as it is installed. On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

<output>
Record the {DIMENSION} items in `RESEARCH` with the fence of {FILE}.
</output>
", subagent_type="gsd-project-researcher", model="{researcher_model}", description="{DIMENSION} research")
```

**Dimension-specific fields:**

| Field | Stack | Features | Architecture | Pitfalls |
|-------|-------|----------|-------------|----------|
| EXISTING_CONTEXT | Existing validated capabilities (DO NOT re-research): [the project's `validated`] | Existing features (already built): [the project's `validated`] | Existing architecture: [the project's context or the codebase map] | Focus on common mistakes when ADDING these features to existing system |
| QUESTION | What stack additions/changes are needed for [new features]? | How do [target features] typically work? Expected behavior? | How do [target features] integrate with existing architecture? | Common mistakes when adding [target features] to [domain]? |
| CONSUMER | Specific libraries with versions for NEW capabilities, integration points, what NOT to add | Table stakes vs differentiators vs anti-features, complexity noted, dependencies on existing | Integration points, new components, data flow changes, suggested build order | Warning signs, prevention strategy, which phase should address it |
| GATES | Versions current (verify with Context7), rationale explains WHY, integration considered | Categories clear, complexity noted, dependencies identified | Integration points identified, new vs modified explicit, build order considers deps | Pitfalls specific to adding these features, integration pitfalls covered, prevention actionable |
| FILE | ~/.claude/gsd-core/templates/research-project/STACK.md | ~/.claude/gsd-core/templates/research-project/FEATURES.md | ~/.claude/gsd-core/templates/research-project/ARCHITECTURE.md | ~/.claude/gsd-core/templates/research-project/PITFALLS.md |

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling all 4 researcher Agent() calls above, do NOT read research files or synthesize content independently while the subagents are active. Wait for all 4 researchers to complete before spawning the synthesizer. This prevents duplicate work and wasted context.

After all 4 complete, spawn the synthesizer to set the research summary:

```text
Agent(prompt="
<task>
Synthesize the project research into its summary.
</task>

<required_reading>
- specforge context gsd/research-synthesizer {project} (the synthesizer pack: the project, `RESEARCH` with its stack choices, findings, patterns and pitfalls, and the requirements)
</required_reading>

${AGENT_SKILLS_SYNTHESIZER}

<output>
Set the summary of `RESEARCH` with the fence of ~/.claude/gsd-core/templates/research-project/SUMMARY.md.
Commit after setting it (`specforge git commit --add spec`).
</output>
", subagent_type="gsd-research-synthesizer", model="{synthesizer_model}", description="Synthesize research")
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

**Synthesizer output self-heal (#222): verify the summary is set.** The synthesizer's canonical output is the `summary` of `RESEARCH`; its brief structured return (`## SYNTHESIS COMPLETE` plus a few `###` confirmation lines) is NOT the summary. A known LLM false-refusal (issue #222) sometimes makes the agent return the full summary inline, fabricating a restriction (e.g. "the runtime is blocking writes"), instead of setting it, and a runtime may refuse a subagent's tool call for real. Prompt hardening alone does not fully eliminate it, so the orchestrator MUST absorb either cause deterministically before spawning `gsd-roadmapper`:

1. Check that the summary is set: until the synthesizer sets it, it reads the pending text the research was recorded with.

   ```bash
   PENDING="Pending: the research synthesizer sets the summary"
   NOW=$(specforge query RESEARCH --depth 0 --format json --pick '.nodes[0].fields.summary') || exit 1
   if [ -n "$NOW" ] && [ "$NOW" != "$PENDING" ]; then echo "summary: recorded"; else echo "summary: pending"; fi
   ```

   On `summary: recorded`, continue normally.
2. On `summary: pending`, if the synthesizer's return holds the FULL summary (recognizable by its sections `## Executive Summary`, `## Key Findings`, `## Implications for Roadmap` and `## Sources`, not merely the brief `## SYNTHESIS COMPLETE` confirmation), the synthesizer returned it inline: set it from the return, the text in the quoted heredoc as it is.

   ```bash
   CONFIDENCE="{high, medium or low: the overall confidence the return states}"
   IFS= read -r -d '' SUMMARY <<'SUMMARY_EOF'
   {the summary the synthesizer returned}
   SUMMARY_EOF
   specforge set RESEARCH summary "$SUMMARY" || exit 1
   specforge set RESEARCH confidence "$CONFIDENCE" || exit 1
   ```

   Then commit the research (the synthesizer commits on behalf of the four researchers): `specforge git commit --add spec -m "docs: complete project research" && specforge collect --runner gsd-git --yes`. Log `⚠ #222 self-heal: synthesizer returned the summary inline without setting it; orchestrator set it.`
3. On `summary: pending` with only a brief confirmation (no full summary to recover), the synthesizer genuinely failed: surface the error and stop; do NOT spawn `gsd-roadmapper` against a pending summary.

This guarantees `gsd-roadmapper` (whose pack holds the research) never runs against a pending summary.

Display the key findings of the summary:
```
### GSD ► RESEARCH COMPLETE ✓

**Stack additions:** [from the summary]
**Feature table stakes:** [from the summary]
**Watch Out For:** [from the summary]
```

**If "Skip research":** Continue to Step 9.

## 9. Define Requirements

```
### GSD ► DEFINING REQUIREMENTS
```

Read the project and the milestone (`specforge export --kinds project,milestone --format context`): core value, the milestone's goal and target features, the validated requirements (what exists).

**If `$SELECTED_SEEDS` is non-empty (from step 2.5):** Include selected seed ideas and their "Why This Matters" sections as additional input when defining requirements. Seeds provide user-validated feature ideas that should be incorporated into the requirement categories alongside research findings or conversation-gathered features.

**If research exists:** read the feature findings of `RESEARCH` (`specforge query RESEARCH --depth 1 --format context`), extract feature categories.

Present features by category:
```
## [Category 1]
**Table stakes:** Feature A, Feature B
**Differentiators:** Feature C, Feature D
**Research notes:** [any relevant notes]
```

**If no research:** Gather requirements through conversation. Ask: "What are the main things users need to do with [new features]?" Clarify, probe for related capabilities, group into categories.

**Scope each category** via AskUserQuestion (multiSelect: true, header max 12 chars):
- "[Feature 1]" — [brief description]
- "[Feature 2]" — [brief description]
- "None for this milestone" — Defer entire category

Track: Selected → this milestone. Unselected table stakes → future. Unselected differentiators → out of scope.

**Identify gaps** via AskUserQuestion:
- "No, research covered it" — Proceed
- "Yes, let me add some" — Capture additions

**Record the requirements:**

- **Future and out-of-scope requirements** (tier `v2` or `out_of_scope`): record them now, one line per requirement, the requirement fence of `templates/requirements.md`. `--reason` is for an out-of-scope requirement: leave it out for a `v2` one.

  ```bash
  REQ_ID="{CATEGORY-NN, continuing the numbering}"
  IFS= read -rd '' TEXT <<'TEXT' || true; TEXT=${TEXT%$'\n'}
  {the requirement: user-centric, testable, atomic}
  TEXT
  IFS= read -rd '' CATEGORY <<'TEXT' || true; CATEGORY=${CATEGORY%$'\n'}
  {its category}
  TEXT
  TIER="{v2 or out_of_scope}"
  IFS= read -rd '' REASON <<'TEXT' || true; REASON=${REASON%$'\n'}
  {why it is out of scope}
  TEXT
  specforge new requirement "$REQ_ID" "$TEXT" --in spec/requirements.spec --text "$TEXT" --category "$CATEGORY" --tier "$TIER" --reason "$REASON" || exit 1
  ```

- **This milestone's v1 requirements:** hold them. A v1 requirement must be in a phase (`gsd/E101`), and the project has phases, so one recorded alone is refused. Show them for approval below, then hand them to the roadmapper in the spawn's `<requirements>` block (Step 10): it writes them with the phases in its one roadmap edit.

**REQ-ID format:** `[CATEGORY]-[NUMBER]` (AUTH-01, NOTIF-02). Continue numbering from existing.

**Requirement quality criteria:**

Good requirements are:
- **Specific and testable:** "User can reset password via email link" (not "Handle password reset")
- **User-centric:** "User can X" (not "System does Y")
- **Atomic:** One capability per requirement (not "User can login and manage profile")
- **Independent:** Minimal dependencies on other requirements

Present FULL requirements list for confirmation:

```
## Milestone v[X.Y] Requirements

### [Category 1]
- [ ] **CAT1-01**: User can do X
- [ ] **CAT1-02**: User can do Y

### [Category 2]
- [ ] **CAT2-01**: User can do Z

Does this capture what you're building? (yes / adjust)
```

If "adjust": Return to scoping.

**Commit the recorded requirements** (the v1 ones are committed with the roadmap):
```bash

specforge git commit --add spec -m "docs: define milestone v[X.Y] requirements" && specforge collect --runner gsd-git --yes
```

## 10. Create Roadmap

```
### GSD ► CREATING ROADMAP

◆ Spawning roadmapper... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)
```

**Starting phase number:**
- If `--reset-phase-numbers` is active, start at **Phase 1**
- Otherwise, continue from the previous milestone's last phase number (v1.0 ended at phase 5 → v1.1 starts at phase 6)

```text
Agent(prompt="
<planning_context>
<required_reading>
- specforge context gsd/roadmapper {project} (the roadmapper pack: the project, its decisions, the requirements, the project research and its summary if any, the milestones)
- {config_path}
</required_reading>

${AGENT_SKILLS_ROADMAPPER}

</planning_context>

<requirements>
Milestone v[X.Y] [Name] (recorded): [its goal]
New v1 requirements, to record with the phases in the roadmap edit:
- [CAT-NN]: [text] ([Category])
</requirements>

<instructions>
Create roadmap for milestone v[X.Y]:
1. Respect the selected numbering mode:
   - `--reset-phase-numbers` → start at Phase 1
   - default behavior → continue from the previous milestone's last phase number
2. Derive phases from THIS MILESTONE's requirements only (the `<requirements>` block)
3. Map every requirement to exactly one phase; leave out the milestone op (it is recorded)
4. Derive 2-5 success criteria per phase (observable user behaviors)
5. Validate 100% coverage
6. Write the roadmap edit immediately: every phase, with the new requirements, in one `specforge.edit` call (each phase's `requirements` is the traceability; STATE is computed)
7. Return ROADMAP CREATED with summary

Write the roadmap first, then return.
</instructions>
", subagent_type="gsd-roadmapper", model="{roadmapper_model}", description="Create roadmap")
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

**Handle return:**

**If `## ROADMAP BLOCKED`:** Present blocker, work with user, re-spawn.

**If `## ROADMAP CREATED`:** Read the milestone's phases (`specforge export --kinds milestone,phase --format context`), present inline:

```
## Proposed Roadmap

**[N] phases** | **[X] requirements mapped** | All covered ✓

| # | Phase | Goal | Requirements | Success Criteria |
|---|-------|------|--------------|------------------|
| [N] | [Name] | [Goal] | [REQ-IDs] | [count] |

### Phase Details

**Phase [N]: [Name]**
Goal: [goal]
Requirements: [REQ-IDs]
Success criteria:
1. [criterion]
2. [criterion]
```

**Ask for approval** via AskUserQuestion:
- "Approve" — Commit and continue
- "Adjust phases" — Tell me what to change
- "Review full roadmap" — Show the milestone and its phases in full

**If "Adjust":** Get notes, re-spawn roadmapper with revision context, loop until approved.
**If "Review":** Display the milestone and its phases in full, re-ask.

**Commit roadmap** (after approval):
```bash

specforge git commit --add spec -m "docs: create milestone v[X.Y] roadmap ([N] phases)" && specforge collect --runner gsd-git --yes
# #4456: no workstream file was persisted, so nothing is removed here.

# nothing to clean up: no scratch file was written
```

## 10.5. Link Pending Todos to Roadmap Phases

After roadmap approval, scan pending todos against the newly approved phases. For each todo whose scope matches a phase, tag it with `resolves_phase: N` in its YAML frontmatter.

**Check for pending todos:**
```bash
PENDING_TODOS=""   # todos are not part of @gsd/core; fill this only if the project keeps some
```

**If no pending todos exist:** Skip this step silently.

**If pending todos exist:**

Read the approved ROADMAP.md and extract the phase list: phase number, phase name, goal, and requirement IDs.

For each pending todo, compare:
- The todo's `title` and `area` frontmatter fields
- The todo body (Problem and Solution sections)

Against each phase's:
- Phase goal
- Requirement IDs and descriptions

**Match criteria (best-effort — do not over-match):** A todo is considered resolved by a phase if the phase's goal or requirements directly describe implementing the same feature, area, or capability as the todo. Narrow, specific todos with concrete scopes are the best candidates. Vague or cross-cutting todos should be left unlinked.

**For each matched todo**, add `resolves_phase: [N]` to the YAML frontmatter block (after the existing fields):
```yaml
---
created: [existing]
title: [existing]
area: [existing]
resolves_phase: [N]
files: [existing]
---
```

**Only modify todos that have a clear, confident match.** Leave unmatched todos unmodified.

**If any todos were linked:**
```bash
specforge git commit -m "docs: tag [count] pending todos with resolves_phase after milestone v[X.Y] roadmap"
```

Print a summary:
```
◆ Linked [N] pending todos to roadmap phases:
  → [todo title] → Phase [N]: [Phase Name]
  (Leave [M] unmatched todos in pending/)
```

## 11. Done

```
### GSD ► MILESTONE INITIALIZED ✓

**Milestone v[X.Y]: [Name]**

| Artifact       | Location                    |
|----------------|-----------------------------|
| Project        | the `project` entity      |
| Research       | the `research` entities       |
| Requirements   | the `requirement` entities |
| Roadmap        | the `milestone` and `phase` entities      |

**[N] phases** | **[X] requirements** | Ready to build ✓

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Phase [N]: [Phase Name]** — [Goal]

`/clear` then:

`/gsd:discuss-phase [N]` — gather context and clarify approach

Also: `/gsd:plan-phase [N]` — skip discussion, plan directly
```

</process>

<success_criteria>
- [ ] The milestone recorded with its goal and target features (skipped when a workstream is active — see Step 4); the project's changes recorded
- [ ] Research completed (if selected) — 4 parallel agents, milestone-aware
- [ ] Requirements gathered and scoped per category
- [ ] Future and out-of-scope requirements recorded; this milestone's v1 requirements handed to the roadmapper with REQ-IDs
- [ ] gsd-roadmapper spawned with phase numbering context
- [ ] Roadmap written immediately, in one edit (not draft)
- [ ] User feedback incorporated (if any)
- [ ] Phase numbering mode respected (continued or reset)
- [ ] All commits made (if planning docs committed)
- [ ] Pending todos scanned for phase matches; matched todos tagged with `resolves_phase: N`
- [ ] User knows next step: `/gsd:discuss-phase [N]`

**Atomic commits:** Each phase commits its artifacts immediately.
</success_criteria>
</output>
