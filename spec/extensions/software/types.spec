// @specforge/software extension types — entity kind shapes + formal methods

use "types/zero-entity-core"
use "types/core"
// ── Entity Kind Shapes ───────────────────────────────────────

type SoftwareBehavior {
  contract       string
  invariants     EntityId[]       @optional
  types          EntityId[]       @optional
  ports          EntityId[]       @optional
  produces       EntityId[]       @optional
  consumers      EntityId[]       @optional
  category       BehaviorCategory @optional
  abstract       boolean          @optional
  refines        EntityId         @optional
  requires       RequiresBlock    @optional
  ensures        EnsuresBlock     @optional
  maintains      MaintainsBlock   @optional
  tests          string[]         @optional
  gherkin        string[]         @optional
}

type SoftwareInvariant {
  guarantee      string
  enforced_by    EntityId[]       @optional
  risk           RiskLevel        @optional
}

type SoftwareFeature {
  behaviors      EntityId[]
  problem        string           @optional
  solution       string           @optional
}

type SoftwareEvent {
  trigger        EntityId
  channel        string           @optional
  payload        EntityId         @optional
  consumers      EntityId[]       @optional
  sync           SyncBlock        @optional
}

type SoftwareTypeDef {
  kind           TypeDefKind      @optional
  fields         TypeFieldDef[]   @optional
}

type SoftwarePort {
  direction      PortDirection
  category       string           @optional
  methods        PortOperation[]  @optional
}

// ── Formal Methods Types ─────────────────────────────────────

type ContractCondition {
  name           string           @readonly
  description    string
}

type RequiresBlock {
  conditions     ContractCondition[]
}

type EnsuresBlock {
  conditions     ContractCondition[]
}

type MaintainsBlock {
  conditions     ContractCondition[]
}

type SyncBlock {
  barrier        EntityId[]       @optional
  timeout        string           @optional
}

type PortOperation {
  name           string           @readonly
  inputType      string           @optional
  outputType     string           @optional
  requires       RequiresBlock    @optional
  ensures        EnsuresBlock     @optional
}

type RefinementChain {
  abstractId     EntityId         @readonly
  concreteIds    EntityId[]
  depth          integer
}

// ── Enums ────────────────────────────────────────────────────

type RiskLevel = low | medium | high | critical

type PortDirection = inbound | outbound

type FieldAnnotation = readonly | unique | optional | literal

type BehaviorCategory = command | query | handler | saga | projection

type TypeDefKind = struct | union | enum

type TypeFieldDef {
  name           string           @readonly
  fieldType      string
  annotations    FieldAnnotation[] @optional
  refined        string           @optional
}

// Software-specific verify kinds. These are declared in the manifest's
// allowedVerifyKinds per entity kind, not as a core type override.
// Core VerifyKind (types/core.spec) remains an open string.
type SoftwareVerifyKind = unit | integration | property | load | e2e
                        | contract | refinement | trace | deadlock_free | liveness | mutation

// ── Proof Obligation Types ───────────────────────────────────

type ProofObligation {
  entityId       EntityId         @readonly
  kind           ProofObligationKind
  description    string
  status         ProofDischargeStatus
  dischargedBy   string           @optional
}

type ConcurrencyAnalysisReport {
  deadlock_count     integer
  livelock_risks     integer
  unmatched_count    integer
  channel_mismatches integer
  timed_out          boolean          @optional
}

type ProofObligationKind = contract_preservation | invariant_preservation | refinement_correctness

type ProofDischargeStatus = pending | auto_proved | test_verified

// ── Progressive Formality Levels (RES-25) ────────────────────

type FormalityLevel = prose | entity_graph | contracts | invariants | proofs
