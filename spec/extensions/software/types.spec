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
  verify unit "SoftwareBehavior schema is valid"
}

type SoftwareInvariant {
  guarantee      string
  enforced_by    EntityId[]       @optional
  risk           RiskLevel        @optional
  verify unit "SoftwareInvariant schema is valid"
}

type SoftwareFeature {
  behaviors      EntityId[]
  problem        string           @optional
  solution       string           @optional
  verify unit "SoftwareFeature schema is valid"
}

type SoftwareEvent {
  trigger        EntityId
  channel        string           @optional
  payload        EntityId         @optional
  consumers      EntityId[]       @optional
  sync           SyncBlock        @optional
  verify unit "SoftwareEvent schema is valid"
}

type SoftwareTypeDef {
  kind           TypeDefKind      @optional
  fields         TypeFieldDef[]   @optional
  verify unit "SoftwareTypeDef schema is valid"
}

type SoftwarePort {
  direction      PortDirection
  category       string           @optional
  methods        PortOperation[]  @optional
  verify unit "SoftwarePort schema is valid"
}

// ── Formal Methods Types ─────────────────────────────────────

type ContractCondition {
  name           string           @readonly
  description    string
  verify unit "ContractCondition schema is valid"
}

type RequiresBlock {
  conditions     ContractCondition[]
  verify unit "RequiresBlock schema is valid"
}

type EnsuresBlock {
  conditions     ContractCondition[]
  verify unit "EnsuresBlock schema is valid"
}

type MaintainsBlock {
  conditions     ContractCondition[]
  verify unit "MaintainsBlock schema is valid"
}

type SyncBlock {
  barrier        EntityId[]       @optional
  timeout        string           @optional
  verify unit "SyncBlock schema is valid"
}

type PortOperation {
  name           string           @readonly
  inputType      string           @optional
  outputType     string           @optional
  requires       RequiresBlock    @optional
  ensures        EnsuresBlock     @optional
  verify unit "PortOperation schema is valid"
}

type RefinementChain {
  abstractId     EntityId         @readonly
  concreteIds    EntityId[]
  depth          integer
  verify unit "RefinementChain schema is valid"
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
  verify unit "TypeFieldDef schema is valid"
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
  verify unit "ProofObligation schema is valid"
}

type ConcurrencyAnalysisReport {
  deadlock_count     integer
  livelock_risks     integer
  unmatched_count    integer
  channel_mismatches integer
  timed_out          boolean          @optional
  verify unit "ConcurrencyAnalysisReport schema is valid"
}

type ProofObligationKind = contract_preservation | invariant_preservation | refinement_correctness

type ProofDischargeStatus = pending | auto_proved | test_verified

// ── Progressive Formality Levels (RES-25) ────────────────────

type FormalityLevel = prose | entity_graph | contracts | invariants | proofs
