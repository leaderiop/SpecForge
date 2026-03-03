---
name: spec-review
description: "Validate specification documents for structural completeness, cross-reference integrity, and orphan detection using the 62-rule catalog from ADR-026 (VAL-001 through VAL-062). Use when auditing a spec directory for structural validity, checking cross-reference integrity between behaviors/features/capabilities/invariants/decisions/deliverables/libraries, detecting orphan entities not referenced by any upstream entity, verifying index.yaml files are in sync with their directories, checking overview.md completeness, validating markdown link integrity, or assessing a spec's readiness for tier promotion. This skill reviews but does NOT auto-fix — it reports findings and suggests which authoring skill to invoke for remediation."
---

# Spec Review (Structural Validation)

Validates specification documents against the **62-rule structural validation catalog** defined in [ADR-026](../../spec/specforge/decisions/ADR-026-spec-structural-validation.md) and cataloged in [process/spec-validation-rules.md](../../spec/specforge/process/spec-validation-rules.md).

This skill **reviews only** — it does not auto-fix. Each finding cites a VAL-NNN rule and recommends which authoring skill to invoke for remediation.

## When to Use

- Auditing a spec directory for structural validity before merge
- Checking cross-reference integrity (forward and reverse)
- Detecting orphan entities (behaviors, features, invariants, ADRs not referenced upstream)
- Verifying index.yaml ↔ filesystem synchronization
- Validating overview.md completeness
- Assessing a spec's readiness for tier promotion
- After bulk spec authoring to catch structural drift

## When NOT to Use

- For GxP regulatory compliance review → use **gxp-spec-review**
- For writing new spec documents → use **spec-authoring** and its delegates
- For code review → this skill reviews spec `.md` files only

---

## Script-Accelerated Review

The skill ships with bash validation scripts that automate the mechanical scanning for all 62 VAL rules. **Run scripts first**, then focus manual effort on semantic judgment.

### Usage

```bash
bash .claude/skills/spec-review/scripts/validate-all.sh <spec-dir>

# Options
bash .claude/skills/spec-review/scripts/validate-all.sh <spec-dir> --strict       # Warnings = failure
bash .claude/skills/spec-review/scripts/validate-all.sh <spec-dir> --only=01,03   # Run specific scripts
bash .claude/skills/spec-review/scripts/validate-all.sh <spec-dir> --format=jsonl  # JSONL output
```

### Script-to-Phase Mapping

| Script | Phase | Rules | What It Checks |
|--------|-------|-------|---------------|
| `01-id-integrity.sh` | 2 | VAL-001–009 | Duplicate IDs, id_range integrity, filename-prefix matching |
| `02-frontmatter-schema.sh` | 3 | VAL-010–017 | Frontmatter presence, required fields, kind/status validity |
| `03-forward-references.sh` | 4 | VAL-018–024 | Cross-references in frontmatter point to existing targets |
| `04-reverse-coverage.sh` | 5 | VAL-025–030 | Every entity referenced by at least one upstream entity |
| `05-index-completeness.sh` | 6 | VAL-031–037 | Index.yaml ↔ filesystem synchronization |
| `06-overview-completeness.sh` | 6 | VAL-038–042 | Overview.md references all entity files |
| `07-link-integrity.sh` | 7 | VAL-043–045 | Markdown links resolve, BEH/ADR link format |
| `08-content-structure.sh` | 7 | VAL-046–048 | BEH sections have Contract + Verification subsections |
| `09-traceability-matrix.sh` | 8 | VAL-049–050 | INV/ADR IDs in traceability files |
| `10-semantic-consistency.sh` | 8 | VAL-051–053 | Supersession chains, deprecated hints, port docs |

### Workflow

1. **Run `validate-all.sh`** as the FIRST step — this produces machine-readable TSV findings for all 62 rules
2. **Populate the findings report** from script output — each TSV line maps to an F-NNN finding
3. **Manual review only for:** (a) rules the script reports as `skip`, (b) semantic judgment nuances the script cannot detect, (c) positive observations about spec quality
4. **Phase 1 (Scope & Tier Detection)** is always manual — scripts auto-detect infix but Claude should confirm tier classification and legacy format notes

### Output Format

Scripts emit TSV to stdout (`RULE\tSEVERITY\tFILE\tMESSAGE`) and a summary table to stderr. No external dependencies — pure bash + awk + grep + sed.

---

## Severity Levels (from ADR-026)

| Level | Meaning | Count |
|-------|---------|-------|
| `error` | Structural break — the spec is inconsistent | 28 rules |
| `warning` | Coverage gap — the spec is incomplete but not broken | 28 rules |
| `info` | Suggestion — the spec could be improved | 6 rules |

---

## 8-Phase Sequential Review Flow

Execute ALL 8 phases in order. Never skip a phase. Each phase reads files and builds state used by later phases.

### Phase 1: Scope & Tier Detection

**Purpose:** Determine the spec's completeness tier, infix, and expected components.

**Steps:**

1. Read the spec root directory listing (top-level files and subdirectories)
2. Detect the **infix** from any existing `behaviors/index.yaml` (`infix` field) or from behavior file naming (`BEH-XX-NNN` where `XX` is the infix)
3. If no infix is detectable, derive from the spec path: `spec/specforge/` → `SF`, `spec/libs/guard/` → `GD`, `spec/libs/flow/` → `FL`
4. Classify the spec into its **completeness tier**:

| Tier | Required Components |
|------|-------------------|
| Stub | `overview.md` only |
| Technical-only | `overview.md` + `behaviors/` |
| Technical + behaviors | Above + `decisions/`, `invariants/`, `glossary.md` |
| Full governance | Above + `traceability/`, `risk-assessment/`, `roadmap/`, `process/`, `scripts/` |
| Full + architecture | Above + `architecture/`, `types/`, `type-system/` |
| Full + optional | Above + any of `product/`, `research/`, `plugins/`, `references/`, `compliance/`, `visual/`, `features/`, `capabilities/`, `deliverables/`, `libraries/` |

5. List which directories/files exist and which are missing for the next tier
6. Count total `.md` files and `index.yaml` files
7. Detect **legacy format** — if the spec uses numbered chapters (e.g., `01-overview.md`, `02-behaviors.md`) instead of the canonical directory structure, note this as info

**Output:** Spec path, detected tier, infix, file counts, legacy format flag.

**Automation:** Phase 1 is always manual. Scripts auto-detect infix via `lib.sh:detect_infix()` but Claude should confirm tier classification.

---

### Phase 2: ID Integrity (VAL-001 through VAL-009)

**Purpose:** Ensure all entity IDs are unique, non-overlapping, and correctly allocated.

**Rules:**

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-001 | error | No duplicate `BEH-{infix}` IDs across all behavior files. Scan every `## BEH-{infix}-NNN:` header in `behaviors/` AND `plugins/` files. Include gap-fill behaviors (IDs outside the file's primary range). |
| VAL-002 | error | No duplicate `FEAT-{infix}` IDs across all feature files. Check `id` frontmatter in `features/`. |
| VAL-003 | error | No duplicate `UX-{infix}` IDs across all capability files. Check `id` frontmatter in `capabilities/`. |
| VAL-004 | error | No duplicate `INV-{infix}` IDs across all invariant files. Check IDs in `invariants/`. |
| VAL-005 | error | No duplicate `ADR` IDs across all decision files. Check `id` frontmatter in `decisions/`. |
| VAL-006 | error | No duplicate `TYPE-{infix}` IDs (if present) across type files. Check `id` frontmatter in `types/`. |
| VAL-007 | error | `BEH-{infix}` section IDs (`## BEH-{infix}-NNN:`) fall within the file's declared `id_range`. Exception: gap-fill behaviors are explicitly allocated outside the primary range (documented in `process/requirement-id-scheme.md`). |
| VAL-008 | error | `id_range` values don't overlap between behavior files. No two behavior files may claim the same ID range. |
| VAL-009 | warning | Filename ID prefix matches frontmatter `id` field. E.g., `BEH-SF-001-graph-operations.md` must have `id: BEH-SF-001` in frontmatter. |

**Parsing notes:**

- BEH IDs are extracted from `## BEH-{infix}-NNN:` markdown headers, NOT from frontmatter `id` (which identifies the file, not individual behaviors)
- Gap-fill behaviors (e.g., IDs 300-396 in specforge) are placed in existing files outside their primary range. Cross-reference the allocation table in `process/requirement-id-scheme.md` for VAL-007 exceptions
- Plugin behaviors (`plugins/PLG-*.md`) define BEH IDs outside `behaviors/` — include them in duplicate detection
- `id_range` format is `"NNN--NNN"` (double-dash separator), parse both bounds as integers

**How to scan for BEH IDs:**

```
Pattern: /^## BEH-{infix}-(\d+):/m
```

Scan all files in `behaviors/` and `plugins/` directories. Build a `Map<string, string[]>` of ID → [file paths] to detect duplicates.

**Automation:** `01-id-integrity.sh` covers all VAL-001–009 rules including gap-fill exception loading from `process/requirement-id-scheme.md`.

---

### Phase 3: Frontmatter Schema (VAL-010 through VAL-017, VAL-056..057)

**Purpose:** Validate YAML frontmatter presence and correctness on all spec files.

**Rules:**

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-010 | error | All spec files have valid YAML frontmatter delimited by `---` fences. **Skip directories:** `visual/`, `references/`, `scripts/`, `research/`, `product/`. Also skip `index.md` files and `index.yaml` files. |
| VAL-011 | error | Required frontmatter fields present: `id`, `kind`, `title`, `status`. Exception: singleton files like `overview.md` and `glossary.md` may omit `id`. |
| VAL-012 | error | `kind` value matches parent directory: `behaviors/` → `behavior`, `decisions/` → `decision`, `features/` → `feature`, `capabilities/` → `capability`, `types/` → `type` or `types`, `invariants/` → `invariant`, `process/` → `process`, `plugins/` → `plugin`, `deliverables/` → `deliverable`, `libraries/` → `library`. |
| VAL-013 | warning | `status` is valid for its kind: behaviors use `active`/`deprecated`; decisions use `Accepted`/`Superseded`/`Draft`; features use `active`/`planned`; capabilities use `active`/`planned`; deliverables use `active`/`planned`; libraries use `active`/`planned`. |
| VAL-014 | warning | Behavior files have recommended fields: `id_range`, `invariants`, `adrs`, `types`, `ports`. |
| VAL-015 | warning | Feature files have recommended fields: `behaviors`, `adrs`, `roadmap_phases`. |
| VAL-016 | warning | Capability files have recommended fields: `features`, `behaviors`, `persona`, `surface`. |
| VAL-017 | warning | Decision files have recommended fields: `date`, `supersedes`. |
| VAL-056 | error | Deliverable files have valid frontmatter: `kind: deliverable`, required fields `id`, `title`, `status`, `deliverable_type`, `capabilities`, `depends_on`. `deliverable_type` must be one of `app`, `service`, `extension`, `cli`. |
| VAL-057 | error | Library files have valid frontmatter: `kind: library`, required fields `id`, `title`, `status`, `npm_name`, `path`, `library_type`, `family`, `features`, `depends_on`. `library_type` must be one of `core`, `feature`, `adapter`, `integration`, `testing`, `tooling`. |

**Parsing approach:**

1. Read each `.md` file
2. Extract content between first `---` and second `---`
3. Parse as YAML
4. If parsing fails → report VAL-010 with the YAML parse error message
5. If no `---` fences found → report VAL-010
6. Validate fields against rules

**Automation:** `02-frontmatter-schema.sh` covers all VAL-010–017 and VAL-056–057 rules. Skips `visual/`, `references/`, `scripts/`, `research/`, `product/` dirs and legacy numbered-chapter files automatically.

---

### Phase 4: Forward Reference Integrity (VAL-018 through VAL-024, VAL-058..061)

**Purpose:** Every cross-reference in frontmatter must point to an existing target.

**Rules:**

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-018 | error | `invariants: [INV-{infix}-N]` in behavior frontmatter references an existing invariant. The ID must exist in `invariants/`. |
| VAL-019 | error | `adrs: [ADR-NNN]` in behavior/feature/type frontmatter references an existing ADR in `decisions/`. |
| VAL-020 | error | `behaviors: [BEH-{infix}-NNN]` in feature/capability frontmatter references an existing BEH definition. **Resolution:** find a behavior file whose `id_range` covers the ID, OR a `## BEH-{infix}-NNN:` header in any behavior/plugin file. |
| VAL-021 | error | `features: [FEAT-{infix}-NNN]` in capability frontmatter references an existing feature in `features/`. |
| VAL-022 | warning | `types: [domain]` in behavior frontmatter references an existing type file (e.g., `types: [graph]` → `types/graph.md`). |
| VAL-023 | error | `supersedes: [ADR-NNN]` in decision frontmatter references an existing ADR in `decisions/`. |
| VAL-024 | warning | `roadmap_phases: [RM-NN]` in feature frontmatter references an existing roadmap file or entry in `roadmap/`. |
| VAL-058 | error | `capabilities: [UX-{infix}-NNN]` in deliverable frontmatter references an existing capability in `capabilities/`. |
| VAL-059 | error | `depends_on: [LIB-{infix}-NNN]` in deliverable frontmatter references an existing library in `libraries/`. |
| VAL-060 | error | `features: [FEAT-{infix}-NNN]` in library frontmatter references an existing feature in `features/`. |
| VAL-061 | error | `depends_on: [LIB-{infix}-NNN]` in library frontmatter references an existing library in `libraries/`. |

**The Cross-Reference Graph:**

```
Source Entity            Frontmatter Field    Target Entity     Rule
─────────────────────────────────────────────────────────────────────
Behavior                 invariants[]     →   Invariant         VAL-018
Behavior/Feature/Type    adrs[]           →   Decision          VAL-019
Feature/Capability       behaviors[]      →   Behavior          VAL-020
Capability               features[]       →   Feature           VAL-021
Behavior                 types[]          →   Type file         VAL-022
Decision                 supersedes[]     →   Decision          VAL-023
Feature                  roadmap_phases[] →   Roadmap           VAL-024
Deliverable              capabilities[]   →   Capability        VAL-058
Deliverable              depends_on[]     →   Library           VAL-059
Library                  features[]       →   Feature           VAL-060
Library                  depends_on[]     →   Library           VAL-061
```

**BEH ID resolution (critical):**

Features reference `BEH-SF-002` but the file is `BEH-SF-001-graph-operations.md` with `id_range: 001--008`. To resolve:

1. Check if any behavior file's `id_range` covers the ID number
2. Check if any file contains a `## BEH-{infix}-NNN:` header matching the ID
3. Check plugin files (`plugins/PLG-*.md`) for the header pattern

**INV ID format note:** INV-SF uses no zero-padding (`INV-SF-1` not `INV-SF-001`).

**ADR resolution:** `adrs: [ADR-005]` resolves to any file in `decisions/` matching `ADR-005-*.md`.

**Automation:** `03-forward-references.sh` covers all VAL-018–024 and VAL-058–061 rules. Builds entity indexes and resolves BEH IDs via both `id_range` coverage and `## BEH-XX-NNN:` header matching.

---

### Phase 5: Reverse Coverage / Orphan Detection (VAL-025 through VAL-030, VAL-054..055)

**Purpose:** Every entity must be referenced by at least one upstream entity (full traceability).

**Rules:**

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-025 | warning | Every `BEH-{infix}` ID (from `## BEH-{infix}-NNN:` headers) is referenced by at least one feature's `behaviors` list. |
| VAL-026 | warning | Every `FEAT-{infix}` is referenced by at least one capability's `features` list. |
| VAL-027 | warning | Every `INV-{infix}` is referenced by at least one behavior's `invariants` list. |
| VAL-028 | warning | Every ADR is referenced by at least one behavior's `adrs` list. |
| VAL-029 | info | Every type domain is referenced by at least one behavior's `types` list. |
| VAL-030 | warning | Every behavior file references at least one invariant OR ADR in its frontmatter (traceability anchor). A file with empty `invariants` and empty `adrs` has no traceability. |
| VAL-054 | warning | Every `UX-{infix}` capability is referenced by at least one deliverable's `capabilities[]` list. |
| VAL-055 | warning | Every `FEAT-{infix}` feature is implemented by at least one library's `features[]` list. |

**Reverse graph (checked here):**

```
Target Entity    Must Be Referenced By         Rule
──────────────────────────────────────────────────────
Behavior     ←   Feature.behaviors[]           VAL-025
Feature      ←   Capability.features[]         VAL-026
Invariant    ←   Behavior.invariants[]         VAL-027
ADR          ←   Behavior.adrs[]               VAL-028
Type domain  ←   Behavior.types[]              VAL-029
(any BEH file)   must have invariants OR adrs  VAL-030
Capability   ←   Deliverable.capabilities[]    VAL-054
Feature      ←   Library.features[]            VAL-055
```

**Note:** If a spec has no `features/` or `capabilities/` directory, VAL-025 and VAL-026 are skipped with an info note — the spec may not be at a tier that requires those components.

**Automation:** `04-reverse-coverage.sh` covers all VAL-025–030 and VAL-054–055 rules. Builds entity ID sets and upstream reference maps, then detects orphans automatically.

---

### Phase 6: Index & Overview Completeness (VAL-031 through VAL-042, VAL-062)

**Purpose:** Index.yaml files and overview.md must be in sync with the filesystem.

#### Index File Rules (VAL-031 through VAL-037)

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-031 | error | Every `.md` file in `behaviors/` (excluding `index.md`) is listed in `behaviors/index.yaml` `entries[]`. |
| VAL-032 | error | Every `.md` file in `decisions/` is listed in `decisions/index.yaml` `entries[]`. |
| VAL-033 | error | Every `.md` file in `types/` is listed in `types/index.yaml` `entries[]`. |
| VAL-034 | warning | Every `.md` file in `features/` is listed in `features/index.yaml` `features[]`. |
| VAL-035 | warning | Every `.md` file in `capabilities/` is listed in `capabilities/index.yaml` `groups[].capabilities[]`. |
| VAL-036 | warning | Every `.md` file in `invariants/` is listed in `invariants/index.yaml` `entries[]`. |
| VAL-037 | error | Every index.yaml entry's `file` field points to an existing `.md` file. No stale entries pointing to deleted files. |
| VAL-062 | warning | Every `.md` file in `deliverables/` and `libraries/` (including sub-folders) is listed in their respective `index.yaml`. |

**Four index.yaml formats (CRITICAL — must handle all four):**

1. **Standard format** (`behaviors/`, `decisions/`, `invariants/`, `types/`, `process/`, `deliverables/`):
   ```yaml
   entries:
     - id: "BEH-SF-001"
       file: "BEH-SF-001-graph-operations.md"
       title: "Graph Operations"
   ```
   File references are in `entries[].file`.

2. **Features format** (`features/`):
   ```yaml
   features:
     - id: FEAT-SF-001
       file: FEAT-SF-001-graph-store.md
       title: "Graph-First Knowledge Store"
   ```
   File references are in `features[].file`.

3. **Capabilities format** (`capabilities/`):
   ```yaml
   groups:
     - name: Flow Operations
       capabilities:
         - id: UX-SF-001
           file: UX-SF-001-run-predefined-flow.md
           title: Run a Predefined Flow
   ```
   File references are in `groups[].capabilities[].file`. Capabilities may use optional group sub-folders (e.g., `group/UX-SF-001-file.md`).

4. **Libraries format** (`libraries/`):
   ```yaml
   families:
     - name: core
       libraries:
         - id: LIB-SF-001
           file: core/LIB-SF-001-di-kernel.md
           title: "DI Kernel"
   ```
   File references are in `families[].libraries[].file` and include family sub-folder prefixes (e.g., `core/LIB-SF-001.md`).

**Extraction approach:**

All four formats use a consistent `file:` key for referencing `.md` files. For each directory with an `index.yaml`:
1. Read the YAML
2. Extract all `file` values via pattern matching on the `file:` key
3. Compare against actual `.md` files in the directory (excluding `index.md`)
4. For sub-folder structures (libraries, capabilities), file paths include sub-folder prefixes
5. Report files missing from index (VAL-031..036, VAL-062) and stale index entries (VAL-037)

#### Overview.md Rules (VAL-038 through VAL-042)

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-038 | warning | Every behavior file is listed in overview.md `### Behaviors` table or section. |
| VAL-039 | warning | Every ADR is listed in overview.md `### Decisions` table or section. |
| VAL-040 | info | Every type file is listed in overview.md `### Types` table or section. |
| VAL-041 | info | Every architecture file is listed in overview.md `### Architecture` table or section. |
| VAL-042 | info | Every feature file is listed in overview.md `### Features` table or section. |

**How to check:** Read `overview.md`, search for filenames or IDs from each category. A file is "listed" if the overview.md contains its filename (e.g., `BEH-SF-001-graph-operations.md`) or its ID (e.g., `BEH-SF-001`).

**Automation:** `05-index-completeness.sh` covers VAL-031–037, VAL-062 (index sync) and `06-overview-completeness.sh` covers VAL-038–042 (overview mentions). Both handle all four index.yaml formats.

---

### Phase 7: Link & Content Structure (VAL-043 through VAL-048)

**Purpose:** Markdown links resolve, behavior sections have required subsections.

#### Markdown Link Rules (VAL-043 through VAL-045)

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-043 | error | All relative markdown links (`](../path)` or `](./path)`) resolve to existing files. Fragment-only links (`](#section)`) are excluded. |
| VAL-044 | warning | Links to behavior files use the correct `BEH-{infix}-NNN-slug.md` filename format. |
| VAL-045 | warning | Links to ADR files use the correct `ADR-NNN-slug.md` filename format. |

**Link extraction pattern:**
```
/\]\(([^)#][^)]*)\)/g
```
For each match, resolve the path relative to the file's directory. Check if the target file exists on disk.

**Scope:** Check links in all `.md` files. Skip external URLs (starting with `http://` or `https://`).

#### Content Structure Rules (VAL-046 through VAL-048)

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-046 | warning | Every `## BEH-{infix}-NNN:` section has a `### Contract` subsection. |
| VAL-047 | error | Every `### Contract` subsection contains a `REQUIREMENT (BEH-{infix}-NNN):` statement where NNN matches the parent section's ID. |
| VAL-048 | warning | Every `## BEH-{infix}-NNN:` section has a `### Verification` subsection. |

**How to check:**

1. For each behavior file, split content by `## BEH-{infix}-NNN:` headers
2. Within each section, check for `### Contract` and `### Verification` subsections
3. Within `### Contract`, check for `REQUIREMENT (BEH-{infix}-NNN):` where NNN matches the section's ID
4. Report mismatched IDs (e.g., section is `## BEH-SF-303:` but requirement says `REQUIREMENT (BEH-SF-057):`)

**Automation:** `07-link-integrity.sh` covers VAL-043–045 (link resolution) and `08-content-structure.sh` covers VAL-046–048 (Contract/Verification subsection checks with REQUIREMENT ID matching).

---

### Phase 8: Traceability & Semantics (VAL-049 through VAL-053)

**Purpose:** Traceability documents are complete and semantic cross-references are consistent.

| Rule | Severity | What to Check |
|------|----------|---------------|
| VAL-049 | warning | Every `INV-{infix}` ID appears in the traceability index or invariant-behavior trace file. |
| VAL-050 | warning | Every ADR ID appears in the traceability index or ADR-behavior trace file. |
| VAL-051 | warning | If ADR-X has `supersedes: [ADR-Y]`, then ADR-Y's `status` should be `"Superseded"`. |
| VAL-052 | info | Behaviors with `status: deprecated` should mention a replacement in their content (heuristic: contains "superseded by", "replaced by", "see BEH-{infix}-", or "see ADR-"). |
| VAL-053 | info | Port names in behavior `ports` frontmatter should appear in `types/ports.md` or `architecture/ports-and-adapters.md`. |

**For VAL-049/050:** Read the `traceability/` directory. Search for each INV/ADR ID in the traceability files. If the spec has no `traceability/` directory, skip these rules with an info note.

**For VAL-051:** Build a map of ADR supersession chains. For each `supersedes: [ADR-Y]`, read ADR-Y and check its `status` field.

**For VAL-052:** For each behavior file with `status: deprecated`, scan its content for replacement mentions.

**For VAL-053:** Collect all port names from behavior frontmatter `ports` fields. Check if they appear in `types/ports.md` or `architecture/ports-and-adapters.md`.

**Automation:** `09-traceability-matrix.sh` covers VAL-049–050 (ID presence in traceability files) and `10-semantic-consistency.sh` covers VAL-051–053 (supersession chains, deprecation hints, port documentation). VAL-051–053 are low-confidence heuristics — Claude should verify script findings and add semantic judgment.

---

## Finding Format

Each finding MUST follow this format:

```markdown
### F-NNN: <Short Title>
- **Severity:** error | warning | info
- **Rule:** VAL-NNN — <rule description from catalog>
- **File:** <path relative to spec root>
- **Evidence:** <exact quote, ID, or detail showing the violation>
- **Fix:** Invoke **<skill-name>** to <specific action>
```

Number findings sequentially (F-001, F-002, ...) within the report.

**Example findings:**

```markdown
### F-001: Duplicate behavior ID BEH-SF-057
- **Severity:** error
- **Rule:** VAL-001 — No duplicate BEH-SF IDs across all behavior files
- **File:** behaviors/BEH-SF-049-flow-definitions.md
- **Evidence:** `## BEH-SF-057: Convergence Evaluation` also defined in behaviors/BEH-SF-057-flow-execution.md
- **Fix:** Invoke **spec-behaviors** to renumber one of the duplicate sections

### F-002: Orphan behavior BEH-SF-300
- **Severity:** warning
- **Rule:** VAL-025 — Every BEH ID referenced by at least one feature
- **File:** behaviors/BEH-SF-001-graph-operations.md (gap-fill)
- **Evidence:** BEH-SF-300 (Idempotent Graph Sync) not in any feature's behaviors list
- **Fix:** Invoke **spec-features** to add BEH-SF-300 to the appropriate feature

### F-003: Missing Contract subsection
- **Severity:** warning
- **Rule:** VAL-046 — Every BEH section has a Contract subsection
- **File:** behaviors/BEH-SF-033-blackboard.md
- **Evidence:** Section `## BEH-SF-035: Blackboard Compaction` has no `### Contract`
- **Fix:** Invoke **spec-behaviors** to add the Contract subsection with a REQUIREMENT statement
```

---

## Output Format

Structure the review report with these sections in order:

### 1. Review Header

```markdown
## Spec Review: <spec path>

| Property | Value |
|----------|-------|
| Spec Path | `<path>` |
| Detected Tier | <tier name> |
| Infix | <XX> |
| Files Scanned | <N> .md files, <N> index.yaml files |
| Legacy Format | Yes/No |

### Severity Summary

| Severity | Count |
|----------|-------|
| error | <N> |
| warning | <N> |
| info | <N> |
| **Total** | **<N>** |
```

### 2. Phase Results

```markdown
### Phase Results

| Phase | Name | Status | Findings |
|-------|------|--------|----------|
| 1 | Scope & Tier Detection | DONE | — |
| 2 | ID Integrity | PASS / FAIL | <N> errors, <N> warnings |
| 3 | Frontmatter Schema | PASS / FAIL | <N> errors, <N> warnings |
| 4 | Forward Reference Integrity | PASS / FAIL | <N> errors, <N> warnings |
| 5 | Reverse Coverage | PASS / WARN | <N> warnings, <N> info |
| 6 | Index & Overview Completeness | PASS / FAIL | <N> errors, <N> warnings, <N> info |
| 7 | Link & Content Structure | PASS / FAIL | <N> errors, <N> warnings |
| 8 | Traceability & Semantics | PASS / WARN | <N> warnings, <N> info |
```

A phase **FAILs** if it has any `error`-severity findings. It **WARNs** if it has only `warning`-severity findings. It **PASSes** if it has no findings or only `info` findings.

### 3. Detailed Findings

Group by severity (errors first, then warnings, then info):

```markdown
### Errors (<N>)

<F-NNN findings with severity: error>

### Warnings (<N>)

<F-NNN findings with severity: warning>

### Info (<N>)

<F-NNN findings with severity: info>
```

### 4. Cross-Reference Graph Summary

```markdown
### Cross-Reference Graph

| Entity Type | Total | Referenced | Orphaned |
|-------------|-------|------------|----------|
| Behaviors (BEH-{infix}) | <N> | <N> | <N> |
| Features (FEAT-{infix}) | <N> | <N> | <N> |
| Capabilities (UX-{infix}) | <N> | <N> | <N> |
| Deliverables (DLV-{infix}) | <N> | <N> | <N> |
| Libraries (LIB-{infix}) | <N> | <N> | <N> |
| Invariants (INV-{infix}) | <N> | <N> | <N> |
| Decisions (ADR) | <N> | <N> | <N> |
| Type domains | <N> | <N> | <N> |
```

### 5. Positive Observations

Acknowledge what the spec does well. Examples:
- "All behavior files have consistent Contract + Verification sections"
- "Index.yaml files are fully synchronized with the filesystem"
- "Cross-reference chain from capabilities → features → behaviors is complete"

### 6. Remediation Priority

```markdown
### Remediation Priority

1. **Fix errors first** — <N> structural breaks must be resolved
   - <grouped summary of error findings with skill recommendations>
2. **Address warnings** — <N> coverage gaps
   - <grouped summary>
3. **Consider info items** — <N> suggestions
   - <grouped summary>

### Tier Promotion Readiness

<Current tier> → <Next tier>: <READY / NOT READY>
Missing for promotion: <list of missing components>
Blocking findings: <count of errors that must be resolved>
```

---

## Generalization for Any Spec

The VAL rules in the catalog use `BEH-SF` (specforge-specific). When reviewing any spec:

1. **Replace `SF` with the detected infix** — e.g., `GD` for guard, `FL` for flow, `CK` for clock
2. **Apply the same structural rules regardless of package** — the rules are about structure, not content
3. **Handle legacy ID formats** — Some packages use non-standard ID prefixes:
   - Guard: `REQ-GUARD-NNN` instead of `BEH-GD-NNN`
   - Flow: `FLW-NNN` instead of `BEH-FL-NNN`
   - Clock: `CLK-{DOMAIN}-NNN` instead of `BEH-CK-NNN`
   When legacy formats are detected, note as info and adapt the scanning patterns accordingly
4. **Handle legacy directory structures** — Specs with numbered chapters (e.g., `spec/libs/guard/`) instead of the canonical directory structure:
   - Detect by checking for files matching `NN-*.md` at the spec root
   - Note as info: "Legacy chapter-based format detected"
   - Validate subdirectories that follow the canonical structure (e.g., if `behaviors/` exists within a legacy spec, validate it normally)
   - Skip rules that require directories not present in the legacy format

---

## Behavioral Instructions

1. **Run `validate-all.sh` FIRST** — before any manual file reading, execute `bash .claude/skills/spec-review/scripts/validate-all.sh <spec-dir>`. This automates all 62 VAL rules and produces machine-readable TSV findings. Use the script output to populate findings for Phases 2–8. Manual file reading is only needed for: (a) rules the script reports as `skip`, (b) semantic judgment nuances (especially VAL-051–053), (c) positive observations about spec quality, and (d) Phase 1 tier classification.

2. **Execute Phase 1 manually** — read the spec root directory to determine tier, infix, file counts, and legacy format. The scripts auto-detect infix but cannot classify tiers.

3. **Do not auto-fix** — report findings only. Each finding MUST suggest which skill to invoke for remediation.

4. **Handle mixed-format and legacy specs gracefully** — adapt patterns to the detected format. Never report a legacy format as an error; report it as info.

5. **State tier promotion readiness** at the end — summarize what's needed to reach the next tier.

6. **Be precise with IDs** — show the exact file path, frontmatter field, and referenced ID in every finding. Quote the evidence directly from the file.

7. **Do not report rules for missing directories** — if `features/` doesn't exist, don't report VAL-002, VAL-034, VAL-021, VAL-026, VAL-042 violations. Instead note that the directory is absent (relevant for tier detection). The scripts handle this automatically via `skip` findings.

8. **Track counts accurately** — the severity summary and phase results tables must have accurate counts matching the actual findings listed. Use the script's summary table counts as the baseline.

9. **Manual deep-dive when needed** — if script findings for a rule seem suspicious (false positives/negatives), read the relevant files manually to verify. Trust but verify.

---

## Skill Cross-Reference (for Fix recommendations)

| Issue Domain | Skill to Invoke |
|-------------|-----------------|
| Behavior file structure | **spec-behaviors** |
| Feature files | **spec-features** |
| Capability files | **spec-capabilities** |
| Invariant files | **spec-invariants** |
| Decision/ADR files | **spec-decisions** |
| Type files | **spec-types** |
| Deliverable files | **spec-deliverables** |
| Library files | **spec-libraries** |
| Index.yaml sync | **spec-behaviors**, **spec-features**, or the relevant directory skill |
| Overview.md gaps | **spec-overview** |
| Traceability gaps | **spec-traceability** |
| Roadmap references | **roadmap-spec-author** |
| Plugin behaviors | **spec-plugins** |
| Process documents | **spec-process** |
| Architecture docs | **spec-architecture** |
| Cross-reference integrity | **spec-authoring** (orchestrator) |

---

## Complete VAL Rule Quick Reference

| Rule | Sev | Category | One-Line Description |
|------|-----|----------|---------------------|
| VAL-001 | E | ID Integrity | No duplicate BEH IDs |
| VAL-002 | E | ID Integrity | No duplicate FEAT IDs |
| VAL-003 | E | ID Integrity | No duplicate UX IDs |
| VAL-004 | E | ID Integrity | No duplicate INV IDs |
| VAL-005 | E | ID Integrity | No duplicate ADR IDs |
| VAL-006 | E | ID Integrity | No duplicate TYPE IDs |
| VAL-007 | E | ID Integrity | BEH section IDs within file's id_range |
| VAL-008 | E | ID Integrity | id_range values don't overlap |
| VAL-009 | W | ID Integrity | Filename prefix matches frontmatter id |
| VAL-010 | E | Frontmatter | Valid YAML frontmatter present |
| VAL-011 | E | Frontmatter | Required fields: id, kind, title, status |
| VAL-012 | E | Frontmatter | kind matches parent directory |
| VAL-013 | W | Frontmatter | status is valid for kind |
| VAL-014 | W | Frontmatter | Behavior recommended fields present |
| VAL-015 | W | Frontmatter | Feature recommended fields present |
| VAL-016 | W | Frontmatter | Capability recommended fields present |
| VAL-017 | W | Frontmatter | Decision recommended fields present |
| VAL-018 | E | Forward Ref | invariants[] → existing INV |
| VAL-019 | E | Forward Ref | adrs[] → existing ADR |
| VAL-020 | E | Forward Ref | behaviors[] → existing BEH |
| VAL-021 | E | Forward Ref | features[] → existing FEAT |
| VAL-022 | W | Forward Ref | types[] → existing type file |
| VAL-023 | E | Forward Ref | supersedes[] → existing ADR |
| VAL-024 | W | Forward Ref | roadmap_phases[] → existing roadmap |
| VAL-025 | W | Reverse Cov | Every BEH referenced by a feature |
| VAL-026 | W | Reverse Cov | Every FEAT referenced by a capability |
| VAL-027 | W | Reverse Cov | Every INV referenced by a behavior |
| VAL-028 | W | Reverse Cov | Every ADR referenced by a behavior |
| VAL-029 | I | Reverse Cov | Every type domain referenced by a behavior |
| VAL-030 | W | Reverse Cov | Every BEH file has invariants or adrs |
| VAL-031 | E | Index | behaviors/ files listed in index.yaml |
| VAL-032 | E | Index | decisions/ files listed in index.yaml |
| VAL-033 | E | Index | types/ files listed in index.yaml |
| VAL-034 | W | Index | features/ files listed in index.yaml |
| VAL-035 | W | Index | capabilities/ files listed in index.yaml |
| VAL-036 | W | Index | invariants/ files listed in index.yaml |
| VAL-037 | E | Index | index.yaml entries point to existing files |
| VAL-038 | W | Overview | Behavior files in overview.md |
| VAL-039 | W | Overview | ADRs in overview.md |
| VAL-040 | I | Overview | Type files in overview.md |
| VAL-041 | I | Overview | Architecture files in overview.md |
| VAL-042 | I | Overview | Feature files in overview.md |
| VAL-043 | E | Links | Relative markdown links resolve |
| VAL-044 | W | Links | BEH link filename format correct |
| VAL-045 | W | Links | ADR link filename format correct |
| VAL-046 | W | Content | BEH sections have Contract subsection |
| VAL-047 | E | Content | Contract has matching REQUIREMENT ID |
| VAL-048 | W | Content | BEH sections have Verification subsection |
| VAL-049 | W | Traceability | INV IDs in traceability index |
| VAL-050 | W | Traceability | ADR IDs in traceability index |
| VAL-051 | W | Semantics | Superseded ADR has Superseded status |
| VAL-052 | I | Semantics | Deprecated behaviors mention replacement |
| VAL-053 | I | Semantics | Port names appear in type/arch docs |
| VAL-054 | W | Reverse Cov | Every UX referenced by a deliverable |
| VAL-055 | W | Reverse Cov | Every FEAT implemented by a library |
| VAL-056 | E | Frontmatter | Deliverable valid frontmatter |
| VAL-057 | E | Frontmatter | Library valid frontmatter |
| VAL-058 | E | Forward Ref | Deliverable capabilities[] → existing UX |
| VAL-059 | E | Forward Ref | Deliverable depends_on[] → existing LIB |
| VAL-060 | E | Forward Ref | Library features[] → existing FEAT |
| VAL-061 | E | Forward Ref | Library depends_on[] → existing LIB |
| VAL-062 | W | Index | deliverables/ and libraries/ files in index.yaml |

**Legend:** E = error, W = warning, I = info
