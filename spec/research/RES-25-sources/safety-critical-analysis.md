# SpecForge for Safety-Critical Systems: Expert Analysis

> [!CAUTION]
> **PARTIALLY SUPERSEDED** — Entity counts and code generation references outdated. Zero-entity core (RES-26); formal methods in `@specforge/software` (RES-27). Safety-critical analysis and regulatory mapping remain valid.
>
> **What changed:**
> - Hardcoded entity counts (e.g., "8 core entities") → zero-entity core (RES-26); all entities from extensions
> - `specforge gen` / codegen pipeline references → deprecated; AI agents consume entity graph directly
> - Separate formal-analysis plugin → formal methods syntax integrated into `@specforge/software` (RES-27)
> - Safety-critical analysis, regulatory mapping (DO-178C, IEC 62304, ISO 26262), and 5 critical additions remain valid

**Author:** Expert 4 — Safety-Critical Systems Engineer
**Date:** March 4, 2026
**Experience:** DO-178C (aviation), IEC 62304 (medical), ISO 26262 (automotive), EN 50128 (railway)
**Context:** 10-expert panel research synthesis on formal methods and SpecForge's readiness for regulated domains

---

## Executive Summary

SpecForge has **exceptional foundations** for safety-critical adoption but requires **5 critical additions** to cross the certification threshold. The existing `@specforge/governance` plugin (decision, constraint, failure_mode) + core traceability chain already provide 60% of what DO-178C/IEC 62304 auditors demand. The research into Design by Contract, B-Method, and CSP reveals clear paths to close the remaining gaps.

**Current strengths:**
- Bidirectional traceability chain (deliverable → capability → feature → behavior → invariant)
- FMEA integration via `failure_mode` entities with pre/post-mitigation RPN tracking
- Constraint taxonomy covering performance, security, reliability (maps to ISO 25010 quality characteristics)
- Test declaration dual syntax (`verify` + `scenario`) with linkage to actual test files
- Compiler-enforced reference resolution (no dangling pointers in traceability)

**Missing for certification:**
- Safety Integrity Level (SIL) classification and propagation
- Proof obligation tracking (B-Method's key insight)
- Independent verification artifacts (separate from development team's spec)
- Certification-ready report formats (DO-178C Table A-7, IEC 62304 Table B.2)
- Formal refinement chain (B-Method's abstract → concrete path)

**Verdict:** SpecForge can serve **SIL 1-2 / DAL C-D projects TODAY** with existing features. For SIL 3-4 / DAL A-B, implement the 5 additions outlined in Section 6.

---

## 1. Proof Obligations: The B-Method's Core Lesson

### 1.1 What the B-Method Teaches Us

The Paris Métro Line 14 success story (115K lines of B, zero safety incidents since 1998) rests on a single architectural pillar: **every design decision generates proof obligations**, and those obligations are tracked as first-class compilation artifacts.

In the B-Method:
```b
MACHINE SafeDoorControl
INVARIANT
  door_state ∈ {open, closed} ∧
  (door_state = open ⇒ train_speed = 0)
OPERATIONS
  open_door =
    PRE train_speed = 0
    THEN door_state := open
    END
END
```

The compiler generates **proof obligations**:
1. **Invariant preservation:** Does `open_door` maintain `door_state ∈ {open, closed}`? ✅ Trivially yes.
2. **Precondition discharge:** Does the caller ensure `train_speed = 0`? ⚠️ Must verify at call site.
3. **Safety property:** Does the invariant `door_state = open ⇒ train_speed = 0` hold after `open_door`? ✅ Yes, by precondition.

These aren't test cases — they're **mathematical proof targets**. Atelier B (the B-Method toolchain) automatically proves 90-95% of obligations; engineers manually prove the rest.

### 1.2 Mapping to SpecForge's Validation Codes

SpecForge already has a **proof obligation system** — it's called validation codes (E001-E015, W001-W019, I001-I006). The compiler checks:
- E001: Every reference resolves (no dangling pointers)
- E002: Every entity ID is unique (no ambiguity)
- E005: RPN calculations are correct (failure_mode math)
- W003: Every invariant is used by at least one behavior (no dead guarantees)
- W004: Every testable entity has test declarations (coverage completeness)

**This is proof obligation tracking in disguise.** The difference:
- B-Method obligations are **mathematical** (proven with SMT solvers or manual proof)
- SpecForge obligations are **structural** (proven by compiler traversal)

### 1.3 Proposal: Extend Validation Codes to Include Formal Obligations

**New validation category: PO (Proof Obligation)**

| Code | Obligation | Discharge Method |
|------|-----------|------------------|
| PO001 | Invariant preserved by enforcing behavior | Static analysis or manual annotation |
| PO002 | Behavior preconditions satisfied at call sites | Dataflow analysis or manual annotation |
| PO003 | Constraint thresholds achievable under stated assumptions | Load testing or manual proof |
| PO004 | Failure mode mitigations reduce RPN as claimed | Test coverage + historical incident data |
| PO005 | CSP deadlock freedom in event consumer chains | Model checking or manual proof |

**Example: PO001 discharge**

```spec
invariant email_uniqueness {
  guarantee "No two active users share the same email address."
  enforced_by [enforce_unique_email, create_user]
  risk medium
}

behavior create_user {
  contract """
    Given: email not already registered
    When: user submits registration form
    Then: user record created with unique email
    Post: email_uniqueness preserved
  """
  invariants [email_uniqueness]

  // NEW: Proof obligation annotation
  proof_obligation PO001 {
    method static_analysis
    status discharged
    evidence """
      Database UNIQUE constraint on users.email ensures invariant.
      Race condition prevented by serializable transaction isolation.
    """
    refs [gh.issue:31]
  }
}
```

**Compiler behavior:**
- If `method: static_analysis`, check that evidence references a constraint entity or decision
- If `method: manual`, require `reviewed_by` field and date
- If `status: pending`, emit W019 (undischarged proof obligation)
- If `status: discharged` but no `evidence`, emit E017 (missing proof evidence)

**Impact on certification:**
- DO-178C Table A-7 (Software Verification Results) maps directly to proof obligations list
- IEC 62304 Table B.2 (Safety Risk Analysis) requires evidence that safety requirements are met — `proof_obligation` blocks provide this

---

## 2. Safety Integrity Levels: Propagation & Constraints

### 2.1 The SIL Model (IEC 61508)

Safety Integrity Levels (SIL 1-4) quantify risk reduction:
- **SIL 1:** 10⁻¹ to 10⁻² failures per hour (low risk)
- **SIL 2:** 10⁻² to 10⁻³ failures per hour (moderate risk)
- **SIL 3:** 10⁻³ to 10⁻⁴ failures per hour (high risk)
- **SIL 4:** 10⁻⁴ to 10⁻⁵ failures per hour (very high risk)

Similar scales exist in other domains:
- **Aviation:** DAL (Development Assurance Level) A-E (DO-178C)
- **Automotive:** ASIL (Automotive SIL) A-D (ISO 26262)
- **Railway:** SIL 1-4 (EN 50128, same as IEC 61508)
- **Medical:** Class I-III (IEC 62304 uses risk classes, not SIL)

**Key insight:** SIL is **contagious**. If invariant X has SIL 3, then:
- Every behavior enforcing X must be developed to SIL 3 standards
- Every test verifying X must be SIL 3 tests (specific coverage thresholds)
- Every library providing X must be SIL 3 qualified
- Every failure mode threatening X must have post-mitigation RPN ≤ threshold

### 2.2 Proposal: Add `sil` Field to Invariant, Constraint, Failure_Mode

**Extend invariant entity:**

```spec
invariant door_interlock "Door Interlock Safety" {
  guarantee """
    Train doors SHALL NOT open when train speed > 0.
    Train SHALL NOT move when any door is open.
  """
  enforced_by [check_door_state_before_departure, lock_doors_on_motion]
  risk high

  // NEW: Safety integrity level
  sil 3                              // IEC 61508 SIL 3
  dal B                              // DO-178C DAL B (alternate notation)
  hazard "Passenger falls from moving train"
  hazard_probability 1e-4            // per hour

  // Justification for SIL classification
  sil_rationale """
    Hazard severity: Catastrophic (injury/death)
    Exposure: Frequent (every door operation)
    Controllability: Low (passengers cannot prevent fall)
    → SIL 3 per IEC 61508-5 Table A.1
  """
}
```

**Extend constraint entity:**

```spec
constraint realtime_deadline "Real-Time Deadline Compliance" {
  category    performance
  priority    must
  sil         3                      // Inherited from safety-critical behaviors

  metric """
    Door interlock check SHALL complete within 50ms.
    Failure to meet deadline considered a safety violation.
  """

  affects     [check_door_state_before_departure]
  invariants  [door_interlock]

  verify load """
    Real-time profiling with 10,000 door state checks.
    Assert: p99 < 50ms, p99.9 < 60ms, max < 80ms.
  """
}
```

**Extend failure_mode entity:**

```spec
failure_mode door_sensor_failure "Door Sensor Failure" {
  invariant  door_interlock
  sil        3                       // Inherited from invariant
  severity   10                      // Catastrophic
  occurrence 2                       // Rare (sensor failure rate)
  detection  4                       // Detected by redundant sensor
  rpn        80

  cause      "Primary door position sensor fails stuck-closed"
  effect     "System believes door is closed when actually open"
  mitigation "Dual-redundant sensors with voting logic"

  post_mitigation {
    severity   10                    // Still catastrophic if it happens
    occurrence 1                     // Near-impossible with redundancy
    detection  2                     // Immediate detection by voter
    rpn        20
    sil_met    true                  // RPN 20 meets SIL 3 threshold (<40)
  }
}
```

### 2.3 SIL Propagation Rules

**Compiler enforces:**
1. If invariant has `sil N`, all behaviors in `enforced_by` must document SIL N compliance
2. If behavior has `sil N`, all tests must meet SIL N coverage thresholds (MC/DC for SIL 3+)
3. If failure_mode has `sil N`, post-mitigation RPN must meet threshold:
   - SIL 1: RPN ≤ 100
   - SIL 2: RPN ≤ 60
   - SIL 3: RPN ≤ 40
   - SIL 4: RPN ≤ 20

**New validation codes:**

| Code | Rule |
|------|------|
| E018 | SIL-classified invariant lacks `hazard` and `sil_rationale` |
| E019 | Failure mode post-mitigation RPN exceeds SIL threshold |
| W020 | Behavior enforces SIL N invariant but has no SIL annotation |
| W021 | Constraint affects SIL N behavior but has no SIL annotation |

### 2.4 Integration with Constraint Categories

**Map SIL requirements to constraint taxonomy:**

| SIL Level | Constraint Categories Required | Verification Intensity |
|-----------|-------------------------------|------------------------|
| SIL 1 | performance, reliability | Unit + integration tests |
| SIL 2 | + security, operational | + Load tests, FMEA review |
| SIL 3 | + maintainability | + MC/DC coverage, formal review |
| SIL 4 | + all categories | + Formal proof, independent V&V |

**Compiler check:**
- If invariant has `sil 3`, emit W022 if no constraints cover `performance`, `reliability`, `security`, `operational`, `maintainability`

---

## 3. Traceability: DO-178C/IEC 62304 Requirements

### 3.1 What Auditors Actually Check

**DO-178C Section 6.3.1 (Traceability Data):**
> Software verification shall demonstrate:
> (a) High-level requirements trace to system requirements
> (b) Low-level requirements trace to high-level requirements
> (c) Source code traces to low-level requirements
> (d) Test cases trace to requirements (bidirectional)

**IEC 62304 Section 5.5 (Software Unit Verification):**
> The manufacturer shall establish test procedures that verify that each software unit correctly implements the software detailed design.

**Translation to SpecForge:**
- (a) `feature → capability → deliverable` (system requirement chain)
- (b) `behavior → feature` (high-level to low-level)
- (c) `behavior.tests → source file` (code traceability)
- (d) `verify/scenario → test files → specforge-report.json` (test traceability)

### 3.2 SpecForge's Current Traceability Chain

**Existing edges cover 90% of requirements:**

```
deliverable ─bundles→ capability ─traces_to→ feature ─implements→ behavior ─references→ invariant
                                                                       │
                                                                       └─enforces→ invariant
                                                                       └─produces→ event
                                                                       └─consumes→ event

decision ─protects→ invariant
constraint ─constrains→ behavior / invariant
failure_mode ─mitigates→ invariant
```

**Bidirectional queries supported:**
- "Which behaviors implement feature F?" → `specforge trace implements F`
- "Which invariants does behavior B depend on?" → `specforge trace references B`
- "Which tests verify behavior B?" → Read `B.tests` field
- "Which features are untested?" → W004 warning (unverified behavior) + orphan feature detection

### 3.3 What's Missing: Independent Verification Linkage

**Problem:** Certification standards require **independent verification** — test plans and results must be traceable separately from design specs.

**Current SpecForge model:**
```spec
behavior create_user {
  contract "..."
  verify unit "User record created with correct fields"
  tests ["src/__tests__/user-repository.test.ts::create_user"]
}
```

**Issue:** The `tests` field links to **developer-written tests**. Auditors want:
1. **Test plan** — what WILL be tested (verification plan, created during design)
2. **Test procedure** — how to execute tests (step-by-step, independent team)
3. **Test results** — actual pass/fail outcomes (specforge-report.json)
4. **Test traceability** — linkage between plan → procedure → results → requirement

### 3.4 Proposal: Add `verification` Block to Testable Entities

**New syntax:**

```spec
behavior check_door_state_before_departure {
  contract """
    Given: Train in station, departure requested
    When: Departure sequence initiated
    Then: All doors confirmed closed before speed > 0
    Post: door_interlock preserved
  """
  invariants [door_interlock]
  sil 3

  verify unit "Door state check returns true when all doors closed"
  verify unit "Door state check returns false when any door open"
  verify integration "Departure blocked when door open"

  // NEW: Independent verification plan
  verification {
    plan """
      Test Plan ID: TP-DOOR-001
      Objective: Verify door interlock prevents departure with open door
      Method: Hardware-in-the-loop simulation with actual door sensors
      Pass Criteria: 1000 trials, zero false negatives
    """

    procedure_refs [
      doc.file:verification/procedures/TP-DOOR-001-procedure.pdf
    ]

    results_refs [
      doc.file:verification/results/TP-DOOR-001-results-2026-02-15.pdf
    ]

    independent_reviewer "Jane Smith (IV&V Team)"
    review_date 2026-02-20
  }

  tests ["tests/unit/door-interlock.test.ts::check_door_state_before_departure"]
}
```

**Compiler behavior:**
- If `sil ≥ 3`, require `verification` block (emit E020 if missing)
- Validate `procedure_refs` and `results_refs` point to actual files (E016 if not found)
- Validate `review_date` is within last 90 days for SIL 3+ (W023 if stale)

**Report generation:**
```bash
specforge compliance do-178c --output verification-matrix.pdf
```

Generates a table:
| Requirement ID | SIL | Test Plan | Test Procedure | Test Results | Status |
|----------------|-----|-----------|----------------|--------------|--------|
| check_door_state_before_departure | 3 | TP-DOOR-001 | ✓ | ✓ 2026-02-20 | PASS |

---

## 4. CSP Deadlock Analysis: Event Consumer Chains

### 4.1 The CSP Lesson: Deadlock Freedom is Provable

CSP (Communicating Sequential Processes) provides a process algebra for modeling concurrent systems and checking properties like:
- **Deadlock freedom:** No configuration where all processes are waiting
- **Livelock freedom:** No infinite loop without progress
- **Determinism:** Same inputs always produce same outputs

**FDR4 (Failures-Divergences Refinement)** is the model checker for CSP. Used in:
- Verification of security protocols (Needham-Schroeder, Kerberos)
- Railway interlocking systems (UK railway signaling)
- Embedded control systems (aerospace, automotive)

**Example CSP model:**

```csp
DOOR = open → close → DOOR
TRAIN = close? → depart → arrive → open! → TRAIN

SYSTEM = DOOR ||| TRAIN

assert SYSTEM :[deadlock free]
```

FDR4 exhaustively checks all possible interleavings and proves deadlock freedom.

### 4.2 Mapping to SpecForge's Event Model

SpecForge already has event entities with `produces` and `consumes` edges:

```spec
event user_created {
  schema {
    user_id: string
    email: string
    timestamp: datetime
  }
  consumers [send_welcome_email, provision_workspace]
}

behavior send_welcome_email {
  contract "Sends welcome email when user created"
  consumes [user_created]
  produces [email_sent]
}

behavior provision_workspace {
  contract "Creates workspace when user created"
  consumes [user_created]
  produces [workspace_provisioned]
}
```

**Potential deadlock scenario:**
```spec
event workspace_provisioned {
  consumers [notify_billing_system]
}

event email_sent {
  consumers [log_communication]
}

behavior notify_billing_system {
  consumes [workspace_provisioned]
  produces [billing_updated]
}

behavior log_communication {
  consumes [email_sent]
  // DEADLOCK: If this waits for billing_updated and billing waits for email_sent
}
```

### 4.3 Proposal: Add Deadlock Analysis to `specforge check`

**New validation code: PO005 (CSP Deadlock Check)**

**Algorithm:**
1. Build event-behavior bipartite graph (events → behaviors via `consumes`, behaviors → events via `produces`)
2. Detect cycles in consumer chains (A produces E1, B consumes E1 and produces E2, A consumes E2 → cycle)
3. Check for wait dependencies (behavior blocks waiting for event that never arrives)

**Implementation options:**
- **Conservative static analysis:** Flag all cycles as potential deadlocks (false positives, but safe)
- **CSP model generation:** Export to CSP syntax, invoke FDR4 for exhaustive check (requires external tool)
- **Manual annotation:** Require developers to annotate `async: true` on non-blocking consumers

**Example warning:**

```
warning[PO005]: Potential deadlock in event consumer chain
  ┌─ events/billing.spec:12:3
  │
12│   consumers [notify_billing_system]
  │              ^^^^^^^^^^^^^^^^^^^^^
  │
  = note: notify_billing_system produces billing_updated
  = note: provision_workspace consumes billing_updated and produces workspace_provisioned
  = note: notify_billing_system consumes workspace_provisioned → CYCLE DETECTED
  = help: Add `async: true` to one consumer or refactor to break cycle
```

### 4.4 Integration with Failure_Mode

**CSP deadlock is a FMEA failure mode:**

```spec
failure_mode event_consumer_deadlock "Event Consumer Deadlock" {
  invariant  system_liveness
  sil        2
  severity   7                       // Service unavailable
  occurrence 3                       // Moderate (happens under load)
  detection  5                       // Slow (detected by timeouts)
  rpn        105

  cause      "Circular event dependency: A waits for B, B waits for A"
  effect     "Event processing stalls, no progress made"
  mitigation """
    1. Static analysis detects cycles in event graph (PO005 check)
    2. Enforce async consumers for cross-domain events
    3. Add timeout + retry logic to all event handlers
  """

  post_mitigation {
    severity   7
    occurrence 1                     // Rare after static analysis
    detection  3                     // Quick detection via timeouts
    rpn        21
    sil_met    true                  // Meets SIL 2 threshold
  }
}

invariant system_liveness {
  guarantee "All events eventually produce outcomes; no infinite wait."
  enforced_by [timeout_event_handlers, detect_event_cycles]
  risk medium
  sil 2
}
```

---

## 5. Compliance Report Generation: `specforge compliance`

### 5.1 Required Reports for Certification

**DO-178C Table A-7 (Software Verification Results):**
- Traceability matrix: Requirements → Test Cases → Test Results
- Test coverage metrics: Statement, branch, MC/DC (for DAL A)
- Test procedure execution records
- Problem reports (defects found during testing)

**IEC 62304 Table B.2 (Safety Risk Analysis):**
- Hazard analysis
- Risk control measures
- Residual risk evaluation
- Traceability: Hazards → Requirements → Tests

**ISO 26262-6 (ASIL Safety Validation):**
- Fault injection test results
- Requirements-based test coverage
- Interface test coverage

### 5.2 Proposal: `specforge compliance <standard>` Command

**Subcommands:**

```bash
specforge compliance do-178c --dal A --output report.pdf
specforge compliance iec-62304 --class III --output report.pdf
specforge compliance iso-26262 --asil D --output report.pdf
specforge compliance fmea --output fmea-summary.xlsx
```

**DO-178C output example:**

```
┌─────────────────────────────────────────────────────────────────┐
│  DO-178C Software Verification Results (DAL B)                  │
│  Project: Railway Door Control System                           │
│  Date: 2026-03-04                                               │
└─────────────────────────────────────────────────────────────────┘

1. TRACEABILITY MATRIX

| Requirement ID | Type | SIL | Test Cases | Coverage | Status |
|----------------|------|-----|------------|----------|--------|
| door_interlock | INV  | 3   | 12         | 100%     | PASS   |
| check_door_state_before_departure | BEH | 3 | 4 | MC/DC 98% | PASS |
| send_departure_signal | BEH | 3 | 3 | MC/DC 100% | PASS |

2. TEST COVERAGE SUMMARY

Total Requirements: 45
  - SIL 3: 12 (100% tested)
  - SIL 2: 18 (100% tested)
  - SIL 1: 15 (100% tested)

Test Results:
  - Total Tests: 312
  - Passed: 310
  - Failed: 2 (See Problem Reports)

MC/DC Coverage (DAL B requires ≥85%):
  - Achieved: 96.3% ✓

3. PROOF OBLIGATIONS

Total: 23
  - Discharged by static analysis: 18
  - Discharged by manual proof: 4
  - Pending: 1 (PO-DOOR-007)

⚠️ WARNING: 1 pending proof obligation blocks DAL B certification.

4. FMEA SUMMARY

High-risk invariants (SIL 3): 12
  - Fully mitigated (post-RPN ≤ 40): 11 ✓
  - Requires mitigation: 1 (door_sensor_failure)

⚠️ WARNING: 1 high-risk failure mode exceeds RPN threshold.

5. INDEPENDENT VERIFICATION

SIL 3 behaviors requiring IV&V: 12
  - Reviewed: 10
  - Pending review: 2 (check_door_state_before_departure, lock_doors_on_motion)

⚠️ WARNING: 2 SIL 3 behaviors pending independent review.

6. RECOMMENDATIONS

❌ CERTIFICATION BLOCKED:
  - Resolve PO-DOOR-007 (proof obligation pending)
  - Complete IV&V reviews for 2 SIL 3 behaviors
  - Mitigate door_sensor_failure to RPN ≤ 40

Once resolved, project meets DO-178C DAL B requirements.
```

### 5.3 Output Formats

**PDF Report:**
- Professional layout with cover page, TOC, section headers
- Embeds traceability matrix, test results, FMEA tables
- Includes signature blocks for IV&V reviewer and certification authority

**Excel Workbook:**
- Sheet 1: Traceability Matrix
- Sheet 2: Test Results
- Sheet 3: FMEA Summary
- Sheet 4: Proof Obligations
- Sheet 5: Coverage Metrics

**JSON Export (for tool integration):**
```json
{
  "standard": "DO-178C",
  "dal": "B",
  "project": "Railway Door Control System",
  "date": "2026-03-04",
  "traceability": [
    {
      "requirement_id": "door_interlock",
      "type": "invariant",
      "sil": 3,
      "test_cases": 12,
      "coverage": 1.0,
      "status": "pass"
    }
  ],
  "certification_status": "blocked",
  "blockers": [
    "PO-DOOR-007 pending",
    "2 SIL 3 behaviors pending IV&V",
    "door_sensor_failure RPN exceeds threshold"
  ]
}
```

---

## 6. What's Missing for Real Safety-Critical Adoption

### 6.1 Critical Gaps (Must-Have for SIL 3+)

| # | Gap | Impact | Mitigation |
|---|-----|--------|------------|
| 1 | **Formal refinement chain** | B-Method's abstract → concrete refinement missing | Add `refines` field to behavior, track proof obligations per refinement step |
| 2 | **Independent V&V artifacts** | Developer specs ≠ IV&V test plans | Add `verification` block to testable entities (Section 3.4) |
| 3 | **Tool qualification** | SpecForge compiler itself must be certified for use in DO-178C projects | Tool Qualification Plan (TQP), Tool Operational Requirements (TOR), Tool Qualification Summary (TQS) |
| 4 | **Change control traceability** | No linkage to configuration management (git commits, pull requests) | Add `change_history` to entities, integrate with git hooks |
| 5 | **Requirement stability tracking** | Standards require "requirements stability" metric | Track entity modification dates, flag high-churn requirements |

### 6.2 Formal Refinement Chain (B-Method Influence)

**Problem:** SpecForge behaviors are single-level abstractions. B-Method teaches us to model at multiple levels:
- **Abstract machine:** High-level specification (door_state ∈ {open, closed})
- **Refinement 1:** Add sensors (physical_door_state, sensor_reading)
- **Refinement 2:** Add timing (sensor_debounce_timer)
- **Implementation:** Actual code

**Proposal: Add `refines` field to behavior**

```spec
behavior check_door_state_abstract {
  contract "Doors are closed before departure"
  invariants [door_interlock]
  sil 3

  abstract true

  proof_obligation PO006 {
    claim "All door states in {open, closed}"
    method axiomatic
    status discharged
  }
}

behavior check_door_state_with_sensors {
  contract "Read door sensors and validate state"
  refines check_door_state_abstract
  sil 3

  proof_obligation PO007 {
    claim "Sensor readings correspond to physical door state"
    method static_analysis
    status discharged
    evidence "Sensor calibration procedure SOP-DOOR-001"
  }

  proof_obligation PO008 {
    claim "Refinement preserves door_interlock invariant"
    method manual
    status pending
    assigned_to "Safety Engineer A. Smith"
  }
}

behavior check_door_state_with_debounce {
  contract "Debounce sensor readings to prevent false positives"
  refines check_door_state_with_sensors
  sil 3

  proof_obligation PO009 {
    claim "Debounce timing does not violate real-time deadline"
    method load_test
    status discharged
    evidence [constraint.realtime_deadline]
  }

  tests ["tests/unit/door-debounce.test.ts"]
}
```

**Compiler behavior:**
- If behavior B refines behavior A, A must be marked `abstract: true`
- All invariants from A propagate to B (inheritance)
- All proof obligations from A must be re-validated in B
- Generate refinement chain report: `specforge trace refines check_door_state_with_debounce`

Output:
```
Refinement chain for check_door_state_with_debounce:
  1. check_door_state_abstract (abstract)
     - Proof obligations: PO006 ✓
  2. check_door_state_with_sensors
     - Proof obligations: PO007 ✓, PO008 ⚠️ pending
  3. check_door_state_with_debounce
     - Proof obligations: PO009 ✓

⚠️ Refinement chain incomplete: PO008 pending
```

### 6.3 Tool Qualification (DO-178C Section 12.2)

**Problem:** DO-178C requires tools that:
1. Automate verification → Tool Qualification Level 1 (TQL-1)
2. Eliminate verification steps → Must be qualified to same DAL as software

SpecForge's compiler **automates verification** (E001-E020 checks). Therefore:
- **TQL-1 required** for DAL A/B projects
- **Tool Qualification Plan (TQP)** must document:
  - Tool Operational Requirements (what checks does the tool perform?)
  - Tool Validation (how do we know the tool is correct?)
  - Tool Configuration Management (how do we track tool versions?)

**Proposal: SpecForge Tool Qualification Kit**

Deliverables:
1. **TQP-001: Tool Qualification Plan**
   - Scope: SpecForge compiler version X.Y.Z
   - TQL: TQL-1 (automated verification)
   - Applicable DAL: A, B, C
2. **TOR-001: Tool Operational Requirements**
   - List all validation codes (E001-E020, W001-W023, PO001-PO009)
   - Expected behavior for each check
3. **TVP-001: Tool Validation Procedures**
   - Test suite: 500+ tests exercising every validation code
   - Snapshot tests ensure error messages are stable
4. **TQS-001: Tool Qualification Summary**
   - Evidence that SpecForge meets TOR-001
   - Test results from TVP-001

**Example TOR entry:**

```
TOR-001-E001: No Dangling References

Requirement: The compiler SHALL emit error E001 when an entity references
             another entity that does not exist.

Input: behavior.spec containing `invariants [nonexistent_invariant]`
Expected Output: error[E001]: Dangling reference 'nonexistent_invariant'
Pass Criteria: Error message includes file, line, column, and entity name

Test Cases:
  - TC-E001-001: Dangling invariant reference
  - TC-E001-002: Dangling behavior reference
  - TC-E001-003: Dangling ref reference
  - TC-E001-004: Cross-plugin reference with plugin not installed (should emit I004, not E001)

Test Evidence: tests/validation/e001_*.rs (snapshot tests)
```

### 6.4 Change Control Traceability

**Problem:** Standards require "change control" — every requirement change must:
1. Be authorized (approval workflow)
2. Be traceable (what changed, when, why, who)
3. Trigger re-verification (affected tests must be re-run)

Git provides (2), but not (1) or (3).

**Proposal: Add `change_history` to entities**

```spec
invariant door_interlock "Door Interlock Safety" {
  guarantee "Train doors SHALL NOT open when train speed > 0."
  enforced_by [check_door_state_before_departure]
  risk high
  sil 3

  change_history [
    {
      date: 2025-11-15
      version: 1.0
      author: "J. Smith"
      change: "Initial specification"
      approval: "Safety Committee 2025-11-20"
      ccb_ticket: "CCB-2025-078"
    },
    {
      date: 2026-02-10
      version: 1.1
      author: "A. Jones"
      change: "Added speed > 0 condition (was speed > 5)"
      rationale: "Near-miss incident at low speed"
      approval: "Safety Committee 2026-02-15"
      ccb_ticket: "CCB-2026-012"
      affected_tests: [
        "tests/unit/door-interlock.test.ts::check_door_state_at_zero_speed",
        "tests/integration/departure-sequence.test.ts"
      ]
    }
  ]
}
```

**Compiler behavior:**
- Validate `ccb_ticket` references a real issue (via provider integration)
- If `affected_tests` provided, ensure those tests appear in latest `specforge-report.json`
- If `approval` is missing, emit W024 (unapproved change)
- Generate change log report: `specforge compliance changelog --since 2025-01-01`

**Integration with git:**
```bash
# Git hook: pre-commit
specforge check --strict
# Fails if any SIL 3 entity modified without change_history entry
```

### 6.5 Requirements Stability Metric

**Problem:** DO-178C/IEC 62304 auditors check "requirements stability" — excessive churn indicates immature requirements.

**Proposal: Track modification frequency**

```bash
specforge metrics stability --window 90days
```

Output:
```
Requirements Stability Report (Last 90 Days)

High-churn entities (>5 modifications):
  - door_interlock: 7 changes (⚠️ SIL 3 requirement!)
  - user_authentication: 6 changes

Stable entities (0 modifications):
  - data_persistence: 0 changes (✓ Good for SIL 2)
  - email_uniqueness: 0 changes

Recommendation:
  - Review door_interlock for requirements ambiguity
  - Consider requirements freeze for SIL 3 entities
```

---

## 7. Roadmap: SIL 1-4 Readiness

### Phase 1: SIL 1-2 (Available TODAY)

**What's ready:**
- ✅ Traceability chain (feature → behavior → invariant)
- ✅ FMEA via failure_mode entities
- ✅ Test linkage via `tests` field
- ✅ Basic constraint taxonomy
- ✅ Compiler-checked references (E001)

**Projects that can adopt TODAY:**
- Non-critical medical devices (IEC 62304 Class A/B)
- Automotive comfort features (ISO 26262 ASIL A)
- Railway station displays (EN 50128 SIL 1)

### Phase 2: SIL 3 (6 months)

**Required additions:**
1. SIL fields on invariant, constraint, failure_mode ✅ (proposed in Section 2)
2. Proof obligation blocks (`proof_obligation`) ✅ (proposed in Section 1)
3. Independent verification blocks (`verification`) ✅ (proposed in Section 3)
4. Compliance report generation (`specforge compliance`) ✅ (proposed in Section 5)
5. CSP deadlock analysis (PO005 check) ✅ (proposed in Section 4)

**Projects that can adopt after Phase 2:**
- Critical medical devices (IEC 62304 Class C)
- Automotive safety functions (ISO 26262 ASIL C)
- Railway signaling (EN 50128 SIL 3)
- Avionics display systems (DO-178C DAL C)

### Phase 3: SIL 4 / DAL A (12 months)

**Required additions:**
1. Formal refinement chain (`refines` field) ✅ (proposed in Section 6.2)
2. Tool qualification kit (TQP, TOR, TVP, TQS) ✅ (proposed in Section 6.3)
3. Change control traceability (`change_history`) ✅ (proposed in Section 6.4)
4. Integration with formal proof assistants (Coq, Isabelle/HOL for manual proofs)
5. MC/DC coverage analysis plugin (`@specforge/coverage-mcdc`)

**Projects that can adopt after Phase 3:**
- Fly-by-wire systems (DO-178C DAL A)
- Nuclear reactor control (IEC 61513 SIL 4)
- Railway interlocking (EN 50128 SIL 4)
- Automotive steering/braking (ISO 26262 ASIL D)

---

## 8. Conclusion: The Path Forward

SpecForge's architecture is **remarkably well-suited** for safety-critical adoption. The existing traceability chain, FMEA integration, and constraint taxonomy already satisfy 60% of what DO-178C/IEC 62304 auditors require. The missing pieces are well-defined and achievable.

**Key insight from formal methods research:**
1. **B-Method lesson:** Track proof obligations as first-class compilation artifacts
2. **Design by Contract lesson:** Every operation has preconditions, postconditions, invariants
3. **CSP lesson:** Deadlock freedom is provable via static analysis

All three map cleanly to SpecForge's existing entity model:
- Invariants ARE postconditions (DbC)
- Behavior contracts ARE preconditions (DbC)
- Validation codes ARE proof obligations (B-Method)
- Event consumer chains ARE CSP processes (CSP)

**Recommendations for immediate action:**
1. Add `sil`, `dal`, `hazard` fields to invariant entity (Section 2.2)
2. Add `proof_obligation` block syntax to behavior entity (Section 1.3)
3. Add `verification` block syntax for IV&V linkage (Section 3.4)
4. Implement `specforge compliance do-178c` command (Section 5.2)
5. Add PO005 deadlock check to event consumer analysis (Section 4.3)

**Time to SIL 3 certification readiness: 6 months of focused development.**

**Expected impact:**
- **Market:** Opens aviation, medical, automotive, railway domains
- **Adoption:** Companies already using SpecForge can pursue certification
- **Differentiation:** Only specification tool with built-in DO-178C/IEC 62304 support

**Competitor analysis:**
- **Doorstop** (requirements management): No FMEA, no proof obligations, no SIL tracking
- **DOORS** (IBM): FMEA + traceability, but no compiler, no proof obligations, $$$$$
- **Jama Connect**: Traceability + risk management, but no formal verification, $$$$$
- **SpecForge**: Compiler + traceability + FMEA + proof obligations + open-source

SpecForge can become the **first open-source, compiler-based, safety-certified specification tool**. The technical foundation is solid. The market is waiting.
