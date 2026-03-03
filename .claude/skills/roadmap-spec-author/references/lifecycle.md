# Roadmap Lifecycle Cycle

The Roadmap Spec system follows a **cyclic lifecycle** where research informs grouping, grouping organizes work, work drives delivery, and delivery feeds the next research cycle.

## The Cycle

```
                      RESEARCH / DISCOVERY
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│                   Grouping Container (PH/REL/FT)             │
│  - Organizes related work into phases, releases, or features │
│  - Defines scope boundaries and ordering                     │
│  - References research sources that informed it              │
└──────────────────────┬───────────────────────────────────────┘
                       │ contains
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                      Work Item (WI)                           │
│  - Defines a single deliverable within its container         │
│  - Specifies scope, package, and status                      │
│  - Allocates behavior IDs from the spec                      │
└──────────────────────┬───────────────────────────────────────┘
                       │ allocated
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                  Behavior Allocation (BEH)                    │
│  - Maps work items to spec behavior ID ranges                │
│  - Ensures every behavior is accounted for                   │
│  - Enables traceability from roadmap to spec                 │
└──────────────────────┬───────────────────────────────────────┘
                       │ measured by
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                   Exit Criterion (EC)                         │
│  - Defines measurable completion condition                   │
│  - Must be verifiable (not vague)                            │
│  - Gates status transition to Delivered                      │
└──────────────────────┬───────────────────────────────────────┘
                       │ confirms
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                       Delivery                                │
│  - All exit criteria met                                     │
│  - Status transitions: In Progress → Delivered               │
│  - Deliverable artifacts exist and pass checks               │
└──────────────────────┬───────────────────────────────────────┘
                       │ validates
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                     Verification                              │
│  - 8-check script validates cross-references                 │
│  - Status values, behavior ranges, dependencies confirmed    │
│  - Product milestones aligned                                │
└──────────────────────┬───────────────────────────────────────┘
                       │ aligns to
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                  Product Milestone (PT)                       │
│  - Business value aligned to technical delivery              │
│  - Success metric validates business outcome                 │
│  - Feeds insights back to research                           │
└──────────────────────┬───────────────────────────────────────┘
                       │ feeds
                       ▼
                  RESEARCH / DISCOVERY
                  (next cycle begins)
```

## Entity Mapping

| Lifecycle Stage | Entity | Responsibility |
|-----------------|--------|----------------|
| Research | External docs | Inform what needs building |
| Organization | Grouping Container (`PH`/`REL`/`FT`) | Structure work into manageable units |
| Definition | Work Item (`WI`) | Define scope, deliverables, behaviors |
| Traceability | Behavior Allocation (`BEH-XX-NNN`) | Link work to spec behaviors |
| Completion | Exit Criterion (`EC`) | Gate delivery with measurable conditions |
| Delivery | Status transition | Confirm criteria met, artifacts exist |
| Validation | Verification script | Automated cross-reference checks |
| Alignment | Product Milestone (`PT`) | Connect technical work to business goals |

## Cycle Enforcement Rules

### Rule 1: Grouping Containers Must Have Exit Criteria

Every grouping container (phase, release, or feature) must define at least one exit criterion:

```markdown
### Exit Criteria

- [ ] EC-PH-1-1: All port factory tests pass
- [ ] EC-PH-1-2: API documentation published
```

A container without exit criteria has no definition of "done."

### Rule 2: Work Items Must Have Status + Scope + Deliverable

Every work item must specify its status, what it covers, and what it produces:

```markdown
| # | Deliverable | Package | Behaviors | Status |
|---|-------------|---------|-----------|--------|
| WI-PH-1-1 | Port factory | `@hex-di/core` | BEH-SF-001–042 | Planned |
```

A work item missing any of these fields is incomplete.

### Rule 3: Behavior Ranges Must Reference Valid IDs

When traceability depth is `full` or `medium`, all `BEH-XX-NNN–NNN` ranges must correspond to behaviors defined in the spec's `behaviors/` directory:

```markdown
| WI-PH-1-1 | Port factory | BEH-SF-001–042 |
```

Verify: the spec has behaviors numbered BEH-SF-001 through BEH-SF-042.

### Rule 4: Exit Criteria Must Be Measurable

Exit criteria must state a verifiable condition, not a vague aspiration:

```markdown
# Good — measurable
- [ ] EC-PH-1-1: 100% of port factory unit tests pass
- [ ] EC-PH-1-2: TypeScript strict mode compiles with zero errors

# Bad — vague
- [ ] EC-PH-1-1: Port factory works well
- [ ] EC-PH-1-2: Code quality is good
```

### Rule 5: Dependencies Must Not Form Cycles

The dependency graph must be a DAG (directed acyclic graph):

```markdown
# Valid (no cycles)
PH-1 → PH-2 → PH-3

# Invalid (cycle: PH-2 → PH-3 → PH-2)
PH-1 → PH-2 → PH-3 → PH-2
```

### Rule 6: Product Milestones Must Reference Existing Containers

Every `PT-N` must reference containers that exist in the roadmap:

```markdown
# Valid
| PT-1 | MVP | PH-1, PH-2 |    ← PH-1 and PH-2 exist

# Invalid
| PT-1 | MVP | PH-1, PH-99 |   ← PH-99 doesn't exist
```

### Rule 7: External Dependencies Must Specify Blocking Container

External dependency entries must name the container they block:

```markdown
# Valid
| `@hex-di/core` v2.0 | PH-3 | PH-2 must deliver first |

# Invalid — no blocking reference
| `@hex-di/core` v2.0 | — | Needed sometime |
```

### Rule 8: Research Sources Must Be Valid File Paths

When research traceability is `explicit-source`, all `Source:` values must point to existing files:

```markdown
# Valid — file exists
**Source:** spec/research/foundation-discovery.md

# Invalid — file doesn't exist
**Source:** spec/research/nonexistent.md
```

## Status Lifecycle State Machine

```
                    ┌─────────────────────────────────┐
                    │                                  │
                    │      ┌──────────┐                │
                    │      │ Deferred │                │
                    │      └────▲─────┘                │
                    │           │                      │
                    │     can defer from               │
                    │     any active state              │
                    │           │                      │
    ┌─────────┐    │    ┌──────┴───┐    ┌────────────┐│    ┌───────────┐
    │ Planned ├────┼───►│Specified ├───►│In Progress ├┼───►│ Delivered │
    └─────────┘    │    └──────────┘    └────────────┘│    └───────────┘
                    │                                  │
                    └─────────────────────────────────┘
```

**Planned** — Identified but not detailed. Can transition to Specified (spec written) or Deferred (postponed).

**Specified** — Spec complete, deliverables defined, exit criteria set. Can transition to In Progress (work begins) or Deferred.

**In Progress** — Active implementation. Can transition to Delivered (all exit criteria met) or Deferred (blocked or reprioritized).

**Delivered** — Terminal state. All exit criteria confirmed met. Cannot transition further.

**Deferred** — Parked. Can only transition back to Planned (re-entering the cycle).

## Tracing a Complete Cycle

Example: Adding a Guard policy engine to the roadmap.

```
1. RESEARCH
   Source: spec/libs/guard/research/policy-patterns.md
   Discovery: Need 10 policy kinds with composable evaluation

2. GROUPING
   FT-3: Policy Engine
   Status: Planned
   Source: spec/libs/guard/research/policy-patterns.md

3. WORK ITEMS
   WI-FT-3-1: Core evaluator          — BEH-GD-015–032, §3–§12
   WI-FT-3-2: Policy composition      — BEH-GD-033–045, §13–§18
   WI-FT-3-3: Async evaluation        — BEH-GD-046–058, §19–§24

4. BEHAVIOR ALLOCATION
   BEH-GD-015–058 allocated across 3 work items (44 behaviors)
   Verified: behaviors/guard-policies.md contains BEH-GD-015 through BEH-GD-058

5. EXIT CRITERIA
   EC-FT-3-1: All 44 policy behaviors pass unit tests
   EC-FT-3-2: Async evaluator handles 100+ concurrent policies without deadlock
   EC-FT-3-3: allOf/anyOf/not composition tested with 3+ nesting levels

6. DELIVERY
   Status: Specified → In Progress → Delivered
   All 3 exit criteria confirmed met

7. VERIFICATION
   verify-roadmap.sh: 8/8 checks PASS
   - Status values valid ✓
   - BEH-GD-015–058 exist in behaviors/ ✓
   - FT-3 in status summary ✓
   - 3 exit criteria present ✓
   - No dependency cycles ✓
   - Spec sections §3–§24 exist ✓
   - No product milestones to check ✓
   - No external deps to check ✓

8. PRODUCT MILESTONE (if applicable)
   PT-2: Authorization MVP — aligned to FT-1, FT-2, FT-3
   Success metric: Guard library usable in production with 10 policy kinds

9. NEXT CYCLE
   Delivery of FT-3 revealed: need FT-4 (Cedar integration) for enterprise use
   Research: spec/libs/guard-cedar/research/cedar-comparison.md
```

## Verification Checklist

| Check | Rule | How to Verify |
|-------|------|---------------|
| Every container has exit criteria | Rule 1 | At least one `EC-` entry per container |
| Every work item has status + scope + deliverable | Rule 2 | All required columns present |
| Behavior ranges reference valid IDs | Rule 3 | Ranges fall within spec `behaviors/` |
| Exit criteria are measurable | Rule 4 | No vague language ("works well", "good quality") |
| No dependency cycles | Rule 5 | Topological sort succeeds |
| Product milestones reference valid containers | Rule 6 | All container IDs exist |
| External deps specify blocking container | Rule 7 | Blocking column is non-empty |
| Research sources resolve to files | Rule 8 | File paths exist on disk |
