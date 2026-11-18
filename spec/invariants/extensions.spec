requirement INVARIANT-083 "Offline-First Extension Resolution" {
  category "Invariant"
  tier     v2
  text     """
    Extension resolution MUST work fully offline when cached manifests
    and .wasm binaries are available locally. The compiler MUST never
    require network access to load already-installed extensions. Network
    registries are opt-in; local path and git sources MUST always work
    without connectivity. When a registry is unreachable, the system
    MUST fall back to the cached manifest in specforge.lock and the
    locally stored .wasm binary.
  """
}

requirement INVARIANT-118 "Registry API Openness" {
  category "Invariant"
  tier     v2
  text     """
    The extension registry API schema MUST be published as an open
    specification so that third-party registries can implement it.
    SpecForge MUST NOT be the only possible registry host.
  """
}

requirement INVARIANT-001 "Authentication Never Gates Core Use" {
  category "Invariant"
  tier     v2
  text     """
    No SpecForge command MUST require registry authentication to function
    with builtin extensions, local paths, or git sources.
    Authentication is exclusively for private and enterprise registries.
    The first-use experience (init, check, export) MUST complete without
    credentials. This is a P8 (seconds to value) structural guarantee.
  """
}
