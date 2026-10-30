<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/ingest-docs.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Ingest Docs Workflow

Scan a repo for mixed planning documents (ADR, PRD, SPEC, DOC), synthesize them into a consolidated context, and bootstrap or merge into the spec (its `project`, `requirement`, `milestone` and `phase` entities).

- `--dir <path>` — optional target directory to scan (defaults to repo root)
- `--mode new|merge` — override auto-detect (defaults: `new` if the project has no `project` entity, `merge` if it has one)
- `--manifest <file>` — YAML file listing `{path, type, precedence?}` per doc; overrides heuristic classification
- `--resolve auto|interactive` — conflict resolution (v1: only `auto` is supported; `interactive` is reserved)

---

<step name="banner">

Display the stage banner:

```
### GSD ► INGEST DOCS
```

</step>

<step name="parse_arguments">

Parse `$ARGUMENTS`:

- `--dir <path>` → `SCAN_PATH` (default: `.`)
- `--mode new|merge` → `MODE` (default: auto-detect)
- `--manifest <file>` → `MANIFEST_PATH` (optional)
- `--resolve auto|interactive` → `RESOLVE_MODE` (default: `auto`; reject `interactive` in v1 with message "interactive resolution is planned for a future release")

**Validate paths:**

```bash
case "{SCAN_PATH}" in *..*) echo "SECURITY_ERROR: path contains traversal sequence"; exit 1 ;; esac
test -d "{SCAN_PATH}" || echo "PATH_NOT_FOUND"
if [ -n "{MANIFEST_PATH}" ]; then
  case "{MANIFEST_PATH}" in *..*) echo "SECURITY_ERROR: manifest path contains traversal"; exit 1 ;; esac
  test -f "{MANIFEST_PATH}" || echo "MANIFEST_NOT_FOUND"
fi
```

**Containment (required):** After resolving `SCAN_PATH` and `MANIFEST_PATH` relative to the repo root, canonicalize each with `realpath` (or platform equivalent) and assert the result is under `realpath("$REPO_ROOT")`. Reject absolute paths outside the repo (e.g. `/tmp`, `C:\Windows`) even when they do not contain `..`.

If `PATH_NOT_FOUND` or `MANIFEST_NOT_FOUND`: display error and exit.

</step>

<step name="init_and_mode_detect">

Read the state the run starts from:

```bash
# the commands below are specforge commands: there is no launcher to define
RESPONSE_LANGUAGE="{{setting:response_language|}}"   # response_language (the settings)
INIT=$(specforge export --kinds project,requirement,decision --format brief 2>/dev/null)   # a project is not linked to its requirements, so the listing is by kind
if echo "$INIT" | jq -e '[.nodes[]|select(.kind=="project")]|length > 0' >/dev/null 2>&1; then PROJECT_EXISTS=1; else PROJECT_EXISTS=0; fi   # 1 when the project already exists
: # model resolution: the installed agents carry their model


```

**If `response_language` is set:** All user-facing output of this workflow — narration between tool calls, status updates, progress notes, findings, questions, prompts, and explanations — MUST be presented in `{response_language}`. Technical terms, code, file paths, and subagent prompts stay in English — only user-facing output is translated.

Work out `project_exists` (`PROJECT_EXISTS` above), `has_git`, `git_worktree_root` and `in_nested_subdir` (from `git rev-parse`).

**Absolute paths (#2376):** a spawned subagent's own cwd may differ from the orchestrator's, so the staging directory is absolute: `INTEL_DIR` (made in the classify step, a scratch directory outside the project) is `{intel_dir}` below, and `{conflicts_path}` is `$INTEL_DIR/INGEST-CONFLICTS.md`. The intel files are staging only; nothing in them is part of the spec.

**Auto-detect MODE** if not set:
- `project_exists: true` → `MODE=merge`
- `project_exists: false` → `MODE=new`

If user passed `--mode new` but a project already exists: display warning and require explicit confirm via `AskUserQuestion` (approve-revise-abort from `gsd-core/references/gate-prompts.md`) before overwriting.

Git initialisation (Bug #3491 — never create a nested `.git` inside an existing worktree):

- If `has_git: true` and `in_nested_subdir: true`: do NOT run `git init`. Surface a warning that planning files will be tracked by the outer repo at `git_worktree_root`.
- If `has_git: true` and `in_nested_subdir: false`: already at a worktree root, skip `git init`.
- If `has_git: false` and `MODE=new`: initialize git:

```bash
git init
```

**Detect runtime** using the same pattern as `new-project.md`:
- execution_context path `/.codex/` → `RUNTIME=codex`
- `/.gemini/antigravity/`, `/.gemini/antigravity-ide/` or `/.gemini/antigravity-cli/` → `RUNTIME=antigravity`
- `/.opencode/` or `/.config/opencode/` → `RUNTIME=opencode`
- `/.trae/` → `RUNTIME=trae`
- else → `RUNTIME=claude`

Fall back to env vars (`CODEX_HOME`, `ANTIGRAVITY_CONFIG_DIR`, `OPENCODE_CONFIG_DIR`, `TRAE_CONFIG_DIR`) if execution_context is unavailable.

</step>

<step name="discover_docs">

Build the doc list from three sources, in order:

**1. Manifest (if provided)** — authoritative:

Read `MANIFEST_PATH`. Expected YAML shape:

```yaml
docs:
  - path: docs/adr/0001-db.md
    type: ADR
    precedence: 0   # optional, lower = higher precedence
  - path: docs/prd/auth.md
    type: PRD
```

Each entry provides `path` (required, relative to repo root) + `type` (required, one of ADR|PRD|SPEC|DOC) + `precedence` (optional integer).

**2. Directory conventions** (skipped when manifest is provided):

```bash
# ADRs
find {SCAN_PATH} -type f \( -path '*/adr/*' -o -path '*/adrs/*' -o -name 'ADR-*.md' -o -regex '.*/[0-9]\{4\}-.*\.md' \) 2>/dev/null

# PRDs
find {SCAN_PATH} -type f \( -path '*/prd/*' -o -path '*/prds/*' -o -name 'PRD-*.md' \) 2>/dev/null

# SPECs / RFCs
find {SCAN_PATH} -type f \( -path '*/spec/*' -o -path '*/specs/*' -o -path '*/rfc/*' -o -path '*/rfcs/*' -o -name 'SPEC-*.md' -o -name 'RFC-*.md' \) 2>/dev/null

# Generic docs (fall-through candidates)
find {SCAN_PATH} -type f -path '*/docs/*' -name '*.md' 2>/dev/null
```

De-duplicate the union (a file matched by multiple patterns is one doc).

**3. Content heuristics** (run during classification, not here) — the classifier handles frontmatter `type:` and H1 inspection for docs that didn't match a convention.

**Cap:** hard limit of 50 docs per invocation (documented v1 constraint). If the discovered set exceeds 50:

```
GSD > Discovered {N} docs, which exceeds the v1 cap of 50.
      Use --manifest to narrow the set to ≤ 50 files, or run
      /gsd:ingest-docs again with a narrower --dir.
```

Exit without proceeding.

**Display discovered set** and request approval (see `gsd-core/references/gate-prompts.md` — `yes-no-pick` pattern works; or `approve-revise-abort`):

```
Discovered {N} documents:
  {N} ADR | {N} PRD | {N} SPEC | {N} DOC | {N} unclassified

  docs/adr/0001-architecture.md       [ADR]    (from manifest|directory|heuristic)
  docs/adr/0002-database.md           [ADR]    (directory)
  docs/prd/auth.md                    [PRD]    (manifest)
  ...
```

**Text mode:** apply the same `--text`/`text_mode` rule as other workflows — replace `AskUserQuestion` with a numbered list.

Use `AskUserQuestion` (approve-revise-abort):
- question: "Proceed with classification of these {N} documents?"
- header: "Approve?"
- options: Approve | Revise | Abort

On Abort: exit cleanly with "Ingest cancelled."
On Revise: exit with guidance to re-run with `--manifest` or a narrower path.

</step>

<step name="classify_parallel">

Create staging directory:

```bash
INTEL_DIR=$(mktemp -d "${TMPDIR:-/tmp}/gsd-ingest-docs.XXXXXX") && mkdir -p "$INTEL_DIR/classifications/"   # scratch files, outside the project
```

For each discovered doc, spawn `gsd-doc-classifier` in parallel. In Claude Code, issue all Task calls in a single message with multiple tool uses so the harness runs them concurrently. For Copilot / sequential runtimes, fall back to sequential dispatch.

<!-- #2517 model-omit-on-inherit -->

> **Model omission (#2517).** Omit the `model` parameter entirely when the value it would carry (`CLASSIFIER_MODEL`, `SYNTHESIZER_MODEL`, `ROADMAPPER_MODEL`) is `"inherit"` or empty. An empty value 404s on runtimes without native tier aliases — the default on non-Claude runtimes. Omitting it inherits the orchestrator's model. See @gsd-core/references/model-profile-resolution.md.

Per-spawn prompt fields:
- `FILEPATH` — absolute path to the doc
- `OUTPUT_DIR` — `{intel_dir}/classifications` (absolute — `INTEL_DIR` above; #2376: a spawned classifier's own cwd may differ from the orchestrator's)
- `MANIFEST_TYPE` — the type from the manifest if present, else omit
- `MANIFEST_PRECEDENCE` — the precedence integer from the manifest if present, else omit
- `<required_reading>` — `agents/gsd-doc-classifier.md` (the agent definition itself)

**Model on every classifier spawn (#3602):** `model="{CLASSIFIER_MODEL}"` is a parameter of each Task/Agent call — not a prompt field, never folded into the prompt text — so `dynamic_routing`/`model_profile` tiers apply instead of the caller's session model. Omit the parameter per the rule above when the value is `"inherit"` or empty.

Collect the one-line confirmations from each classifier. If any classifier errors out, surface the error and abort without writing any entity.

</step>

<step name="synthesize">

Spawn `gsd-doc-synthesizer` once (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze):

<!-- #2508 runtime-aware-dispatch -->

> **Runtime-aware dispatch (#2508 Phase 4).** GSD workflows dispatch specialized subagents by role. Before dispatching on a built-in-only runtime (kimi-code — three built-ins only), resolve the role to a built-in via the dispatch table (not part of @gsd/core: use the role unchanged). On named-dispatch runtimes (Claude/OpenCode/…) the role is returned unchanged; on kimi-code it maps to `coder`/`explore`/`plan` by role-suffix. The persona rides `${AGENT_SKILLS_<ROLE>}` (Phase 3) regardless. See @gsd-core/references/runtime-aware-dispatch.md.

```
Agent({
  subagent_type: "gsd-doc-synthesizer",
  model: "{SYNTHESIZER_MODEL}",
  prompt: "
    CLASSIFICATIONS_DIR: {intel_dir}/classifications
    INTEL_DIR: {intel_dir}
    CONFLICTS_PATH: {conflicts_path}
    MODE: {MODE}
    EXISTING_CONTEXT: {the existing `project`, `requirement`, `milestone` and `phase` entities (`specforge query`) if MODE=merge, else empty}
    PRECEDENCE: {array from manifest defaults or default ['ADR','SPEC','PRD','DOC']}

    <required_reading>
    - agents/gsd-doc-synthesizer.md
    - gsd-core/references/doc-conflict-engine.md
    </required_reading>
  "
})
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read or synthesize any classified documents independently while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

The synthesizer writes its staging files outside the spec:
- `{intel_dir}/decisions.md`, `{intel_dir}/requirements.md`, `{intel_dir}/constraints.md`, `{intel_dir}/context.md`
- `{intel_dir}/SYNTHESIS.md`
- `{conflicts_path}` (`INGEST-CONFLICTS.md`, which is reported to the user)

</step>

<step name="conflict_gate">

Read `{conflicts_path}`. Count entries in each bucket (the synthesizer always writes the three-bucket header; parse the `### BLOCKERS ({N})`, `### WARNINGS ({N})`, `### INFO ({N})` lines). A missing, empty or unparsable conflicts file is a BLOCKER: the synthesizer failed to report, so nothing says the documents agree; treat it as `BLOCKERS > 0` (re-run the synthesizer or stop) and never read it as "no conflicts".

Apply the safety semantics from `gsd-core/references/doc-conflict-engine.md`. Operation noun: `ingest`.

**If BLOCKERS > 0:**

Render the report to the user, then display:

```
GSD > BLOCKED: {N} blockers must be resolved before ingest can proceed.
```

Exit WITHOUT writing the `project`, `requirement`, `milestone` or `phase` entities. The staging intel files remain for inspection. The safety gate holds — no destination entity is written when blockers exist.

**If WARNINGS > 0 and BLOCKERS = 0:**

Render the report, then ask via AskUserQuestion (approve-revise-abort):
- question: "Review the competing variants above. Resolve manually and proceed, or abort?"
- header: "Approve?"
- options: Approve | Abort

On Abort: exit cleanly with "Ingest cancelled. Staged intel preserved at `{intel_dir}/`."

**If BLOCKERS = 0 and WARNINGS = 0:**

Optionally display `GSD > No conflicts. Auto-resolved: {N}.` Absence of conflicts is not authorization to write: proceed to the routing gate for the active mode — new mode's routing gate (next step) or merge mode's merge-diff approve-revise-abort gate — which decides whether destination files are created.

</step>

<step name="route_new_mode">

**Applies only when MODE=new.**

Audit `project` fields that `gsd-roadmapper` expects. For fields derivable from `{intel_dir}/SYNTHESIS.md` (project scope, goals/non-goals, constraints, locked decisions), synthesize from the intel. For fields NOT derivable (project name, developer-facing success metric, target runtime), prompt via `AskUserQuestion` one at a time — minimal question set, no interrogation.

**Routing gate (#3827): approval to classify documents is not approval to write the planning scaffold.** Before delegating, display the exact destinations and require an explicit choice:

```
Routing — create the planning setup now?

  the project entity                (new)
  the requirement entities          (new)
  the milestone and phase entities  (new)
  (the project state is computed: nothing else is written)
```

Use `AskUserQuestion`:
- question: "Routing — create the planning setup now?"
- header: "Routing"
- options: Create planning setup | Keep synthesized intel only | Abort

**Text mode:** numbered list (1/2/3) with the same three choices.

On **Create planning setup**: continue to the `gsd-roadmapper` delegation below.

On **Keep synthesized intel only**: analysis-only ingest. Do NOT invoke `gsd-roadmapper`; write no entity. The staged intel under `{intel_dir}/` is preserved. Display the completion banner with mode `new (intel only)`.

On Abort: exit cleanly with "Ingest cancelled. Staged intel preserved at `{intel_dir}/`."

Any other response (freeform/"Other"): re-ask once; if still ambiguous, treat as Abort. Never infer Create from an ambiguous answer.

Delegate to `gsd-roadmapper` (runs in a subagent — no output until it returns, ~1–5 min; expected, not a freeze):

```
Agent({
  subagent_type: "gsd-roadmapper",
  model: "{ROADMAPPER_MODEL}",
  prompt: "
    Mode: new-project-from-ingest
    Intel: {intel_dir}/SYNTHESIS.md (entry point)
    Per-type intel: {intel_dir}/decisions.md, {intel_dir}/requirements.md, {intel_dir}/constraints.md, {intel_dir}/context.md
    User-supplied fields: {collected in previous step}

    Produce:
    - the `project` entity
    - the `requirement` entities
    - the `milestone` and `phase` entities (the roadmap)
    - (nothing for the project state: it is computed)

    Treat ADR-locked decisions as locked in the `project` entity's decisions.
  "
})
```

> **ORCHESTRATOR RULE — CODEX RUNTIME**: After calling Agent() above, stop working on this task immediately. Do not read more intel files, write planning artifacts, or create ROADMAP.md independently while the subagent is active. Wait for the subagent to return its result. This prevents duplicate work, conflicting edits, and wasted context. Only resume when the subagent result is available.

</step>

<step name="route_merge_mode">

**Applies only when MODE=merge.**

Load the existing roadmap (`specforge export --kinds milestone,phase,context --format context`: the milestones, the phases and the `context` entities of the phases), the `project` entity (`specforge query "$PROJECT" --depth 0 --format context`) and the `requirement` entities (`specforge export --kinds requirement --format context`). A project is not linked to its milestones or requirements, so a query from it would read only the project.

The synthesizer has already hard-blocked on any LOCKED-in-ingest vs LOCKED-in-existing contradiction; if we reach this step, no such blockers remain.

Plan the merge:
- **New requirements** from synthesized `{intel_dir}/requirements.md` that do not overlap existing requirement entities → add them as `requirement` entities
- **New decisions** from synthesized `{intel_dir}/decisions.md` that do not overlap existing `context` entities' `<decisions>` blocks → write to a new phase's `context` entity or append to the next milestone's requirements
- **New scope** → derive phase additions following the `new-milestone.md` pattern; add the phases to the roadmap (`specforge new phase`)

Preview the merge diff to the user and gate via approve-revise-abort before writing.

</step>

<step name="finalize">

The entities are written through `specforge new` and `specforge set`, so there is nothing to commit here; the staged intel and the conflict report are not committed:

```bash
: # no commit: the entities are written by the roadmapper, the staged intel is not committed







```

(The same in merge mode.)

Display completion:

```
### GSD ► INGEST DOCS COMPLETE
```

Show:
- Mode ran (new, new (intel only), or merge)
- Docs ingested (count + type breakdown)
- Decisions locked, requirements created, constraints captured
- Conflict report (`{conflicts_path}`, shown above)
- Next step: `/gsd:plan-phase 1` (new) or `/gsd:plan-phase N` (merge, pointing at the first newly-added phase); for intel-only runs there is no roadmap yet — point at `/gsd:new-project` (or a later re-run with `--mode merge` once scaffold files exist), never `/gsd:plan-phase`

</step>

---

## Anti-Patterns

Do NOT:
- Violate the shared conflict-engine contract in `gsd-core/references/doc-conflict-engine.md` (no markdown tables, no new severity labels, no bypass of the BLOCKER gate)
- Write the `project`, `requirement`, `milestone` or `phase` entities when BLOCKERs exist in the conflict report
- Skip the 50-doc cap — larger sets must use `--manifest` to narrow the scope
- Auto-resolve LOCKED-vs-LOCKED ADR contradictions — those are BLOCKERs in both modes
- Merge competing PRD acceptance variants into a combined criterion — preserve all variants for user resolution
- Bypass the discovery approval gate — users must see the classified doc list before classifiers spawn
- Skip path validation on `SCAN_PATH` or `MANIFEST_PATH`
- Implement `--resolve interactive` in this v1 — the flag is reserved; reject with a future-release message
