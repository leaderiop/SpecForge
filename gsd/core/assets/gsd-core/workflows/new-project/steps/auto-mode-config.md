<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/new-project/steps/auto-mode-config.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
## 2a. Auto Mode Config (auto mode only)

**If auto mode:** Collect config settings upfront before processing the idea document.

YOLO mode is implicit (auto = YOLO). Ask remaining config questions:

**Round 1 — Core settings (3 questions, no Mode question):**

```
AskUserQuestion([
  {
    header: "Granularity",
    question: "How finely should scope be sliced into phases?",
    multiSelect: false,
    options: [
      { label: "Coarse (Recommended)", description: "Fewer, broader phases (3-5 phases, 1-3 plans each)" },
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
])
```

**Round 2 — Workflow agents (same as Step 5):**

```
AskUserQuestion([
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
    header: "Drift Guard",
    question: "Enable the plan drift-guard? It verifies that symbols your plans cite (decorators, classes, functions, CLI flags) actually exist in your source at review time, catching hallucinated names before execution. [Y/n]",
    multiSelect: false,
    options: [
      { label: "Yes (Recommended)", description: "Resolve symbol references against live source during plan review — catches hallucinated names before execution" },
      { label: "No", description: "Skip symbol grounding — plan review proceeds without source verification" }
    ]
  }
])

// Model profile uses a two-question split because AskUserQuestion enforces a hard
// 4-option cap and there are 5 valid profiles (quality, balanced, budget, adaptive,
// inherit). Q1 routes between adaptive/standard-tier/inherit; Q2 (shown only when
// Q1 = "Standard tier…") picks among the three standard profiles. Mirrors the
// /gsd:settings split (#3784, #1516).
AskUserQuestion([
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
])

**Conditional visibility — model_profile (Q2):**
  Only ask this question when Q1's answer is "Standard tier…".
  If Q1 = "Adaptive (Recommended)" → write model_profile=adaptive and SKIP Q2.
  If Q1 = "Inherit"                → write model_profile=inherit and SKIP Q2.
  If user cancels Q2 after picking "Standard tier…" → leave existing model_profile value unchanged.

AskUserQuestion([
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
])

// Map UI choices → config values:
//   Q1 "Adaptive (Recommended)"         → model_profile = "adaptive"
//   Q1 "Inherit"                        → model_profile = "inherit"
//   Q1 "Standard tier…" + Q2 "Quality"  → model_profile = "quality"
//   Q1 "Standard tier…" + Q2 "Balanced" → model_profile = "balanced"
//   Q1 "Standard tier…" + Q2 "Budget"   → model_profile = "budget"
```

**Round 3 — PR body onboarding:**

Ask which optional PRD-style sections `/gsd:ship` should append to generated PR bodies. These map to `ship.pr_body_sections`; selected sections are written with `"enabled": true`, unselected seeded sections are written with `"enabled": false` so the project can enable them later without editing `ship.md`.

Prefer lean/agile PRD sections that make the delivered increment clear: user stories, acceptance criteria, Definition of Done or release criteria, risks, dependencies, and stakeholder review.

```
AskUserQuestion([
  {
    header: "PR Body",
    question: "Which optional PRD-style sections should /gsd:ship include in PR bodies?",
    multiSelect: true,
    options: [
      { label: "User Stories & Acceptance Criteria", description: "Append user-facing stories and acceptance checks from REQUIREMENTS.md" },
      { label: "Risks & Dependencies", description: "Append rollout risks, dependencies, and rollback notes from PLAN.md" },
      { label: "Success Metrics & Release Criteria", description: "Append measurable Definition of Done and release checks for stakeholder review" },
      { label: "Stakeholder Review & Approval", description: "Append approval checklist for projects that need sign-off traceability" }
    ]
  }
])
```

Build `ship.pr_body_sections` from those choices. For selected options, set `enabled: true`; for seeded but unselected options, set `enabled: false`. If the user selects none, use `"ship":{"pr_body_sections":[]}`.

Record the settings with all choices (the settings default the rest):

```bash
# the commands below are specforge commands: there is no launcher to define
# no folder to create: the entities live in the spec
# the choices go in as settings, one `specforge settings set @gsd/core <name> <value>` per answer (mode yolo, granularity, parallelization, commit_docs, model_profile, workflow.*, plan_review.source_grounding); the PR body sections stay at their defaults
```

**If commit_docs = No:** Add the spec to `.gitignore`.

**Commit config.json:**

```bash
# no folder to create: the entities live in the spec
git add spec && git commit -m "chore: add project config" && specforge collect --runner gsd-git --yes
```

**Persist auto-advance chain flag to config (survives context compaction):**

```bash
: # workflow._auto_chain_active is not a setting here: the chain lasts as long as the conversation, so there is nothing to set or clear
```

Proceed to Step 4 (skip Steps 3 and 5).
