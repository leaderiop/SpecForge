requirement FEATURE-154 "Kani Proof Facts" {
  category "Feature"
  tier     v2
  text     """
    A Kani proof harness left no trace in the graph: no pack ran Kani, and
    no reader read what it prints.

    @rust/proof runs cargo-kani over the workspace when the project has a
    Cargo.toml and reads Kani's regular output, which the host captures
    from standard output. Each proof harness becomes a test and its run,
    named rust:<crate>#<harness path>, passed when Kani says SUCCESSFUL
    and failed when it says FAILED, placed at the item @rust/source
    answered for the harness function. The crate is found from the served
    items, as Kani prints harness paths without it. An output cut off
    before Kani's Complete line, or whose Complete totals disagree with
    the harnesses read, is the collector's error, never a dropped proof.
    An output with no harness at all reads the same whether the crate has
    none or Kani stopped at the build, so the pack warns that nothing was
    checked (W203) instead of looking like a clean run.

    Kani verifies Rust only and runs on Linux x86_64 and macOS, not on
    Windows. There is no JavaScript proof pack: no JavaScript tool proves
    a property of all inputs and reports a verdict per harness the way
    Kani does. Property-based testing samples inputs and already reports
    through the runner packs, and TypeScript's checker proves types, not
    behaviour, so a JavaScript proof pack would be a test runner under
    another name.
  """
}
