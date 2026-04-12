# Phase 7: Re-Score Verification

**Status**: TODO
**Depends on**: Phase 6 (Tests & Snapshots)
**Target**: Average >= 9.0, no axis below 8.0
**Estimated Duration**: 1 session

---

## 1. Pre-Verification Checklist

Every item must be confirmed before launching expert agents. If any item fails, stop and fix it before proceeding.

- [ ] **All manifests updated** -- changes from Phases 1-4 are committed
  - `extensions/software/manifest.json`
  - `extensions/product/manifest.json`
  - `extensions/governance/manifest.json`
  - `extensions/formal/manifest.json`
- [ ] **Model builder patched** -- Phase 5 changes committed
  - `crates/specforge-emitter/src/model/build.rs`
  - `crates/specforge-emitter/src/model/mod.rs`
- [ ] **All snapshots updated** -- Phase 6 complete
- [ ] **`cargo test --workspace` passes** -- zero failures
  ```bash
  cargo test --workspace 2>&1 | tail -5
  # Expect: "test result: ok. N passed; 0 failed; 0 ignored"
  ```
- [ ] **`cargo clippy --workspace` clean** -- zero warnings
  ```bash
  cargo clippy --workspace -- -D warnings 2>&1 | tail -3
  # Expect: no output (clean)
  ```
- [ ] **`cargo build --release -p specforge-cli` succeeds** -- release binary compiles
- [ ] **Working tree is clean** -- no uncommitted changes that could affect output
  ```bash
  git status --short
  # Expect: empty output
  ```

---

## 2. Model Regeneration

Generate all model representations from the updated codebase. These become the artifacts that every expert agent evaluates.

### 2.1 Build Release Binary

```bash
cargo build --release -p specforge-cli
```

### 2.2 Generate Model Artifacts

```bash
# Full Markdown model (primary artifact for expert analysis)
specforge model --fields=all --format=markdown > /tmp/specforge-model-full.md

# Mermaid diagram (visual relationship verification)
specforge model --fields=all --format=mermaid > /tmp/specforge-model-full.mmd

# JSON model (machine-readable, for structural assertions)
specforge model --fields=all --format=json > /tmp/specforge-model-full.json

# DBML model (relational perspective)
specforge model --fields=all --format=dbml > /tmp/specforge-model-full.dbml

# Per-extension views (for boundary analysis)
specforge model --fields=all --extension=@specforge/software > /tmp/specforge-model-software.md
specforge model --fields=all --extension=@specforge/product > /tmp/specforge-model-product.md
specforge model --fields=all --extension=@specforge/governance > /tmp/specforge-model-governance.md
specforge model --fields=all --extension=@specforge/formal > /tmp/specforge-model-formal.md
```

### 2.3 Verify Artifacts Are Non-Empty

```bash
for f in /tmp/specforge-model-*.md /tmp/specforge-model-full.mmd \
         /tmp/specforge-model-full.json /tmp/specforge-model-full.dbml; do
  lines=$(wc -l < "$f")
  echo "$f: $lines lines"
  if [ "$lines" -lt 10 ]; then
    echo "ERROR: $f is suspiciously short -- investigate before proceeding"
    exit 1
  fi
done
```

### 2.4 Archive Artifacts

```bash
# Save a timestamped copy for future comparison
mkdir -p /tmp/specforge-rescore-$(date +%Y%m%d)
cp /tmp/specforge-model-* /tmp/specforge-rescore-$(date +%Y%m%d)/
```

---

## 3. Scoring Rubric

Each expert agent scores its assigned axis on a 1-10 scale using the rubric below. The rubric is axis-specific, but all share the same severity tiers.

### 3.1 Universal Severity Tiers

| Score | Meaning |
|-------|---------|
| 10 | Flawless. No improvements identifiable by a domain expert. |
| 9 | Excellent. At most 1-2 minor cosmetic suggestions, no structural issues. |
| 8 | Good. A few concrete improvements possible, but nothing blocks usability. |
| 7 | Adequate. Several clear improvements needed; a knowledgeable user would notice gaps. |
| 6 | Below average. Multiple structural issues or omissions that affect usability. |
| 5 | Poor. Fundamental problems that undermine the axis's purpose. |
| 1-4 | Broken. Major category of concern is unaddressed or actively wrong. |

### 3.2 Axis-Specific Rubrics

Each expert must evaluate against these specific criteria:

| # | Axis | 9+ Requires | 8 Allows |
|---|------|-------------|----------|
| 1 | Domain Completeness | All entity kinds cover their domain with no missing first-class concepts. Every extension's declared kinds are justified and sufficient. | One entity kind that could arguably be added but is not strictly necessary. |
| 2 | Relationship Correctness | Every edge type has correct source/target kinds, no phantom edges, no missing required relationships, cardinalities are accurate. | One edge with debatable direction or cardinality. |
| 3 | Field Completeness | Every entity kind has all fields needed for its domain purpose. No missing required fields, no orphan fields. | One or two fields that could arguably be added for advanced use cases. |
| 4 | Naming Consistency | All identifiers (kinds, fields, edges) follow a single naming convention. snake_case throughout, verb phrases for edges, noun phrases for kinds. | One or two inconsistencies that do not impair readability. |
| 5 | Cross-Extension Coherence | Entity enhancements, peer dependencies, and shared fields compose cleanly. No contradictions, no ambiguous ownership. | One enhancement whose ownership could be debated. |
| 6 | AI Agent Usability | Model output is self-contained, unambiguous, parseable. An LLM can consume the full model and answer structural questions without external docs. | Minor ambiguity that requires one clarifying question. |
| 7 | Normalization | No data is duplicated across entity kinds. Each fact lives in exactly one place. References use IDs, not embedded copies. | One field that could arguably be normalized further. |
| 8 | Redundancy | No overlapping entity kinds, no duplicate edge types, no fields that serve the same purpose under different names. | One field pair that is borderline redundant but serves different audiences. |
| 9 | Connectivity | Every entity kind is reachable from at least one other via edges. No disconnected subgraphs. Cross-extension edges bridge all extensions. | One entity kind with only a single edge connection. |
| 10 | Cardinality | All edges specify correct multiplicity (1:1, 1:N, N:M). Reference fields match edge cardinalities. No implicit many-to-many without junction. | One edge whose cardinality is inferred rather than explicit. |
| 11 | DDD Alignment | Aggregates, value objects, and bounded contexts are clearly delineated. Entity kinds map to DDD building blocks. Extensions correspond to bounded contexts. | Minor debate over whether one entity is an aggregate or an entity. |
| 12 | Product Management | Product extension covers planning, tracking, and delivery lifecycle. Journeys, milestones, features, deliverables, releases form a coherent PM workflow. | One PM concept that is modeled indirectly rather than as a first-class entity. |
| 13 | Software Engineering | Software extension covers behavioral contracts, interfaces, types, events, and invariants. Adequate for specifying a non-trivial system. | One SE concept that requires composition of multiple entities rather than direct support. |
| 14 | Governance | Governance extension covers decisions, constraints, and failure modes. Traceability from decisions to the entities they govern is clear. | One governance concept that requires a workaround. |
| 15 | Formal Methods | Formal extension covers conditions, properties, axioms, protocols, refinements, and processes. Specification layering is sound. | One formal methods concept that is modeled at reduced fidelity. |
| 16 | Scalability | Model handles 500+ entities without structural degradation. No O(N^2) relationships, no mandatory global fields, modular extension loading. | Performance concern at 1000+ entities but not at 500. |
| 17 | Learnability | A new user can understand the model in under 15 minutes. Naming is self-documenting, structure is predictable, few surprises. | One concept that requires reading documentation to understand. |
| 18 | Edge Label Quality | Edge labels are descriptive, unambiguous verb phrases. Reading "A --EdgeLabel--> B" forms a grammatical sentence. No generic labels like "relates_to". | One edge label that is technically correct but not maximally descriptive. |
| 19 | Description Quality | All entity kinds, fields, and edge types have clear, concise, non-redundant descriptions. Descriptions add information beyond the name. | One or two descriptions that merely restate the name. |
| 20 | Extension Boundary | Each extension owns a coherent, non-overlapping domain. Dependencies flow in one direction. No circular extension dependencies. | One entity kind that could arguably live in a different extension. |

---

## 4. Expert Agent Launch Protocol

### 4.1 Agent Prompt Template

Each of the 20 agents receives the following prompt, with `{AXIS_NUMBER}`, `{AXIS_NAME}`, and `{RUBRIC}` substituted:

```
You are Expert #{AXIS_NUMBER}: {AXIS_NAME}.

You are evaluating the SpecForge logical data model for quality on the
axis of "{AXIS_NAME}".

## Your Task

1. Read the full model output below carefully.
2. Score the model on a scale of 1-10 using this rubric:
   {RUBRIC}
3. List every specific issue you found, ordered by severity.
4. For each issue, provide:
   - A one-line description
   - The entity kind, field, or edge type affected
   - Suggested fix
5. Give your final score with a one-paragraph justification.

## Scoring Rules

- Be rigorous. A score of 9 means near-flawless.
- A score of 10 means you cannot identify a single improvement.
- Do not inflate scores. If you see problems, score accordingly.
- Compare against production-grade data models in the domain.

## Model Output

<model>
{FULL_MARKDOWN_MODEL}
</model>

## Extension-Specific Views (for reference)

<software>{SOFTWARE_MODEL}</software>
<product>{PRODUCT_MODEL}</product>
<governance>{GOVERNANCE_MODEL}</governance>
<formal>{FORMAL_MODEL}</formal>

## Your Response Format

### Score: X/10

### Issues Found
1. [SEVERITY] Description -- affected: `entity_or_edge` -- fix: suggestion
2. ...

### Justification
One paragraph explaining the score.
```

### 4.2 Launch Sequence

Launch all 20 agents in parallel. Each agent is independent and analyzes the same model artifacts.

```
Expert  1: Domain Completeness
Expert  2: Relationship Correctness
Expert  3: Field Completeness
Expert  4: Naming Consistency
Expert  5: Cross-Extension Coherence
Expert  6: AI Agent Usability
Expert  7: Normalization
Expert  8: Redundancy
Expert  9: Connectivity
Expert 10: Cardinality
Expert 11: DDD Alignment
Expert 12: Product Management
Expert 13: Software Engineering
Expert 14: Governance
Expert 15: Formal Methods
Expert 16: Scalability
Expert 17: Learnability
Expert 18: Edge Label Quality
Expert 19: Description Quality
Expert 20: Extension Boundary
```

### 4.3 Agent Input Files

Every agent receives the same file set:

| File | Purpose |
|------|---------|
| `/tmp/specforge-model-full.md` | Primary analysis artifact |
| `/tmp/specforge-model-full.mmd` | Visual relationship map |
| `/tmp/specforge-model-full.json` | Structural assertions |
| `/tmp/specforge-model-software.md` | Software extension detail |
| `/tmp/specforge-model-product.md` | Product extension detail |
| `/tmp/specforge-model-governance.md` | Governance extension detail |
| `/tmp/specforge-model-formal.md` | Formal extension detail |

---

## 5. Score Comparison Table

### 5.1 Before vs After

| # | Axis | Before | Phase Impact | Predicted | Actual | Delta |
|---|------|--------|--------------|-----------|--------|-------|
| 1 | Domain Completeness | 6 | P5 +2 | 8 | ___ | ___ |
| 2 | Relationship Correctness | 6 | P1 +2, P5 +2 | 10 | ___ | ___ |
| 3 | Field Completeness | 6 | P3 +2 | 8 | ___ | ___ |
| 4 | Naming Consistency | 6 | P2 +3, P3 +2 | 10+ | ___ | ___ |
| 5 | Cross-Extension Coherence | 7 | P1 +2, P4 +2, P5 +2 | 10+ | ___ | ___ |
| 6 | AI Agent Usability | 7 | P5 +2 | 9 | ___ | ___ |
| 7 | Normalization | 7 | P1 +2 | 9 | ___ | ___ |
| 8 | Redundancy | 7 | P1 +2 | 9 | ___ | ___ |
| 9 | Connectivity | 6 | P4 +2 | 8 | ___ | ___ |
| 10 | Cardinality | 7 | P1 +2, P3 +2 | 10+ | ___ | ___ |
| 11 | DDD Alignment | 5 | (indirect) | 7 | ___ | ___ |
| 12 | Product Management | 8 | (indirect) | 9 | ___ | ___ |
| 13 | Software Engineering | 8 | (indirect) | 9 | ___ | ___ |
| 14 | Governance | 6 | P4 +2 | 8 | ___ | ___ |
| 15 | Formal Methods | 8 | (indirect) | 9 | ___ | ___ |
| 16 | Scalability | 6 | P4 +2 | 8 | ___ | ___ |
| 17 | Learnability | 5 | P2 +3 | 8 | ___ | ___ |
| 18 | Edge Label Quality | 6 | P2 +3 | 9 | ___ | ___ |
| 19 | Description Quality | 7 | (indirect) | 8 | ___ | ___ |
| 20 | Extension Boundary | 8 | (indirect) | 9 | ___ | ___ |
| | **Average** | **6.6** | | **8.9** | ___ | ___ |

### 5.2 Predicted Risk Areas

The following axes have predicted scores below 9.0 and may require additional work:

| Axis | Predicted | Risk | Mitigation |
|------|-----------|------|------------|
| 1 Domain Completeness | 8 | Medium -- depends on P5 model builder accuracy | Ensure model builder emits ALL entity kinds from ALL extensions |
| 3 Field Completeness | 8 | Medium -- depends on P3 field normalization | Verify every declared field appears in model output |
| 9 Connectivity | 8 | Medium -- depends on P4 cross-extension edges | Verify no disconnected subgraph in mermaid output |
| 11 DDD Alignment | 7 | High -- no phase directly targets DDD | May need a targeted patch if score < 8 |
| 14 Governance | 8 | Medium -- depends on P4 governance edges | Verify governance entities connect to all other extensions |
| 16 Scalability | 8 | Medium -- structural, hard to fix in model alone | Ensure no O(N^2) edge patterns |
| 17 Learnability | 8 | Medium -- depends on P2 naming clarity | Verify all renamed edges are self-documenting |
| 19 Description Quality | 8 | Medium -- no phase directly targets descriptions | May need description pass if score < 8 |

---

## 6. Failure Protocol

### 6.1 Per-Axis Failure (Score < 8)

If any individual axis scores below 8:

1. **Diagnose**: Read the expert's issue list. Categorize each issue as:
   - **Manifest fix** -- wrong/missing data in extension manifests
   - **Model builder fix** -- data exists but is not rendered in output
   - **Structural fix** -- requires new entity kinds, edges, or fields
   - **Description fix** -- data is correct but poorly described

2. **Create patch plan**: Write a mini-plan with:
   - Exact files to modify
   - Exact changes (diff-level)
   - Which tests must be updated

3. **Apply patch**: Make the changes, run `cargo test --workspace`.

4. **Regenerate model**: Re-run Section 2.2 to produce updated artifacts.

5. **Re-run single expert**: Launch only the failing expert agent with the updated model.

6. **Verify score >= 8**: If still failing, escalate to Section 6.2.

### 6.2 Average Failure (Average < 9.0)

If the overall average is below 9.0 after all per-axis patches:

1. **Identify systemic issues**: Look for patterns across low-scoring axes.
   - Are multiple axes flagging the same entity kind?
   - Is one extension dragging down multiple scores?
   - Is the model builder dropping information?

2. **Create Phase 7b plan**: A focused additional improvement phase targeting the systemic root cause.

3. **Execute Phase 7b**: Apply fixes, update tests, regenerate model.

4. **Full re-score**: Re-launch all 20 experts (not just the failing ones) because systemic fixes may affect all axes.

### 6.3 Escalation Ladder

| Situation | Action |
|-----------|--------|
| 1-2 axes at 7, rest at 9+ | Per-axis patch (Section 6.1) |
| 3-5 axes below 8 | Phase 7b (Section 6.2) |
| 6+ axes below 8 | Revisit Phase 1-5 assumptions; root cause likely in manifest structure |
| Any axis below 5 | Critical regression -- git bisect to find offending commit |
| Average drops from initial 6.6 | Regression -- revert to pre-Phase-1 and investigate |

---

## 7. Progress Tracking

### 7.1 Pre-Verification

- [ ] Phase 1 committed and tested
- [ ] Phase 2 committed and tested
- [ ] Phase 3 committed and tested
- [ ] Phase 4 committed and tested
- [ ] Phase 5 committed and tested
- [ ] Phase 6 committed and tested
- [ ] `cargo test --workspace` -- 0 failures
- [ ] `cargo clippy --workspace` -- 0 warnings
- [ ] Working tree clean

### 7.2 Model Generation

- [ ] Release binary built
- [ ] Full Markdown model generated
- [ ] Full Mermaid model generated
- [ ] Full JSON model generated
- [ ] Full DBML model generated
- [ ] Per-extension models generated (software, product, governance, formal)
- [ ] All artifacts verified non-empty
- [ ] Artifacts archived with timestamp

### 7.3 Expert Scoring

- [ ] Expert 1 (Domain Completeness): Score ___
- [ ] Expert 2 (Relationship Correctness): Score ___
- [ ] Expert 3 (Field Completeness): Score ___
- [ ] Expert 4 (Naming Consistency): Score ___
- [ ] Expert 5 (Cross-Extension Coherence): Score ___
- [ ] Expert 6 (AI Agent Usability): Score ___
- [ ] Expert 7 (Normalization): Score ___
- [ ] Expert 8 (Redundancy): Score ___
- [ ] Expert 9 (Connectivity): Score ___
- [ ] Expert 10 (Cardinality): Score ___
- [ ] Expert 11 (DDD Alignment): Score ___
- [ ] Expert 12 (Product Management): Score ___
- [ ] Expert 13 (Software Engineering): Score ___
- [ ] Expert 14 (Governance): Score ___
- [ ] Expert 15 (Formal Methods): Score ___
- [ ] Expert 16 (Scalability): Score ___
- [ ] Expert 17 (Learnability): Score ___
- [ ] Expert 18 (Edge Label Quality): Score ___
- [ ] Expert 19 (Description Quality): Score ___
- [ ] Expert 20 (Extension Boundary): Score ___

### 7.4 Post-Scoring

- [ ] All 20 scores recorded in comparison table (Section 5.1)
- [ ] Average computed
- [ ] Average >= 9.0 confirmed
- [ ] No axis below 8.0 confirmed
- [ ] Failure protocol executed for any sub-8 axes (if applicable)
- [ ] Re-scoring of patched axes complete (if applicable)
- [ ] Final scores committed to MASTER.md
- [ ] Phase 7 status updated to DONE in MASTER.md

---

## 8. Success Criteria

Phase 7 is DONE when ALL of the following hold:

1. **Average score >= 9.0** across all 20 axes
2. **No individual axis below 8.0**
3. **All scores recorded** in the comparison table with justifications
4. **MASTER.md updated** with final "After" scorecard
5. **All changes committed** with passing tests and clean clippy

If criteria 1 or 2 are not met after the failure protocol, the phase remains OPEN and a Phase 7b plan is created.

---

## 9. Estimated Score Budget

Based on phase impacts, the predicted improvement path:

```
Initial average:  6.6
Phase 1 impact:   +0.5 (axes 2,5,7,8,10)
Phase 2 impact:   +0.45 (axes 4,17,18)
Phase 3 impact:   +0.3 (axes 3,4,10)
Phase 4 impact:   +0.4 (axes 5,9,14,16)
Phase 5 impact:   +0.5 (axes 1,2,5,6)
Indirect lift:    +0.15 (axes 11,12,13,15,19,20)
                  ------
Predicted total:  8.9

Gap to 9.0:       0.1 (within failure protocol margin)
```

The predicted average of 8.9 is within striking distance of the 9.0 target. The failure protocol (Section 6) provides the mechanism to close this gap through targeted per-axis patches. The highest-risk axes (11-DDD Alignment at predicted 7, 19-Description Quality at predicted 8) are the most likely candidates for additional work.
