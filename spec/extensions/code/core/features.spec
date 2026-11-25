requirement FEATURE-155 "An API Entry Judged Against Its Accepted Report" {
  category "Feature"
  tier     v2
  text     """
    An api_item fact said what a run's public API holds, but not what
    changed since the API a project committed, so no guard could name a
    break.

    @code/core 1.4.0 adds four optional api_item fields (ADR 0390 D3):
    change, one of added, unchanged, changed, narrowed or removed;
    accepted_signature and accepted_tag, what the committed accepted report
    wrote; and accepted_visibility, its visibility when it differs. A pack
    judges only a crate or package present in both the run and the accepted
    report; any other entry carries no change. Entries pair by id, then by
    comparable signature, never by repeat suffix. A removed entry cites no
    item, and W104 does not report it.
  """
}

requirement FEATURE-004 "Test Facts a Spec Can Cite" {
  category "Feature"
  tier     v2
  text     """
    A runner's tests are not entities, so a requirement cannot name the test
    that proves it, and a citation written as a path rots silently when the
    test is renamed or never written.

    @code/core is the neutral base of test facts (ADR 0329). It declares the
    collector-written kinds test and test_run, whose ids name a test's
    package, file and suite path, and it judges every cited fact reference:
    linked, E101 (no such test in a served store), W102 (no facts of its
    scheme were collected) or unchecked while its owner is pending. A failing
    cited test is W401.
  """
}

requirement FEATURE-003 "An Entity Is Pending While Its Owner Is" {
  category "Feature"
  tier     v2
  text     """
    A requirement cited before its test exists, or while the test is still
    red, is TDD, not a defect, but the kernel had no notion of an entity
    being done.

    A kind declares complete_when, a field and the values that make its
    entities complete (protocol 1.15.0). A pass entity is pending while its
    own kind's declaration, or its nearest scope ancestor's, does not hold.
  """
}

requirement FEATURE-002 "Code Facts Every Pack Names Alike" {
  category "Feature"
  tier     v2
  text     """
    Source and runner packs write Rust tests, Rust and JavaScript items and
    packages, and must mint identical ids without calling each other; one
    scheme now holds several kinds, so a store of items is no evidence a
    test runner ran.

    One linked library, specforge-code-facts, mints every id and fact in a
    grammar a SCIP symbol maps into (ADR 0347). @code/core declares the item
    and package kinds beside test, and judges a cited reference by the
    collected kinds its field admits: E101 when a store holds one, W102
    naming the pack that writes it otherwise.
  """
}
