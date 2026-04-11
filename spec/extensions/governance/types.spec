// @specforge/governance extension types — entity kind shapes

use "types/core"

// ── Entity Kind Shapes ───────────────────────────────────────

type GovernanceDecision {
  status       string @optional
  date         string @optional
  context      string
  decision     string
  consequences string[] @optional
  invariants   EntityId[] @optional

  verify property "GovernanceDecision"
}

type GovernanceConstraint {
  category   string @optional
  priority   string
  metric     string
  constrains EntityId[] @optional
  protects   EntityId[] @optional

  verify property "GovernanceConstraint"
}

type GovernanceFailureMode {
  invariant       EntityId
  severity        integer
  occurrence      integer
  detection       integer
  rpn             integer
  cause           string
  effect          string
  mitigation      string
  post_mitigation PostMitigation @optional

  verify property "GovernanceFailureMode"
}

type PostMitigation {
  severity   integer
  occurrence integer
  detection  integer
  rpn        integer

  verify property "PostMitigation"
}

// ── Enums ────────────────────────────────────────────────────


type ConstraintPriority = must | should | may


type ConstraintCategory = performance | security | reliability | compatibility | usability | portability


type DecisionStatus = proposed | accepted | deprecated | superseded
