# Version diamond resolution is intersection-based unification, not backtracking (C8-07)

**Status:** accepted

When two installed extensions each declare a peer dependency on the same
third package at different semver ranges (a "version diamond"), `specforge
add` must decide whether a single locked version can satisfy both, instead of
silently installing and leaving `doctor` to discover the conflict later.

We resolve this by **intersection**: collect every requirer's range for the
shared peer, fetch the peer's published versions from the registry, and pick
the highest version that satisfies every range simultaneously
(`specforge_registry::client::resolver::{resolve_diamond, unify_diamond}`).
If no version satisfies every requirer, `add` fails with a diagnostic
(`R-RES-005`/`R-RES-006`) naming each conflicting requirer and its range,
rather than guessing.

We deliberately do **not** implement general backtracking — searching for an
alternative combination of *other* packages' versions to unblock an
otherwise-unsatisfiable diamond (the way Cargo's resolver can downgrade an
unrelated dependency to make room). SpecForge's install model is flat (one
locked version per extension name, no lockfile-wide SAT search), and the
extension graph is shallow enough in practice that unresolvable diamonds are
rare and best fixed by the user upgrading the narrower requirer. Backtracking
would add real complexity (a proper resolver algorithm, non-deterministic
output ordering, harder-to-explain failures) for a case that hasn't shown up
yet. If it does, that's its own design doc and its own ADR — this one only
covers unification.
