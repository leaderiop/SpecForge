// Judged rule behaviors — a rule that holds while a fresh verdict says so (ADR 0290)

use "invariants/core"
use "ports/inbound"
use "ports/outbound"

defaults behavior {
  features [typed_edits]
  ports    [CompilerApi, FileSystem]
}

behavior parse_verdict_statement "Parse a Verdict Statement" {
  invariants [graph_traversal_integrity]
  category   validation
  contract   """
    A `verdict <rule> <outcome> { … }` statement in an entity's block is the entity's own member, as `verify` is
    (ADR 0290, ADR 0094): word-reserved only where a rule, an outcome and a block follow it. A field named `verdict`
    with no such shape after it stays a field.
  """
  verify unit "a field named verdict stays a field when no rule, outcome and block follow it"
}

behavior judge_rule_by_fresh_verdict "Judge a Rule by a Fresh Verdict" {
  invariants [incremental_correctness]
  category   validation
  contract   """
    A judged rule holds while a fresh verdict from its judge says so (ADR 0290). A project that declares no judged
    rule reads no test report, stamps no file and says neither W168 nor I205.
  """
  verify unit "a project with no judged rule reads no report and stamps no file"
}
