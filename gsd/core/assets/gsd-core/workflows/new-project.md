<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/new-project.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
<purpose>
Initialize a new project through unified flow: questioning, research (optional), requirements, roadmap. This is the most leveraged moment in any project — deep questioning here means better plans, better execution, better outcomes. One workflow takes you from idea to ready-for-planning.
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

<auto_mode>

<!-- gsd:section id="auto-mode-detection" when="flag:--auto" -->
If `section_manifest` is `null` or `"auto-mode-detection"` is in its `included` list: read and execute `gsd-core/workflows/new-project/steps/auto-mode-detection.md`. Otherwise skip — do not read the file.
<!-- /gsd:section -->

</auto_mode>

<process>

**Compact Content Gate.** Read and follow `gsd-core/references/compact-content-gate.md` now — it states the `workflow.compact_content` check and the resolution rule this spine defers to. When it directs a Read, read `gsd-core/workflows/new-project/detail/elaboration.md` in full before continuing past this point; its content elaborates on two sections below (Step 2b's prior spike/sketch detection, and Step 6's researcher/synthesizer prompts).

## 1. Setup

**MANDATORY FIRST STEP — Execute these checks before ANY user interaction:**

```bash

# the commands below are specforge commands: there is no launcher to define

INIT=$(specforge check --format json)

AGENT_SKILLS_RESEARCHER=""   # the host install carries the skills of gsd-project-researcher
AGENT_SKILLS_SYNTHESIZER=""   # the host install carries the skills of gsd-research-synthesizer
AGENT_SKILLS_ROADMAPPER=""   # the host install carries the skills of gsd-roadmapper
```

`INIT` is `specforge check`'s answer: the project's diagnostics as it stands. The project, its requirements and its roadmap are entities, read with `specforge query`, so no path is read; `commit_docs` and `response_language` are settings: `specforge settings --format json --extension @gsd/core`; an installed agent carries its own model.

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

**If `agents_installed` is false:** Display a warning before proceeding:
```text
⚠ GSD agents not installed. The following agents are missing from your agents directory:
  {missing_agents joined with newline}

Runtime checked: {agent_runtime}
Agents directory checked: {agents_dir}
Required new-project agents missing:
  {missing_required_agents joined with newline, or "none"}

Agent skill payloads available: {agent_skill_payloads_available}
Agent skill payload agents:
  {agent_skill_payload_agents joined with newline, or "none"}

Skill payloads only provide prompt context. Named subagent spawns still require agent
definitions to be installed for this runtime.

Subagent spawns (gsd-project-researcher, gsd-research-synthesizer, gsd-roadmapper) will fail
with "agent type not found" if `required_agents_installed` is false. Install them for this host (they are project files) to make them available:

  specforge install <host>   (claude-code, codex or cursor)

Proceeding without research subagents — roadmap will be generated inline.
```
Skip Steps 6–7 (parallel research and synthesis) and proceed directly to roadmap creation in Step 8.

**Detect runtime and set instruction file name:**

Derive `RUNTIME` from the invoking prompt's `execution_context` path:
- Path contains `/.codex/` → `RUNTIME=codex`
- Path contains `/.gemini/antigravity/`, `/.gemini/antigravity-ide/` or `/.gemini/antigravity-cli/` → `RUNTIME=antigravity`
- Path contains `/.config/opencode/` or `/.opencode/` → `RUNTIME=opencode`
- Path contains `/.trae/` → `RUNTIME=trae`
- Otherwise → `RUNTIME=claude`

If `execution_context` path is not available, fall back to env vars:
```bash
if [ -n "$CODEX_HOME" ]; then RUNTIME="codex"
elif [ -n "$ANTIGRAVITY_CONFIG_DIR" ]; then RUNTIME="antigravity"
elif [ -n "$OPENCODE_CONFIG_DIR" ] || [ -n "$OPENCODE_CONFIG" ]; then RUNTIME="opencode"
elif [ -n "$TRAE_CONFIG_DIR" ]; then RUNTIME="trae"
else RUNTIME="claude"; fi
```

Set the instruction file variable via the shared runtime-name policy adapter (the host install knows its own instruction file):
```bash
INSTRUCTION_FILE=""   # the host install writes the instruction file; leave it empty here
```

All subsequent references to the project instruction file use `$INSTRUCTION_FILE`.

**If `project_exists` is true and `init_incomplete` is true (#4040 — interrupted bootstrap):** Resume initialization instead of erroring. the spec exists but initialization stopped before all core artifacts landed. Keep the `project` entity and whatever is already recorded (its requirements, `config.json`); skip the steps that would record them again and continue the flow from the first missing piece in init order (the requirements, then the roadmap) until all exist. Do not error and do not bounce the user back to `/gsd:progress` (that routing loop is the #4040 bug).

**If `project_exists` is true and `init_incomplete` is false:** Error — project already initialized. Use `/gsd:progress`.

**Git init (#3491 — never nest `.git` inside an existing worktree):**

- If `has_git` true and `in_nested_subdir` true: skip `git init`; warn `⚠ Initializing inside existing worktree (${git_worktree_root}); planning files will track to outer repo.`
- If `has_git` true and `in_nested_subdir` false: skip `git init` (already at worktree root).
- If `has_git` false: `git init`.

## 2. Brownfield Offer

**If auto mode:** Skip to Step 4 (assume greenfield, synthesize the project from provided document).

<!-- gsd:section id="codebase-map-offer" when="state:needs-codebase-map" -->
If `section_manifest` is `null` or `"codebase-map-offer"` is in its `included` list: read and execute `gsd-core/workflows/new-project/steps/codebase-map-offer.md`. Otherwise skip — do not read the file.
<!-- /gsd:section -->

**If "Skip mapping" OR `needs_codebase_map` is false:** Continue to Step 3.

<!-- gsd:section id="auto-mode-config" when="flag:--auto" -->
If `section_manifest` is `null` or `"auto-mode-config"` is in its `included` list: read and execute `gsd-core/workflows/new-project/steps/auto-mode-config.md`. Otherwise skip — do not read the file.
<!-- /gsd:section -->

## 2b. Prior Spike/Sketch Detection

Check for a spike or sketch findings skill, or raw spike or sketch manifests (they are not part of @gsd/core). If any exist, surface them before questioning (which findings-skill, if any, and any raw un-wrapped spikes or sketches worth `/gsd:spike --wrap-up` / `/gsd:sketch --wrap-up`), and if a findings skill exists, read its SKILL.md to inform the questioning phase — it carries validated patterns, constraints, and design decisions that should shape the project definition.

Exact detection commands and the surfaced-findings banner: `gsd-core/workflows/new-project/detail/elaboration.md` § 1.

## 3. Deep Questioning

**If auto mode:** Skip (already handled in Step 2a). Extract project context from provided document instead and proceed to Step 4.

**Display stage banner:**

```
### GSD ► QUESTIONING
```

**Open the conversation:**

Ask inline (freeform, NOT AskUserQuestion):

"What do you want to build?"

Wait for their response. This gives you the context needed to ask intelligent follow-up questions.

**Research-before-questions mode:** Check if `workflow.research_before_questions` is enabled in the settings (`specforge settings`) (or the config from init context). When enabled, before asking follow-up questions about a topic area:

1. Do a brief web search for best practices related to what the user described
2. Mention key findings naturally as you ask questions (e.g., "Most projects like this use X — is that what you're thinking, or something different?")
3. This makes questions more informed without changing the conversational flow

When disabled (default), ask questions directly as before.

**Follow the thread:**

Based on what they said, ask follow-up questions that dig into their response. Use AskUserQuestion with options that probe what they mentioned — interpretations, clarifications, concrete examples.

Keep following threads. Each answer opens new threads to explore. Ask about:

- What excited them
- What problem sparked this
- What they mean by vague terms
- What it would actually look like
- What's already decided

Consult `questioning.md` for techniques:

- Challenge vagueness
- Make abstract concrete
- Surface assumptions
- Find edges
- Reveal motivation

**Check context (background, not out loud):**

As you go, mentally check the context checklist from `questioning.md`. If gaps remain, weave questions naturally. Don't suddenly switch to checklist mode.

**Decision gate:**

When you could record a clear project, use AskUserQuestion:

- header: "Ready?"
- question: "I think I understand what you're after. Ready to record the project?"
- options:
  - "Record the project" — Let's move forward
  - "Keep exploring" — I want to share more / ask me more

If "Keep exploring" — ask what they want to add, or identify gaps and probe naturally.

Loop until "Record the project" selected.

## 4. Record the project

**If auto mode:** Synthesize from provided document. No "Ready?" gate was shown — proceed directly to commit.

Synthesize all context into the `project` entity and its key decisions: run **the project fence** in `templates/project.md` (`specforge gsd reference templates/project.md`), its values filled from what was gathered, one `new decision` and `link` per decision made during questioning (outcome `pending`). Do not compress. Capture everything gathered.

The requirements are not part of the project: Step 7 records them as `requirement` entities, and each starts active, its status computed as phases cover and verify it. For a brownfield project the `@gsd/codebase` map (`/gsd:map-codebase`) records what the code already does; read it and write the current state into `context`. The project evolves through typed edits at phase transitions and milestone boundaries (`workflows/transition.md`, `workflows/complete-milestone.md`): nothing is appended to it.

**Commit the project:**

```bash
specforge git commit --add spec -m "docs: initialize project" && specforge collect --runner gsd-git --yes
```

## 5. Workflow Preferences

**If auto mode:** Skip — config was collected in Step 2a. Proceed to Step 5.5.

**Check for global defaults** at `${GSD_HOME:-$HOME}/.gsd/defaults.json` (`~/.gsd/defaults.json` unless `GSD_HOME` is set). If the file exists, read it with your Read tool (it is the user's, outside the project's roots, which no block reaches) and display its contents before asking.


Format the JSON into human-readable bullets using these label mappings:
- `mode` → "Mode"
- `granularity` → "Granularity"
- `parallelization` → "Execution" (`true` → "Parallel", `false` → "Sequential")
- `commit_docs` → "Git Tracking" (`true` → "Yes", `false` → "No")
- `model_profile` → "AI Models"
- `workflow.research` → "Research" (`true` → "Yes", `false` → "No")
- `workflow.plan_check` → "Plan Check" (`true` → "Yes", `false` → "No")
- `workflow.verifier` → "Verifier" (`true` → "Yes", `false` → "No")
- `plan_review.source_grounding` → "Drift Guard" (`true` → "Yes", `false` → "No")

Display above the prompt:

```text
Your saved defaults (~/.gsd/defaults.json):
  • Mode: [value]
  • Granularity: [value]
  • Execution: [Parallel|Sequential]
  • Git Tracking: [Yes|No]
  • AI Models: [value]
  • Research: [Yes|No]
  • Plan Check: [Yes|No]
  • Verifier: [Yes|No]
  • Drift Guard: [Yes|No]
```

Then ask:

```text
AskUserQuestion([
  {
    question: "Use these saved defaults?",
    header: "Defaults",
    multiSelect: false,
    options: [
      { label: "Use as-is (Recommended)", description: "Proceed with the defaults shown above" },
      { label: "Modify some settings", description: "Keep defaults, change a few" },
      { label: "Configure fresh", description: "Walk through all questions from scratch" }
    ]
  }
])
```

**If "Use as-is":** use the defaults values for config.json and skip directly to **Commit config.json** below.

**If "Modify some settings":** present a selection of every setting with its current saved value.

**If TEXT_MODE is active** (non-Claude runtimes): display a numbered list and ask the user to type the numbers of settings they want to change (comma-separated). Parse the response and proceed.

```text
Which settings do you want to change? (enter numbers, comma-separated)

  1. Mode — Currently: [value]
  2. Granularity — Currently: [value]
  3. Execution — Currently: [Parallel|Sequential]
  4. Git Tracking — Currently: [Yes|No]
  5. AI Models — Currently: [value]
  6. Research — Currently: [Yes|No]
  7. Plan Check — Currently: [Yes|No]
  8. Verifier — Currently: [Yes|No]
  9. Drift Guard — Currently: [Yes|No]
```

**Otherwise** (Claude runtime with AskUserQuestion): use a two-block split
to stay within the 4-option runtime cap.

```text
AskUserQuestion([
  {
    question: "Do you want to change any core workflow settings (Mode, Granularity, Execution, Git Tracking)?",
    header: "Core Settings",
    multiSelect: false,
    options: [
      { label: "Yes", description: "Choose from core workflow settings" },
      { label: "No", description: "Skip core workflow settings" }
    ]
  }
])
```

If "Yes", ask:

```text
AskUserQuestion([
  {
    question: "Which core workflow settings do you want to change?",
    header: "Core Select",
    multiSelect: true,
    options: [
      { label: "Mode", description: "Currently: [value]" },
      { label: "Granularity", description: "Currently: [value]" },
      { label: "Execution", description: "Currently: [Parallel|Sequential]" },
      { label: "Git Tracking", description: "Currently: [Yes|No]" }
    ]
  }
])
```

Then ask:

```text
AskUserQuestion([
  {
    question: "Do you want to change any model/agent settings (AI Models, Research, Plan Check, Verifier)?",
    header: "Agent Settings",
    multiSelect: false,
    options: [
      { label: "Yes", description: "Choose from model/agent settings" },
      { label: "No", description: "Skip model/agent settings" }
    ]
  }
])
```

If "Yes", ask:

```text
AskUserQuestion([
  {
    question: "Which model/agent settings do you want to change?",
    header: "Agent Select",
    multiSelect: true,
    options: [
      { label: "AI Models", description: "Currently: [value]" },
      { label: "Research", description: "Currently: [Yes|No]" },
      { label: "Plan Check", description: "Currently: [Yes|No]" },
      { label: "Verifier", description: "Currently: [Yes|No]" }
    ]
  }
])
```

Then ask:

```text
AskUserQuestion([
  {
    question: "Do you want to change the Drift Guard setting (plan-review source-grounding)?",
    header: "Drift Guard",
    multiSelect: false,
    options: [
      { label: "Yes", description: "Toggle Drift Guard (currently: [Yes|No])" },
      { label: "No", description: "Keep current Drift Guard setting" }
    ]
  }
])
```

For each selected setting across both blocks, ask only that question using the
option set from Round 1 / Round 2 below. Merge user answers over the saved
defaults — unchanged settings retain their saved values. Then skip to
**Commit config.json**.

**If "Configure fresh" or `~/.gsd/defaults.json` doesn't exist:** proceed with the questions below.

**Round 1 — Core workflow settings (4 questions):**

```
questions: [
  {
    header: "Mode",
    question: "How do you want to work?",
    multiSelect: false,
    options: [
      { label: "YOLO (Recommended)", description: "Auto-approve, just execute" },
      { label: "Interactive", description: "Confirm at each step" }
    ]
  },
  {
    header: "Granularity",
    question: "How finely should scope be sliced into phases?",
    multiSelect: false,
    options: [
      { label: "Coarse", description: "Fewer, broader phases (3-5 phases, 1-3 plans each)" },
      { label: "Standard", description: "Balanced phase size (5-8 phases, 3-5 plans each)" },
      { label: "Fine", description: "Many focused phases (8-12 phases, 5-10 plans each)" }
    ]
  },
  {
    header: "Execution",
    question: "Run plans in parallel?",
    multiSelect: false,
    options: [
      { label: "Parallel (Recommended)", description: "Independent plans run simultaneously" },
      { label: "Sequential", description: "One plan at a time" }
    ]
  },
  {
    header: "Git Tracking",
    question: "Commit planning docs to git?",
    multiSelect: false,
    options: [
      { label: "Yes (Recommended)", description: "Planning docs tracked in version control" },
      { label: "No", description: "Keep the spec local-only (add spec/ to .gitignore, creating it if needed)" }
    ]
  }
]
```

**Round 2 — Workflow agents:**

These spawn additional agents during planning/execution. They add tokens and time but improve quality.

| Agent | When it runs | What it does |
|-------|--------------|--------------|
| **Researcher** | Before planning each phase | Investigates domain, finds patterns, surfaces gotchas |
| **Plan Checker** | After plan is created | Verifies plan actually achieves the phase goal |
| **Verifier** | After phase execution | Confirms must-haves were delivered |

All recommended for important projects. Skip for quick experiments.

A fourth question in this same round covers Compact Content (#4139) — not a spawned agent,
but grouped here because it's the last general workflow-behavior toggle before the more
involved AI-models round below.

```
questions: [
  {
    header: "Research",
    question: "Research before planning each phase? (adds tokens/time)",
    multiSelect: false,
    options: [
      { label: "Yes (Recommended)", description: "Investigate domain, find patterns, surface gotchas" },
      { label: "No", description: "Plan directly from requirements" }
    ]
  },
  {
    header: "Plan Check",
    question: "Verify plans will achieve their goals? (adds tokens/time)",
    multiSelect: false,
    options: [
      { label: "Yes (Recommended)", description: "Catch gaps before execution starts" },
      { label: "No", description: "Execute plans without verification" }
    ]
  },
  {
    header: "Verifier",
    question: "Verify work satisfies requirements after each phase? (adds tokens/time)",
    multiSelect: false,
    options: [
      { label: "Yes (Recommended)", description: "Confirm deliverables match phase goals" },
      { label: "No", description: "Trust execution, skip verification" }
    ]
  },
  {
    header: "Compact Content",
    question: "Use token-minimized instruction content where available? (smaller context footprint)",
    multiSelect: false,
    options: [
      { label: "No (Recommended)", description: "Full instruction detail loaded every time. Best while evaluating GSD or on a large context window." },
      { label: "Yes", description: "Terser instructions where a compact variant exists; canonical detail loads only when actually needed. Frees up context for long sessions or large codebases." }
    ]
  }
]

// Model profile uses a two-question split because AskUserQuestion enforces a hard
// 4-option cap and there are 5 valid profiles (quality, balanced, budget, adaptive,
// inherit). Q1 routes between adaptive/standard-tier/inherit; Q2 (shown only when
// Q1 = "Standard tier…") picks among the three standard profiles. Mirrors the
// /gsd:settings split (#3784, #1516).
questions: [
  {
    header: "AI Models",
    question: "Which AI models for planning agents?",
    multiSelect: false,
    options: [
      { label: "Adaptive (Recommended)", description: "Role-based cost optimization: heavy roles use the highest-tier model available on the active runtime, light roles use the cheapest. Best balance of quality and cost across all supported runtimes (Claude, Codex, Antigravity, OpenRouter, local)." },
      { label: "Standard tier…", description: "Choose Quality, Balanced, or Budget — flat tier applied to all agents" },
      { label: "Inherit", description: "Use the current session model for all agents (required for non-Claude runtimes: Codex, Antigravity, OpenCode /model, OpenRouter, local models)" }
    ]
  }
]

**Conditional visibility — model_profile (Q2):**
  Only ask this question when Q1's answer is "Standard tier…".
  If Q1 = "Adaptive (Recommended)" → write model_profile=adaptive and SKIP Q2.
  If Q1 = "Inherit"                → write model_profile=inherit and SKIP Q2.
  If user cancels Q2 after picking "Standard tier…" → leave existing model_profile value unchanged.

questions: [
  {
    question: "Which standard profile? (Quality / Balanced / Budget)",
    header: "Model Tier",
    multiSelect: false,
    options: [
      { label: "Quality", description: "Opus everywhere except verification (highest cost) — Claude only" },
      { label: "Balanced", description: "Opus for planning, Sonnet for research/execution/verification — Claude only" },
      { label: "Budget", description: "Sonnet for writing, Haiku for research/verification (lowest cost) — Claude only" }
    ]
  }
]

// Map UI choices → config values:
//   Q1 "Adaptive (Recommended)"         → model_profile = "adaptive"
//   Q1 "Inherit"                        → model_profile = "inherit"
//   Q1 "Standard tier…" + Q2 "Quality"  → model_profile = "quality"
//   Q1 "Standard tier…" + Q2 "Balanced" → model_profile = "balanced"
//   Q1 "Standard tier…" + Q2 "Budget"   → model_profile = "budget"
```

**PR body onboarding:** Ask which optional PRD-style sections `/gsd:ship` should append to generated PR bodies. Use the same `ship.pr_body_sections` mapping as Step 2a: selected sections get `enabled: true`, seeded-but-unselected sections get `enabled: false`, and selecting none writes an empty list. Prefer lean/agile PRD sections that make user value, acceptance criteria, Definition of Done, and stakeholder traceability explicit.

Recommended options:

- `User Stories & Acceptance Criteria`
- `Risks & Dependencies`
- `Success Metrics & Release Criteria`
- `Stakeholder Review & Approval`

Record the settings with all choices (the settings default the rest):

```bash
# no folder to create: the entities live in the spec
# the choices go in as settings, one `specforge settings set @gsd/core <name> <value>` per answer (mode, granularity, parallelization, commit_docs, model_profile, workflow.*, plan_review.source_grounding); the PR body sections stay at their defaults
```

**Note:** Run `/gsd:settings` anytime to update model profile, workflow agents, branching strategy, and other preferences.

**If commit_docs = No:**

- Set `commit_docs: false` in config.json
- Add the spec to `.gitignore` (create if needed)

**If commit_docs = Yes:**

- No additional gitignore entries needed

**Commit config.json:**

```bash
specforge git commit --add spec -m "chore: add project config" && specforge collect --runner gsd-git --yes
```

## 5.1. Sub-Repo Detection

**Detect multi-repo workspace:**

Check for directories with their own `.git` (separate repos within the workspace —
this also finds linked git worktree children, whose `.git` is a file rather than a
directory, unlike a plain directory walk would):

```bash
SUB_REPOS=$(specforge git repos . --pick '.repos[]' --fields name) || exit 1   # each child directory that is a repository or a linked worktree, one per line: a failed listing stops here, and a name holding a newline is refused, never split
```

Each line of `SUB_REPOS` is a bare directory name relative to the workspace root
(e.g. `backend`); none means no sub-repos.

**If sub-repos found:**

Use AskUserQuestion:

- header: "Multi-Repo Workspace"
- question: "I detected separate git repos in this workspace. Which directories contain code that GSD should commit to?"
- multiSelect: true
- options: one option per detected directory
  - "[directory name]" — Separate git repo

**If user selects one or more directories:** each is a repository of its own, so it is a SpecForge project of its own:

- Write nothing here: `planning.sub_repos` is not a setting of the port, and this project's commits never reach a sub-repository
- Suggest running `/gsd:new-project` inside each selected directory, so its own spec tracks its work

The spec stays committed like code: `commit_docs` is not a setting here.

**If no sub-repos found or user selects none:** Continue with no changes to config.

## 5.5. Resolve Model Profile

Use models from init: `researcher_model`, `synthesizer_model`, `roadmapper_model`.

## 6. Research Decision

**If auto mode:** Default to "Research first" without asking.

Use AskUserQuestion:

- header: "Research"
- question: "Research the domain ecosystem before defining requirements?"
- options:
  - "Research first (Recommended)" — Discover standard stacks, expected features, architecture patterns
  - "Skip research" — I know this domain well, go straight to requirements

**If "Research first":**

Display stage banner:

```
### GSD ► RESEARCHING

Researching [domain] ecosystem...
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

**Determine milestone context:**

Check if this is greenfield or subsequent milestone:

- If the project's computed `validated` list is empty (`specforge export --kinds project --format context`) → Greenfield (building from scratch)
- If it lists requirements → Subsequent milestone (adding to existing app)

Display spawning indicator:

```
◆ Spawning 4 researchers in parallel... (each runs in a subagent — no output until they return, ~1–5 min; expected, not a freeze)
  → Stack research
  → Features research
  → Architecture research
  → Pitfalls research
```

Spawn 4 parallel gsd-project-researcher agents — one per dimension (Stack, Features, Architecture, Pitfalls) — each given the domain and greenfield/subsequent milestone context, a dimension-specific question, a downstream-consumer note (what the next stage needs from this file), and a quality gate; each records its dimension's items in `RESEARCH` with its template's fence (~/.claude/gsd-core/templates/research-project/STACK.md, likewise FEATURES.md, ARCHITECTURE.md and PITFALLS.md): stack → `stack_choice`, features → `finding` (topic `features`), architecture → `pattern` and `finding` (topic `architecture`), pitfalls → `pitfall`.

<!-- gsd:protected -->
> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling all 4 researcher Agent() calls above, do NOT read research files or synthesize content independently while the subagents are active. Wait for all 4 researchers to complete before spawning the synthesizer. This prevents duplicate work and wasted context.

**Model omission (#2517) applies to every one of these 5 spawns** (4 researchers + synthesizer): omit the `model=` parameter entirely when the value it would carry (`researcher_model`, `synthesizer_model`) is `"inherit"` or empty — passing it literally 404s on runtimes without native tier aliases (the default on non-Claude runtimes). Omitting `model=` inherits the orchestrator's model.

After all 4 agents complete, spawn the synthesizer to set the research summary:

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

<!-- gsd:protected -->
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

Exact agent prompts (all four researcher dimensions): `gsd-core/workflows/new-project/detail/elaboration.md` § 2.

Display research complete banner and the key findings of the summary:

```
### GSD ► RESEARCH COMPLETE ✓

## Key Findings

**Stack:** [from the summary]
**Table Stakes:** [from the summary]
**Watch Out For:** [from the summary]

Research: RESEARCH (specforge query RESEARCH --depth 1 --format context)
```

**If "Skip research":** Continue to Step 7.

## 7. Define Requirements

Display stage banner:

```
### GSD ► DEFINING REQUIREMENTS
```

**Load context:**

Read the `project` entity (`specforge export --kinds project --format context`) and extract:

- Core value (the ONE thing that must work)
- Stated constraints (budget, timeline, tech limitations)
- Any explicit scope boundaries

**If research exists:** Read research/FEATURES.md and extract feature categories.

**If auto mode:**

- Auto-include all table stakes features (users expect these)
- Include features explicitly mentioned in provided document
- Auto-defer differentiators not mentioned in document
- Skip per-category AskUserQuestion loops
- Skip "Any additions?" question
- Skip requirements approval gate
- Record the requirements and commit directly

**Present features by category (interactive mode only):**

```
Here are the features for [domain]:

## Authentication
**Table stakes:**
- Sign up with email/password
- Email verification
- Password reset
- Session management

**Differentiators:**
- Magic link login
- OAuth (Google, GitHub)
- 2FA

**Research notes:** [any relevant notes]

---

## [Next Category]
...
```

**If no research:** Gather requirements through conversation instead.

Ask: "What are the main things users need to be able to do?"

For each capability mentioned:

- Ask clarifying questions to make it specific
- Probe for related capabilities
- Group into categories

**Scope each category:**

For each category, use AskUserQuestion:

- header: "[Category]" (max 12 chars)
- question: "Which [category] features are in v1?"
- multiSelect: true
- options:
  - "[Feature 1]" — [brief description]
  - "[Feature 2]" — [brief description]
  - "[Feature 3]" — [brief description]
  - "None for v1" — Defer entire category

Track responses:

- Selected features → v1 requirements
- Unselected table stakes → v2 (users expect these)
- Unselected differentiators → out of scope

**Identify gaps:**

Use AskUserQuestion:

- header: "Additions"
- question: "Any requirements research missed? (Features specific to your vision)"
- options:
  - "No, research covered it" — Proceed
  - "Yes, let me add some" — Capture additions

**Validate core value:**

Cross-check requirements against the project's core value. If gaps detected, surface them.

**Record the requirements:**

Run **the requirement fence** in `templates/requirements.md` (`specforge gsd reference templates/requirements.md`) once per requirement, grouped by category: the v1 requirements (tier `v1`), the deferred ones (`v2`) and the explicit exclusions (`out_of_scope`, with their `reason`). There is no traceability to fill: the roadmapper maps each v1 requirement to a phase, and `gsd/E101` binds once the project has a phase.

**REQ-ID format:** `[CATEGORY]-[NUMBER]` (AUTH-01, CONTENT-02)

**Requirement quality criteria:**

Good requirements are:

- **Specific and testable:** "User can reset password via email link" (not "Handle password reset")
- **User-centric:** "User can X" (not "System does Y")
- **Atomic:** One capability per requirement (not "User can login and manage profile")
- **Independent:** Minimal dependencies on other requirements

Reject vague requirements. Push for specificity:

- "Handle authentication" → "User can log in with email/password and stay logged in across sessions"
- "Support sharing" → "User can share post via link that opens in recipient's browser"

**Present full requirements list (interactive mode only):**

Show every requirement (not counts) for user confirmation:

```
## v1 Requirements

### Authentication
- [ ] **AUTH-01**: User can create account with email/password
- [ ] **AUTH-02**: User can log in and stay logged in across sessions
- [ ] **AUTH-03**: User can log out from any page

### Content
- [ ] **CONT-01**: User can create posts with text
- [ ] **CONT-02**: User can edit their own posts

[... full list ...]

---

Does this capture what you're building? (yes / adjust)
```

If "adjust": Return to scoping.

**Commit requirements:**

```bash
specforge git commit --add spec -m "docs: define v1 requirements" && specforge collect --runner gsd-git --yes
```

## 7.5. Project Structure Mode

**If auto mode:** Set `PROJECT_MODE=mvp` and skip this prompt.

**Mode prompt: Vertical MVP vs Horizontal Layers.**

Ask the user how they want to structure the project. Use `AskUserQuestion` with two options:

- **Vertical MVP** — get a working app fast, add features slice by slice. Each phase delivers an end-to-end user capability. *(Recommended for new products and rapid-iteration MVPs.)*
- **Horizontal Layers** — build complete technical layers (DB → API → UI → wiring) and assemble at the end. *(Better for infrastructure-heavy projects with multiple developers.)*

Set `PROJECT_MODE=mvp` if the user picks Vertical MVP, otherwise `PROJECT_MODE=standard`.

When `TEXT_MODE=true` (per the workflow's existing TEXT_MODE handling for non-Claude runtimes), present the same two options as a plain-text numbered list and ask the user to type their choice number.

## 8. Create Roadmap

Display stage banner:

```
### GSD ► CREATING ROADMAP

◆ Spawning roadmapper... (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze)
```

**The phases' mode.** The mode is each phase's `mode` field, which the roadmapper writes: with `PROJECT_MODE=mvp` every initial phase gets `mode mvp` (per-phase mode, not project-wide config); with `PROJECT_MODE=standard` the field is left out (Horizontal Layers, no behavioral change).

Pass `PROJECT_MODE` to the roadmapper so it applies the correct template.

Spawn gsd-roadmapper agent with path references:

```text
Agent(prompt="
<planning_context>

<required_reading>
- specforge context gsd/roadmapper {project} (the roadmapper pack: the project, its decisions, the requirements, the project research and its summary if any, the milestones)
- {config_path} (Granularity and mode settings)
</required_reading>

${AGENT_SKILLS_ROADMAPPER}

</planning_context>

<instructions>
Create roadmap:
1. Derive phases from requirements (don't impose structure)
2. Map every v1 requirement to exactly one phase
3. Derive 2-5 success criteria per phase (observable user behaviors)
4. Validate 100% coverage
5. Write the roadmap edit immediately: the milestone and every phase in one `specforge.edit` call (each phase's `requirements` is the traceability; STATE is computed)
6. Return ROADMAP CREATED with summary

Write the roadmap first, then return. This ensures the entities persist even if context is lost.
</instructions>
", subagent_type="gsd-roadmapper", model="{roadmapper_model}", description="Create roadmap")
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

**Handle roadmapper return:**

**If `## ROADMAP BLOCKED`:**

- Present blocker information
- Work with user to resolve
- Re-spawn when resolved

**If `## ROADMAP CREATED`:**

Read the written roadmap (`specforge export --kinds milestone,phase --format context`) and present it nicely inline:

```
---

## Proposed Roadmap

**[N] phases** | **[X] requirements mapped** | All v1 requirements covered ✓

| # | Phase | Goal | Requirements | Success Criteria |
|---|-------|------|--------------|------------------|
| 1 | [Name] | [Goal] | [REQ-IDs] | [count] |
| 2 | [Name] | [Goal] | [REQ-IDs] | [count] |
| 3 | [Name] | [Goal] | [REQ-IDs] | [count] |
...

### Phase Details

**Phase 1: [Name]**
Goal: [goal]
Requirements: [REQ-IDs]
Success criteria:
1. [criterion]
2. [criterion]
3. [criterion]

**Phase 2: [Name]**
Goal: [goal]
Requirements: [REQ-IDs]
Success criteria:
1. [criterion]
2. [criterion]

[... continue for all phases ...]

---
```

**If auto mode:** Skip approval gate — auto-approve and commit directly.

**CRITICAL: Ask for approval before committing (interactive mode only):**

Use AskUserQuestion:

- header: "Roadmap"
- question: "Does this roadmap structure work for you?"
- options:
  - "Approve" — Commit and continue
  - "Adjust phases" — Tell me what to change
  - "Review full roadmap" — Show the milestone and its phases in full

**If "Approve":** Continue to commit.

**If "Adjust phases":**

- Get user's adjustment notes
- Re-spawn roadmapper with revision context:

  ```text
  Agent(prompt="
  <revision>
  User feedback on roadmap:
  [user's notes]

  <required_reading>
  - specforge context gsd/roadmapper {project} (the roadmapper pack: the current phases among its milestones)
  </required_reading>

  ${AGENT_SKILLS_ROADMAPPER}

  Update the roadmap based on feedback. Change the phases in place, all in one `specforge.edit` call.
  Return ROADMAP REVISED with changes made.
  </revision>
  ", subagent_type="gsd-roadmapper", model="{roadmapper_model}", description="Revise roadmap")
  ```

  > **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more files, edit code, or run tests related to this task while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

- Present revised roadmap
- Loop until user approves

**If "Review full file":** Display the `milestone` and `phase` entities in full (`specforge query "$MILESTONE" --kinds phase --format context`), then re-ask.

**Generate or refresh project instruction file before final commit:**

```bash
# the host install writes the instruction file: nothing to generate here
```

This ensures new projects get the default GSD workflow-enforcement guidance and current project context in `$INSTRUCTION_FILE`.

**Commit roadmap (after approval or auto mode):**

```bash
specforge git commit --add spec -m "docs: create roadmap ([N] phases)" && specforge collect --runner gsd-git --yes
```

## 9. Done

Present completion summary:

```
### GSD ► PROJECT INITIALIZED ✓

**[Project Name]**

| Artifact       | Location                    |
|----------------|-----------------------------|
| Project        | the `project` entity      |
| Config         | the settings (`specforge settings`)     |
| Research       | the `research` entities       |
| Requirements   | the `requirement` entities |
| Roadmap        | the `milestone` and `phase` entities      |
| Project guide  | `$INSTRUCTION_FILE`         |

**[N] phases** | **[X] requirements** | Ready to build ✓
```

**If auto mode:**

```
### AUTO-ADVANCING → DISCUSS PHASE 1
```

Exit skill and invoke SlashCommand("/gsd:discuss-phase 1 --auto")

**If interactive mode:**

Check whether Phase 1 has a frontend (its computed `frontend` field):

```bash
PHASE1_HAS_UI=$(specforge query "$PHASE1" --depth 0 --pick '.nodes[0].fields.frontend' 2>/dev/null)
PHASE1_HAS_UI=${PHASE1_HAS_UI:-false}   # @gsd/ui computes frontend: false when it is not loaded
```

**If Phase 1 has UI (`PHASE1_HAS_UI` is `true`):**

```
---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Phase 1: [Phase Name]** — [Goal of the phase]

/clear then:

/gsd:discuss-phase 1 — gather context and clarify approach

---

**Also available:**
- /gsd:ui-phase 1 — generate UI design contract (recommended for frontend phases)
- /gsd:plan-phase 1 — skip discussion, plan directly

---
```

**If Phase 1 has no UI:**

```
---

## ▶ Next Up — [${PROJECT_CODE}] ${PROJECT_TITLE}

**Phase 1: [Phase Name]** — [Goal of the phase]

/clear then:

/gsd:discuss-phase 1 — gather context and clarify approach

---

**Also available:**
- /gsd:plan-phase 1 — skip discussion, plan directly

---
```

</process>

<output>

- the `project` entity
- the settings (`specforge settings`)
- the `research` entities (if research selected)
  - `STACK.md`
  - `FEATURES.md`
  - `ARCHITECTURE.md`
  - `PITFALLS.md`
  - `SUMMARY.md`
- the `requirement` entities
- the `milestone` and `phase` entities
- the computed project state
- `$INSTRUCTION_FILE` (runtime-derived via the shared `getProjectInstructionFile` policy: `AGENTS.md` for codex/opencode/kilo/kimi, `.github/copilot-instructions.md` for copilot, `GEMINI.md` for antigravity, `.claude/CLAUDE.md` for claude)

</output>

<success_criteria>

- [ ] project entities created in the spec
- [ ] Git repo initialized
- [ ] Brownfield detection completed
- [ ] Deep questioning completed (threads followed, not rushed)
- [ ] The `project` entity and its key decisions capture full context → **committed**
- [ ] config.json has workflow mode, granularity, parallelization → **committed**
- [ ] Research completed (if selected) — 4 parallel agents spawned → **committed**
- [ ] Requirements gathered (from research or conversation)
- [ ] User scoped each category (v1/v2/out of scope)
- [ ] `requirement` entities recorded with REQ-IDs and tiers → **committed**
- [ ] gsd-roadmapper spawned with context
- [ ] Roadmap files written immediately (not draft)
- [ ] User feedback incorporated (if any)
- [ ] Milestone and `phase` entities recorded, each v1 requirement in exactly one phase, with success criteria
- [ ] `$INSTRUCTION_FILE` generated with GSD workflow guidance (runtime-derived via the shared `getProjectInstructionFile` policy — `AGENTS.md` for codex/opencode/kilo/kimi, `.github/copilot-instructions.md` for copilot, `GEMINI.md` for antigravity, `.claude/CLAUDE.md` for claude; an existing hand-crafted file without GSD markers is left untouched unless `--force`)
- [ ] User knows next step is `/gsd:discuss-phase 1`

**Atomic commits:** Each phase commits its artifacts immediately. If context is lost, artifacts persist.

</success_criteria>
