// @specforge/software features — capability groupings

use "behaviors/validation"
use "extensions/software/formal-contracts"
use "extensions/software/formal-refinement"
feature se_gherkin_bridge "Gherkin Bridge" {

  problem """
    Behavior entities need a way to reference Gherkin .feature files for
    BDD traceability. This is a domain-specific concern — not all spec
    domains use Cucumber/BDD — so it must be an extension responsibility,
    not a core grammar construct.
  """

  solution """
    The @specforge/software extension declares a gherkin field with type
    string_list and file_reference=true on behavior entities via the
    FieldRegistry. The field is parsed as a standard StringList value —
    no dedicated grammar rule or AST type is needed. File existence
    validation is handled by the generic validate_file_reference_paths
    behavior which operates on any field with file_reference=true (E016).
  """
}

feature se_core_entity_kinds "Core Entity Kind Registration" {

  problem """
    The @specforge/software extension must register 6 entity kinds with
    full metadata, 9 edge types, field definitions, validation rules,
    verify kinds, and LSP metadata. Without this registration, the
    compiler has zero knowledge of software engineering domain concepts.
  """

  solution """
    A comprehensive manifest declaration provides all entity kinds with
    testability flags, verify kind allowlists, LSP metadata (semantic
    tokens, icons), DOT shapes, typed field definitions with edge
    mappings, and declarative validation rules. Registration follows
    the zero-entity core protocol defined in ManifestV2.
  """
}

feature se_validation_suite "Software Validation Suite" {

  problem """
    Without domain-specific validation rules, the compiler can only
    perform structural checks. Orphan entities, unverified testables,
    invalid trigger references, and type annotation errors go undetected.
    Users receive no warnings about specification quality issues.
  """

  solution """
    Declarative validation rules (W001-W005, W007-W010, W009, W028, E006, E004)
    detect common specification quality issues: orphan entities without
    incoming edges, testable entities without verify statements, invalid
    event triggers, features with empty behavior lists, unknown field
    annotations, unenforced invariants, invalid verify kinds for entity
    types, and contracts without formal verification. Each rule uses the
    declarative pattern engine.
  """
}

feature se_formal_contracts "Design by Contract" {

  problem """
    Behavior contract fields are free-form prose. Preconditions and
    postconditions are mixed together in a single text block. There is
    no structured way to declare named conditions, check consistency,
    or generate verification conditions from contracts. AI agents
    cannot reliably extract preconditions from prose.
  """

  solution """
    Structured requires/ensures/maintains blocks with named conditions
    augment (not replace) prose contracts (progressive formality Level 2
    per RES-25). The contract_check compiler pass validates satisfiability,
    reachability, cross-consistency with invariants, and Liskov compliance
    for refined behaviors. Named conditions map 1:1 to test assertions and
    proof obligations. Port operation contracts extend DbC to port method
    signatures (W036). Additional warnings detect unverifiable conditions
    (W037), unreachable postconditions (W038), redundant preconditions
    (W039), and invariants without formal properties (W040).
  """
}

feature se_formal_refinement "B-Method Refinement" {

  problem """
    There is no way to express that a behavior is an abstract
    specification refined by concrete implementations. The existing
    entity graph is flat — all behaviors are at the same level. This
    makes it impossible to model progressive elaboration from high-
    level requirements to detailed implementation behaviors.
  """

  solution """
    The abstract/refines mechanism creates refinement chains between
    behaviors. Abstract behaviors serve as specification-only entries.
    Concrete behaviors declare refines to build a DAG. The refinement
    verify pass checks completeness (every abstract has a concrete),
    correctness (refined preserves abstract's guarantees), and DAG
    structure (no cycles). This models B-Method stepwise refinement.
  """
}

feature se_formal_concurrency "CSP Concurrency Analysis" {

  problem """
    Event entities declare producers and consumers but the compiler
    performs no concurrency analysis. Circular event dependencies
    (deadlocks), payload type mismatches, unmatched producers, and
    infinite retry loops (livelocks) are invisible until runtime.
    Distributed system failures are the hardest bugs to reproduce.
  """

  solution """
    The process_analyze compiler pass builds an event-behavior bipartite
    graph and performs CSP-inspired analysis. Tarjan's SCC algorithm
    detects deadlocks (E034). Payload type checking catches channel
    mismatches (E035). Producer/consumer pairing detects lost side
    effects (W029). Retry pattern analysis flags livelock risks (W032).
    Starvation detection on ports (W033). Unbounded channel buffer
    detection (W034). Sync blocks declare barrier and timeout constraints.
  """
}

feature se_proof_obligations "Proof Obligations and Discharge" {

  problem """
    There is no way to know whether formal properties (contracts,
    invariants, refinements) are verified by existing tests. Coverage
    is binary (has test / no test) rather than graduated. Proof
    obligations are invisible — users cannot see what remains unverified.
  """

  solution """
    The proof_obligation pass generates machine-readable verification
    conditions categorized as contract_preservation, invariant_preservation,
    or refinement_correctness. Each obligation tracks discharge status:
    pending, auto_proved, or test_verified. Info diagnostics (I008, I009,
    I015) confirm when obligations are discharged or when formal analysis
    is available. Coverage evolves from binary to graduated: UNPROVED,
    PARTIAL, PROVED, STRONG, VERIFIED (RES-25).
  """
}

feature se_analyze_commands "Formal Analysis CLI Commands" {

  problem """
    Formal analysis results are only available as part of the full
    compilation pipeline. There is no way to run specific analyses
    in isolation, get machine-readable output for CI integration,
    or fail builds on formal violations.
  """

  solution """
    The specforge analyze subcommands (contracts, refinement, concurrency,
    all) run individual or combined analysis passes. Each supports --json
    for machine-readable output and --strict for CI gate integration
    (non-zero exit on any violation). This follows the RES-25 CLI design.
  """
}
