// specforge-test crate architectural decisions

use invariants
use behaviors

decision drop_over_catch_unwind "Drop Guard Over catch_unwind" {
  status accepted

  context """
    The proc macro needs to detect whether a test passed or failed to
    record the result. Two approaches exist: wrapping the test body in
    catch_unwind, or using a Drop guard that checks std::thread::panicking().
  """

  decision """
    Use Drop-based TestGuard that checks std::thread::panicking() during
    drop. This works with async tests (#[tokio::test]), parameterized
    tests (#[rstest]), and does not require AssertUnwindSafe wrappers.
    The guard does not interfere with panic propagation.
  """

  consequences """
    Positive: works with all major test frameworks, simpler expansion,
    no interaction with await points.
    Negative: #[should_panic] tests are incompatible — the guard records
    fail because panicking() is true during expected panics. Documented
    as unsupported; use Result-based error testing instead.
  """

  protects [drop_guard_correctness]
}

decision build_rs_over_manual_export "build.rs Over Manual Export" {
  status accepted

  context """
    The coverage summary and compile-time entity validation need the spec
    graph to be available at build/test time. Options: (a) manual
    `specforge export` step, (b) build.rs calls specforge automatically.
    Manual export leads to staleness — developers forget to re-export
    after spec changes.
  """

  decision """
    build.rs calls `specforge export --format=graph --output
    target/specforge/graph.json` and sets cargo::rerun-if-changed=spec/.
    If specforge is not installed, build.rs emits a cargo::warning and
    continues. This makes the graph always fresh without manual steps.
  """

  consequences """
    Positive: zero staleness, zero manual steps, spec changes trigger
    rebuild, standard Rust pattern (like protoc for prost).
    Negative: adds ~1 second to builds when spec changes. Requires
    specforge on PATH for full functionality.
  """

  protects [graceful_degradation]
}

decision separate_workspace "Separate Workspace from Compiler" {
  status accepted

  context """
    The specforge-test crate could live in the main compiler workspace
    or in a separate integrations/ directory with its own Cargo.lock.
  """

  decision """
    Place in integrations/rust/ with its own Cargo.lock, separate CI,
    and separate release cycle. Same pattern as rustfmt in rust-lang/rust.
    The crate has zero dependency on compiler crates — only the JSON
    schema is shared.
  """

  consequences """
    Positive: independent versioning, can publish to crates.io on its
    own cadence, no compiler rebuild when crate changes.
    Negative: schema changes require manual synchronization. Mitigated
    by versioned schema (specforge: "1.0" field in report).
  """

  protects [zero_compiler_dependency]
}

decision atexit_over_custom_harness "atexit Over Custom Test Harness" {
  status accepted

  context """
    Rust's custom_test_frameworks feature is unstable since 2018 with no
    stabilization timeline. Using it would require nightly Rust. The
    alternative is libc::atexit to hook process exit.
  """

  decision """
    Use libc::atexit (POSIX) registered via std::sync::Once on first
    TestGuard creation. Each test binary writes its own report file.
    No nightly Rust required. No custom test framework needed.
  """

  consequences """
    Positive: works on stable Rust, no test harness interference,
    handles multi-binary workspaces naturally.
    Negative: atexit is POSIX — Windows needs a different mechanism
    (std::process::exit handler or similar). Deferred to Phase 2.
  """

  protects [atexit_write_once]
}

decision convention_as_fallback "Convention Mapping as Fallback" {
  status accepted

  context """
    Not all test scenarios support proc macro attributes. proptest's
    decl-macro style and some generated tests cannot carry attributes.
    Teams may also want zero-dependency adoption.
  """

  decision """
    Convention-based mapping is the fallback tier: module name matches
    entity ID, or double-underscore naming {entity_id}__{slug} in
    function names. The proc macro is primary; conventions are for edge
    cases and zero-dependency adoption.
  """

  consequences """
    Positive: zero-dependency path exists, works with proptest decl macros.
    Negative: convention is fragile — renames break it. Mitigated by
    specforge trace showing unmatched tests.
  """

  protects [convention_separator_unambiguous]
}
