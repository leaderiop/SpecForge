# A snapshot is a graph at a revision, and a pass can read what changed

**Status:** accepted (2026-11-18). Builds on [ADR 0006](0006-one-incremental-rebuild.md) (one incremental rebuild,
whose graph delta this reuses), [ADR 0032](0032-one-graph-build.md) (one graph build, which a revision's graph goes
through), [ADR 0058](0058-every-mcp-message-is-a-spec-type.md) and
[ADR 0074](0074-the-session-update-states-the-diagnostics-delta.md) (the graph and diagnostics deltas the surfaces
send, whose bytes this leaves alone), [ADR 0198](0198-a-typed-edit-is-judged-by-the-graph-change-it-makes.md) (a typed
edit judged by the graph change it makes), [ADR 0222](0222-what-an-entity-holds-is-written-once.md) (what an entity
holds is written once, the bytes its digest covers),
[ADR 0326](0326-a-host-refuses-a-guest-of-a-newer-protocol-minor.md) (E088, which keeps a host of the previous minor
from dropping the new input),
[ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md) D2 (inputs are one
list, which the delta input joins) and [ADR 0353](0353-a-pass-reads-the-facts-it-declares.md) (the `facts` input,
which the delta input mirrors). **Amends**
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) D4: a superseded fact store is
archived and pruned beyond a retention count, not deleted. Protocol 2.3.0.

## Context

The kernel can tell what changed in a graph, but only inside one session and only to itself.

- **The graph delta exists and is private.** `compute_graph_delta` (`crates/pipeline/graph/src/delta.rs`) names the
  nodes added, removed and modified, with the changed fields and aspects, and the edges added and removed. The watch
  loop, the MCP session and the typed edits read it. No user and no extension can ask for it, and it carries no
  content digest: two edits that change one field the same way are not told apart from one that changes it twice.
- **A graph at a commit is built and thrown away.** `Baseline::at` reads a commit's `*.spec` files and its harness
  ledger through the `Git` port, builds the graph with the one cold build, then flattens it to the baseline's entities
  and drops the graph. A second reader of a past commit would have to copy that block.
- **Facts at a past commit are gone.** A collect files its facts under a key (the HEAD sha, `<sha>-dirty` or
  `worktree`) and deletes the collector's stores under every other key (ADR 0327 D4). The ledger at a commit names the
  store and its digest, but the store itself no longer exists, so a past graph can never hold its facts.
- **A pass sees everything every time.** A pass is sent every entity of the kinds it reads (ADR 0353 D2). A pass that
  only needs to look at what changed has no way to know what that is, so it re-judges the whole project on every run.

## D1. A snapshot is a graph and a key

A snapshot is the compiled graph of one state of a project, its spec entities and its fact nodes, with a key that says
which state it is: a git revision, named as the user wrote it and resolved to its sha, or the working tree, keyed by the
facts key of the current compile (`<sha>`, `<sha>-dirty` or `worktree`). Two snapshots are compared node by node and
edge by edge; neither is a file list.

The reason: what changed between two states is a statement about their graphs, and a key that names the commit, or the
working tree under the key its facts are filed by, is what lets the answer say which two states it compared.

## D2. One loader builds the graph at a commit

One session function reads the `*.spec` files, the harness ledger and `specforge.json` of a commit through the `Git`
port, with no checkout and no file written, and builds the graph through `Environment::build_sources`, the one cold
build. It returns what it read beside the graph: the held texts, the ledger, the tree when asked for. `Baseline::at`
keeps its cache, its fingerprint and its held set and builds through this function on a miss, so there is one way to
build a graph from a commit. The revision text reaches only `Git::commit`, which resolves it in process; it is never an
argument to a program.

The reason: two readers of a past commit that each build the graph drift apart the first time one of them learns to
read a new file, and a revision string handed to a process is an option an attacker can write.

## D3. Both sides compile with the current extension environment

A snapshot at a revision compiles with the extensions loaded now, not the ones the revision's `specforge.json` named.
A revision whose `specforge.json` names another extension set compiles all the same, and the answer carries a
structured `notes` entry that says so; it is not a diagnostic code.

The reason: a diff answers what changed in the content, and two sides compiled by two toolchains would also show what
changed in the toolchain, mixed in with it; a note says the toolchain differs without making the answer fail.

## D4. Facts at a revision come from a retained store the revision's ledger trusts

A revision's facts are read from a retained store, never from git (stores are git-ignored) and never by running a
collector. A store is retained in the live `.specforge/facts/<key>/<collector>.json` while it is current, then in the
archive `.specforge/facts-history/<seq>-<key>/<collector>.json`, where `<seq>` is monotonic because a store has no
clock. A collect archives the store it supersedes and prunes the archive beyond `facts.retain` stores per collector
(1 to 100, default 8). A store is served at a revision only when its sha256 equals the digest the revision's ledger line
records for its collector and key, the rule the working tree already applies (W191, W192). Otherwise that side says
"facts not collected at <rev>" for the collector, and the collector's facts are left out of both sides, so a missing
store is never read as every fact removed. A ledger line whose key or collector is not a well-formed store name is never
made into a path. Serving the working tree never lists the archive, so an archived store is no W191 or W192.

The reason: a fact describes the code at the commit it was collected on, so only the store the ledger recorded there
may stand for it; a collector run on a past revision would run project tooling against code that is not checked out,
and a fact invented to fill the gap would be a lie the diff then reports as a change.

## D5. The diff surface

`specforge diff [<from>[..<to>]]` and its MCP twin `specforge.diff` name what changed between two snapshots. No argument
compares HEAD with the working tree, `<rev>` that revision with the working tree, `<a>..<b>` two revisions. `<a>...<b>`
is a usage error, and an unknown revision is the `unknown_revision` refusal that `check --since` gives. Each changed
node is named by its kind and id, with its change (`added`, `removed` or `modified`), for a modified node its changed
fields and aspects, and its before and after content digests; each changed edge by its source, target, label and
change. `--kind`, repeatable, keeps the nodes of the kinds it names and the edges with an end of one. A summary counts
the changes by change and by kind. The command exits 0 whether or not anything changed. The operation lives in the read
group of `specforge-ops` (ADR 0086), and the tool is a row of the engine-tools catalogue (ADR 0334). A node whose kind
changed is its old kind removed and its new kind added, since an entity is its kind and its id.

The reason: a diff is a read, and a read that exits non-zero on a finding is a check; one parser for the range and one
operation keep the two surfaces word for word the same.

## D6. The delta input

A pass may declare `Input::Delta { kinds }`, wire tag `delta`. It is served to passes only. An empty, unknown or
repeated delta input is treated as the `facts` input is: W021, served empty, the call kept. `PassInput.delta` is absent
from the bytes of a pass that does not declare it. The payload names the base, then per changed node of a named kind
its kind, id, change, changed fields and aspects, and its before and after digests (a removed node only its kind and
id), and the edges added and removed whose source or target is of a named kind. The full content of an entity stays in
`entities`. The delta is computed once per check and shared by every pass that declares it.

The reason: a pass that knows what changed can skip what did not, and a pass that does not declare it must see the same
bytes as before, or every guest built before would see a new key.

## D7. Its base

The base is `--since <rev>` when the user gave one (the merge-base, resolved before any pass runs), otherwise the graph
of a session's last completed check (held only while a loaded pass declares the delta, dropped when the environment
reloads), otherwise none, which is the empty graph: every node is added. The payload names its base: `none`,
`revision` with the sha, or `previous`.

The reason: with the empty graph as the base of last resort, a pass written to process only what changed is correct on
a cold run by construction, only slower; a base resolved after the passes ran would make them run twice.

## D8. Protocol 2.3.0

The delta input and `PassInput.delta` are a minor of the protocol: 2.3.0. A host of 2.2 refuses a guest that declares
the delta (E088), instead of dropping an input the guest relies on.

The reason: an additive input a host does not know is served empty, and a pass written for changes would then judge
nothing without a word; the minor makes the old host say so.

## As built

- `crates/session/project/src/snapshot.rs`: the loader (`load_at`, shared with `Baseline::at`), `Snapshot`
  (`at_revision`, `of_project`), the facts a revision's ledger trusts, and `compare`.
- `crates/session/project/src/delta.rs`: `Delta`, the delta between two graphs with digests, sorted, filtered by kind.
- `crates/leaf/protocol-types/src/delta.rs`: the wire payload (`DeltaPayload`, `DeltaBase`, `DeltaNode`, `DeltaEdge`).
- `crates/app/ops/src/read/diff.rs`: the diff operation and its range parser.
- `crates/surfaces/cli/src/diff.rs`: `specforge diff`.
- `crates/surfaces/mcp/src/tools/diff.rs`: `specforge.diff`.
- `crates/session/project/src/facts/retain.rs`: the archive and its pruning.

## Rejected

- **A checkout into a temporary directory.** It writes the disk and needs a git program; the `Git` port reads a commit
  in process.
- **Facts from git.** Stores are git-ignored; only their digest is committed, in the ledger.
- **Collecting at a past revision.** Collectors run project tooling, against code that is not checked out.
- **A new diagnostic code for an extension-set mismatch.** It is a fact about the comparison, not a defect of the
  project; it rides in the answer's notes.
- **Deleting superseded stores, as ADR 0327 D4 did.** No revision but the current one could then be diffed with its
  facts.
- **`...` as a merge-base compare.** It hides which side changed; a user who wants the merge-base names it.
- **Digests in the graph crate's delta.** They would change the `graphChanged` bytes the surfaces send, and the digest
  is a session concern.

## Consequences

- A user and an agent can ask what changed between two commits, or since HEAD, as a shaped answer.
- `Baseline::at` and the diff read a commit the same way.
- A collect keeps up to `facts.retain` older stores per collector under `.specforge/facts-history/`.
- A pass may run on only what changed, and is correct with no base.
- A guest that declares the delta needs a 2.3 host.

## Would reopen it

- A pass that needs the content of a removed node, which the payload does not carry.
- A collector that can run against a commit without a checkout.
- The archive growing past what `facts.retain` bounds, for example one store per collector per commit on a large team.
