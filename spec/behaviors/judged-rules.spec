// Judged rule behaviors — a rule that holds while a fresh verdict says so (ADR 0290)

use "invariants/core"
use "ports/inbound"
use "ports/outbound"

defaults behavior {
  features [judged_rules]
  ports    [CompilerApi, FileSystem]
}

feature judged_rules "Judged Rules" {
  status     proposed
  priority   high
  problem    """
    Some checks cannot be decided by code: whether a plan covers its requirements, whether a truth holds in the
    code. A model judges them and records its answer nowhere the kernel can see, so an edit made after the judgement
    changes nothing and a status computed from a pass is an agent's claim.
  """
  solution   """
    A rule may be judged: it names a judge, its outcomes and the inputs it reads. The judge's verdict is a `verdict`
    statement in the entity's block that only the host writes, with a digest of every input. The rule holds while
    the verdict is fresh and passing; a changed input makes it stale and the rule fires again.
  """
  acceptance [
    "a judged rule fires while its verdict is missing, unrecorded, stale or failing",
    "a verdict is stale exactly when an input it was judged over changed",
    "a project that declares no judged rule reads and checks as before",
  ]
}

behavior declare_judged_rule "Declare a Judged Rule" {
  invariants [entity_id_uniqueness]
  category   validation
  contract   """
    A rule whose check is `judged` names its judge, its outcomes, the outcomes that pass and, per target kind,
    its named inputs by selector (ADR 0290). A rule that cannot work as declared is W112 and is not registered.
  """
  verify unit "a judged rule is declared with its judge, outcomes and per-target selectors"
  verify unit "a malformed judged rule is W112 and is not registered"
  verify unit "a judge may be a prompt of the rule's own extension, and a missing or a peer's prompt is W112"
  verify unit "every input of the model's judged rules parses and resolves"
  verify unit "a selector prints back to its canonical text"
  verify unit "a verdict-written field is set by exactly one judged rule, and is never required"
  verify unit "an SDK judged rule builds the descriptor the host reads"
}

behavior parse_verdict_statement "Parse a Verdict Statement" {
  invariants [graph_traversal_integrity]
  category   validation
  contract   """
    A `verdict <rule> <outcome> { … }` statement in an entity's block is the entity's own member, as `verify` is
    (ADR 0290, ADR 0094): word-reserved only where a rule, an outcome and a block follow it. A field named `verdict`
    with no such shape after it stays a field. The statement is a node member and a delta aspect, never a field;
    its bare `cited` items are finder tokens a rename respells and make no edge.
  """
  verify unit "a field named verdict stays a field when no rule, outcome and block follow it"
  verify unit "a verdict statement has a rule, an outcome and members, and a truncated one keeps its block"
  verify unit "a verdict is a node member and a delta aspect, never a field"
  verify unit "a cited reference is a finder token and no edge"
  verify unit "the formatter keeps a verdict's members in their fixed order"
  verify unit "highlights.scm captures a verdict statement's keyword, rule and outcome"
  verify unit "a verdict line being typed is told apart from a verdict statement and from a field"
}

behavior digest_judged_inputs "Digest a Judged Rule's Inputs" {
  invariants [incremental_correctness]
  category   validation
  contract   """
    Each input of a judged rule is digested over its resolved authored values: layout, comments, verdicts and
    derived values change no digest, a respelled reference changes none, a rename changes the digests of its readers.
    A file input is read from its field's base through the checks' disk, so it is a watched check input, and a
    file that is a spec source of the project digests as what it declares. Entity digests are kept across updates
    and dropped by the graph delta; file digests are kept per stamp (ADR 0290).
  """
  verify unit "an entity digest ignores layout, comments, verdicts and derived values"
  verify unit "a respelling stales nothing and a rename stales the verdicts that read it"
  verify unit "a file input reads from its field's base and an absent file digests as absent"
  verify unit "a file input that is a spec source digests as what it declares"
  verify unit "a missing report hides no change to a cited entity or file"
  verify unit "cited tests are an input of their own, judged once a report exists"
  verify unit "the tests of a report digest sorted, whatever order they ran in"
  verify unit "a cited entity is digested by the resolution that accepted it"
  verify unit "a cited file outside the project is not read"
  verify unit "a spec source that declares nothing digests as its bytes"
  verify unit "a reference digests as the graph links it"
  verify unit "incremental digests equal cold digests"
  verify unit "a memo hit still watches its file"
  verify unit "a file change the session has not applied misses the memo"
}

behavior judge_rule_by_fresh_verdict "Judge a Rule by a Fresh Verdict" {
  invariants [incremental_correctness]
  category   validation
  contract   """
    A judged rule holds while a fresh verdict from its judge says so (ADR 0290). It fires at its severity for a
    missing, unrecorded, stale or failing verdict, and is silent for a fresh passing one. A verdict written by hand
    is E083 and counts as missing; one that cannot be read is W168. Test inputs are compared when the report is
    there, and said not judged (I205) when it is not. A gated rule is neither evaluated nor reported. A field whose
    writer is the verdict holds the fresh verdict's outcome or value, and is E082 when written by hand. A project
    that declares no judged rule reads no report and stamps no file.
  """
  verify unit "a project with no judged rule reads no report and stamps no file"
  verify unit "a hand-written verdict is E083"
  verify unit "a hand-written verdict counts as missing"
  verify unit "a verdict is a value of the ledger: set records it, a respelling keeps it, a move re-keys it"
  verify unit "a judged rule reports missing, stale and failed verdicts"
  verify unit "a stale verdict names the inputs that changed"
  verify unit "test inputs are not judged without a report"
  verify unit "a prompt-judged verdict is recorded and goes stale like an agent's, and its rendered prompt lists the rules it judges"
  verify unit "a verdict recorded without a report is stale once a report exists"
  verify unit "a re-run that agrees on every status stales nothing"
  verify unit "a gated judged rule computes and reports nothing"
  verify unit "a verdict that cannot be read is W168"
  verify unit "a changed file input rechecks the session"
  verify unit "a compute pass reads judgements and a V field is its fresh verdict's value"
  verify unit "a V field written by hand is E082"
  verify unit "a pass input carries each entity's judgements as they were written"
  verify unit "re-verification reads the baseline's judgements"
  verify unit "a baseline whose files git could not read is neither cached nor judged"
}

behavior answer_judged_inputs "Answer a Judged Rule's Inputs" {
  invariants [graph_traversal_integrity]
  types      [McpVerdictInputsResult]
  category   query
  contract   """
    `specforge query <entity> --inputs <rule>` and the MCP tool `specforge.verdict_inputs` answer each input's
    members and digest, the cited digest, the current verdict's state and the `basis` a `set_verdict` is checked
    against, by the one function that check calls (ADR 0290). Inspect, query, the graph export and a command's graph
    list each entity's verdicts and judgements.
  """
  verify unit "the inputs query answers the basis a set_verdict checks"
  verify unit "the verdict inputs tool answers what the inputs query answers"
  verify unit "a cited file is a relative path inside the project"
  verify unit "inspect, query, the graph export and a command's graph list each entity's judgements"
}

behavior show_verdict_state "Show a Verdict's State" {
  invariants [lsp_response_latency]
  category   query
  contract   """
    The hover on an entity lists its verdicts with their state; the hover on a verdict statement adds each
    input and whether it changed; a code lens at the verdict says fresh, failed or which inputs changed, and one
    at the entity's name says how many verdicts are missing (ADR 0290).
  """
  verify unit "the hover lists an entity's verdicts with their state and the inputs that changed"
  verify unit "a code lens says whether each verdict is fresh, failed or stale, and which inputs changed"
}
