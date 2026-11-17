requirement INVARIANT-147 "Wasm Sandbox Integrity" {
  category "Invariant"
  tier     v2
  text     """
    Wasm extensions MUST NOT escape the Wasm sandbox. An extension MUST NOT
    access the host filesystem, network, environment, or memory outside its
    linear memory region; no sandbox policy permits it, since a policy
    declares limits only (ADR 0037). Any call that crosses a limit of its
    sandbox MUST trap the extension and emit a diagnostic (E028). No text
    an extension supplies reaches a user's terminal as a control sequence:
    the host reads it printable where it reads the answer (ADR 0244).
  """
}

requirement INVARIANT-029 "Extension Load Order Determinism" {
  category "Invariant"
  tier     v2
  text     """
    Given the same specforge.json extensions, the compiler MUST produce
    the same load order on every invocation and platform: the entries'
    order, except that an extension comes after the peers it declares
    (ADR 0041). The registry build MUST produce it, so every surface
    reads the same order.
  """
}

requirement INVARIANT-110 "Peer Dependency Satisfaction" {
  category "Invariant"
  tier     v2
  text     """
    If an extension declares peer dependencies, the compiler MUST verify that
    all declared required peers are installed, and that every installed peer
    satisfies its declared semver range. An optional peer may be absent.
    Unsatisfied peer dependencies MUST produce an error diagnostic (E-level), not
    a silent degradation. A range that is not a SemVer requirement satisfies
    no version (E073). `check`, `doctor`, `add`, `init`, `update` and `remove`, and
    the dry runs of `add` and `remove`, judge a peer by one rule over one
    set: the extensions the project's specforge.json entries load
    (ADR 0041). An installed extension is judged by what its module
    declares, which its lock entry must record (E070).
  """
}

requirement INVARIANT-146 "Wasm Compile Cache Integrity" {
  category "Invariant"
  tier     v2
  text     """
    The Wasm compilation cache MUST be keyed by the engine configuration and
    the exact component bytes that produced each artifact: an entry MUST
    deserialize only for the binary and engine that compiled it. Corrupted
    or mismatched cache entries MUST be ignored — the runtime falls back to
    fresh compilation — never served. The cache directory is managed by the
    runtime engine (wasmtime), selected via SPECFORGE_WASMTIME_CACHE.
    Separately, the integrity of an installed extension binary is enforced
    by the specforge.lock hash pin: a binary that no longer matches its
    recorded hash MUST be refused at load time (E070).
  """
}

requirement INVARIANT-028 "Extension Isolation" {
  category "Invariant"
  tier     v2
  text     """
    An extension failure MUST NOT affect other extensions or the host compiler.
    After an extension traps or fails during any lifecycle phase, the remaining
    extensions MUST continue execution normally. The failed extension MUST be
    excluded from subsequent phases in the current compilation.
  """
}

requirement INVARIANT-055 "Host Function Type Safety" {
  category "Invariant"
  tier     v2
  text     """
    Data exchanged between the host and extensions via host functions MUST
    conform to declared schemas. Malformed input from an extension MUST
    produce an ExtensionError diagnostic, not undefined behavior. The host
    MUST validate all extension-provided data before processing.
  """
}

requirement INVARIANT-026 "Entity Kind Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    No two extensions MAY register the same entity kind name: a kind an
    earlier-loaded extension registered is E026, and the first extension
    in load order keeps it. Collisions are detected when the registries are
    built. The compiler never arbitrates conflicts — extension authors
    resolve collisions via renames or peer dependencies.
  """
}

requirement INVARIANT-024 "Enhancement Field Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    No two extensions MAY register the same field name for the same entity
    kind: an enhancement never overwrites a field the kind already has,
    whether the kind's own or an earlier enhancement's, so the first
    registration in load order wins. The resolution is deterministic.
  """
}

requirement INVARIANT-005 "Collector Output Conformance" {
  category "Invariant"
  tier     v2
  text     """
    What a collector reports is only recorded for entities the graph
    declares: results for any other entity ID are dropped with a W115
    warning, not a hard error, so one stale annotation doesn't block the
    rest of a run. A test's status is passed, failed or skipped, the one
    vocabulary of the collect wire, the recorded test report and a pass's
    test results. Recorded statuses are only passed and failed; skipped
    tests are counted, never recorded as proof.
  """
}

requirement INVARIANT-119 "Registry Integrity" {
  category "Invariant"
  tier     v2
  text     """
    Downloaded extension binaries from a registry MUST be verified against
    their declared SHA256 hash before installation. Hash mismatches MUST
    produce a hard error diagnostic and abort installation. The hash and,
    for a signed package, its publisher key id MUST be recorded in
    specforge.lock. At the version specforge.lock pins, the bytes installed
    MUST be the ones it pins. A download MUST be no longer than the size
    the registry declares for it (64 MiB at most).
  """
}

requirement INVARIANT-115 "Publisher Trust" {
  category "Invariant"
  tier     v2
  text     """
    A registry package is installed only when its publisher signature
    verifies over the digests the registry describes (which the downloaded
    bytes must match) and the served manifest, or when it is unsigned, no
    key is expected for its name (no pin of the user's and none in
    specforge.lock) and the user passed --allow-unsigned; an unsigned
    package for a name whose key is pinned or locked is never installed;
    a broken signature is
    never installed. The first verified key for a package is pinned, and a
    package signed by another key is refused unless the user consents. A
    key is pinned only for an install that is written, and a store of pins
    that can't be read trusts nothing.
  """
}

requirement INVARIANT-121 "Registry Reply Binding" {
  category "Invariant"
  tier     v2
  text     """
    What a registry install verifies, pins and installs is the package and
    version that was requested, described by a manifest that can be read:
    a reply or manifest for another package or version, or a missing or
    unreadable manifest, is refused before any key is pinned; a reply
    whose name, version or digest is no package name, SemVer version or
    lowercase SHA-256 is refused before anything is downloaded.
  """
}

requirement INVARIANT-030 "Extension Operation Atomicity" {
  category "Invariant"
  tier     v2
  text     """
    Extension install, uninstall, and update operations MUST be atomic.
    On failure, all changes MUST be rolled back — no partial installs,
    no stray files, no inconsistent lock state, and no publisher key
    pinned for an install that was not written.
  """
}

requirement INVARIANT-009 "Registry Credential Secrecy" {
  category "Invariant"
  tier     v2
  text     """
    Raw authentication tokens MUST never be logged, stored in
    specforge.json, or included in diagnostic output. Only token
    presence/absence and validity status may be reported. The user's
    registry files (the credential store, the kept secrets, the publisher
    signing key and the known keys) MUST be created owner-only, 0600 files
    in a 0700 directory on unix, before a byte is written, and replaced
    whole through a temporary created new: never readable by another user,
    never left half written, never written through a link. A signing key
    file is named in a diagnostic, never quoted.
  """
}

requirement INVARIANT-135 "Surface Schema Validity" {
  category "Invariant"
  tier     v2
  text     """
    Every registered extension MCP tool MUST have an input_schema that is
    a JSON object, and an output_schema, when declared, that is a JSON
    object: a tool with another value produces E055 and is not
    registered. Every command arg MUST have a CommandArgType: the type is
    closed, so a surfaces description with another one does not parse and
    fails its extension's load (E028).
  """
}

requirement INVARIANT-031 "Extensions Run in Their Loading Runtime" {
  category "Invariant"
  tier     v2
  text     """
    Every extension call the host makes over a project MUST run in the
    runtime that loaded the project's extensions: the environment holds
    that runtime from the moment it loads them, a compiled project, a
    project session and every project view read it there, and no
    operation over a project view takes a runtime of its own (analyze's
    passes, collect's collectors, an extension
    command, a check's custom verdicts and passes). An environment loaded
    without a runtime loads no extension, and an extension call over it
    is E028, not loaded. Migrate, which compiles the project itself, and
    reading a candidate's declaration, which is in no environment, are
    handed a runtime.
  """
}
