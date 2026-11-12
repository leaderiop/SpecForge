// @code/core features (ADR 0329)

use "extensions/code/core/behaviors"

feature cc_test_facts "Test Facts a Spec Can Cite" {
  status   in_progress
  problem  """
    A runner's tests are not entities, so a requirement cannot name the test
    that proves it, and a citation written as a path rots silently when the
    test is renamed or never written.
  """
  solution """
    @code/core is the neutral base of test facts (ADR 0329). It declares the
    collector-written kinds test and test_run, whose ids name a test's
    package, file and suite path, and it judges every cited fact reference:
    linked, E101 (no such test in a served store), W102 (no facts of its
    scheme were collected) or unchecked while its owner is pending. A failing
    cited test is W401.
  """
}

feature cc_owner_state "An Entity Is Pending While Its Owner Is" {
  status   in_progress
  problem  """
    A requirement cited before its test exists, or while the test is still
    red, is TDD, not a defect, but the kernel had no notion of an entity
    being done.
  """
  solution """
    A kind declares complete_when, a field and the values that make its
    entities complete (protocol 1.15.0). A pass entity is pending while its
    own kind's declaration, or its nearest scope ancestor's, does not hold.
  """
}

feature cc_code_facts "Code Facts Every Pack Names Alike" {
  status   in_progress
  problem  """
    Source and runner packs write Rust tests, Rust and JavaScript items and
    packages, and must mint identical ids without calling each other; one
    scheme now holds several kinds, so a store of items is no evidence a
    test runner ran.
  """
  solution """
    One linked library, specforge-code-facts, mints every id and fact in a
    grammar a SCIP symbol maps into (ADR 0347). @code/core declares the item
    and package kinds beside test, and judges a cited reference by the
    collected kinds its field admits: E101 when a store holds one, W102
    naming the pack that writes it otherwise.
  """
}
