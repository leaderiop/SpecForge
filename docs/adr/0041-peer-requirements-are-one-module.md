# Peer requirements are one module

**Status:** accepted (2026-10-08). Amends ADR 0001 and ADR 0012 D11.

An extension's declared peers (`PeerDependency { name, version, optional }`, the range as raw text)
were judged in five places that disagreed. A range that is not SemVer was a warning in `check`
(W062, and only when the peer was loaded), an error in `doctor`, satisfied in `add` and `update`,
and R-RES-003 when another requirer's range was unified. Doctor judged only lock entries, so a peer
a builtin satisfied was "not installed". The load order "dependencies first" was stated by the
spec, the docs and the code, and produced by nothing: declarations loaded in `specforge.json` order,
which decided E026, W018, enhancements, passes, rules and surfaces, so a dependent listed first took
its peer's keyword. The one sort, `specforge_wasm::topological_sort_extensions`, ran only inside
`migrate`'s hook run, so a cycle among required peers reached no compile, and failed a migration
only when a file was pending one.

## D1. One pure module, beside the package types

`specforge_protocol_types::peers` holds the rule: `PeerRequirement::read` reads a declared peer's
range as Cargo reads a SemVer requirement; `verdict(declared, installed)` is the one satisfaction rule
(`Satisfied`, `Missing`, `OutOfRange`, `NotSemver`, `Unreadable`); `Peers::of(members)` gives a set's
`load_order`, `cycles` and `unsatisfied`. It is pure and diagnostic-free like `package` (ADR 0036
D1); `specforge_common::peers` builds the diagnostics. The wire keeps the text: a declaration with an
unreadable range still loads and registers its kinds (as a bad rule costs one rule, ADR 0020 D6).

## D2. A range that can't be read satisfies nothing: E073

The range is read first, so an unreadable range is reported whether its peer is loaded, missing or
optional. It is the error E073, not W062 (retired, replaced by E073): no version can satisfy it, and
its fix is the dependent's, not the peer's. A peer installed at a version that is not SemVer is E027
(no range accepts it).

## D3. The registry build puts the declarations in load order

`build_registries` takes the declarations in entry order and loads them in load order before
anything else reads them. Load order is entry order, except that an extension comes after the peers
it declares: every required one, and an optional one unless its edge would close a cycle (optional
edges are taken in dependent-then-peer name order, after the required ones). Every first-wins rule
and every in-order list follows it. The order is a function of the entries, deterministic, and
idempotent: building from a build's declarations changes nothing. Ties keep the user's order rather
than names, so unrelated extensions keep the precedence `specforge.json` gives them.

## D4. A cycle among required peers is E027 in every compile

Each strongly connected set of required peers is one E027 naming its extensions, among the
declarations' own diagnostics (E030, W021, the peers' E073 and E027, the cycles' E027, W145). Its
members still load, together, in entry order. `migrate` runs hooks in the build's order and no longer
fails on a cycle; the sort leaves `specforge-wasm`.

## D5. Doctor reports the compile's verdicts

`Installed::health` checks modules only. Doctor reports the compile's E027 and E073 as its peer
findings (code, message, the diagnostic's suggestion), so doctor and check cannot disagree.

## D6. The gate judges by the same rule and covers every add

`check_diamonds` reads `verdict`: an unreadable range (the candidate's or a requirer's) is E073; a
locked peer out of range is unified against the registry (R-RES-006/005, ADR 0001) or, for a local
install, refused with E027. Both add paths check the locked extensions that require the package
they install, as update checks the dependents of what it updates.

## Not done

- `add` does not install a missing peer (dependency resolution); `check`'s E027 reports it.
- The gate does not consult builtins or `.wasm` file entries: their versions are not installable
  choices, so there is nothing to unify; `check`'s E027 reports a mismatch.
- The SDK does not panic on a malformed range; authors meet E073 in `extension validate`, `publish`
  and `check`.
- `PeerDependency.version` keeps its wire name and peer names stay text.

## Consequences

- A malformed peer range fails `check`, the LSP and MCP (was a warning), `doctor`, `add`, `update`
  and `publish`.
- A project listing a dependent before its peer now loads the peer first: E026/W018 name the
  dependent as the later registration, and passes, rules, surfaces and the schema's extension list
  follow dependency order. The repo's own config loads governance before testing.
- `doctor --format json`: peer problems are in `peers` with the diagnostic's code; `issues` holds
  only binary problems; `peer_mismatch` is gone.
- A local `add`, and an add that would break a locked dependent, can be refused.

**What would reopen it:** a need for user-chosen precedence that contradicts the peers (load order
overrides), or a dependency resolver for `add` (a new ADR, superseding D6's "missing peers are
check's").
