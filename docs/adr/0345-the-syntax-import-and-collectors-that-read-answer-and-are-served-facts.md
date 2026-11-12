# The syntax import, and collectors that read, answer and are served facts

**Status:** accepted (2026-11-08, R2 of the Beyond Parity build plan). Builds on
[ADR 0344](0344-tree-sitter-is-the-hosts-one-source-reader.md) (the reader). **Amends**
[ADR 0335](0335-the-host-owns-every-effect-through-one-effect-plane.md) D4 (five ports, not four) and D5 (a `syntax`
import is backed by an `fs` or a `git` capability),
[ADR 0337](0337-the-ports-protocol-is-2-0-0-inputs-are-one-list-and-effects-are-journaled.md) D2 (a `facts` input,
served to a collector only),
[ADR 0327](0327-a-collectors-facts-are-read-only-entities-recorded-by-digest.md) D5 (facts are also served to a
collector) and its admission (a fact of a kind its collector does not answer is E116), and
[ADR 0125](0125-collects-machine-is-one-port-the-caller-passes.md) D7 (a fourth evidence source: the ports). Protocol
2.0.0, unreleased: its additions go into the 2.0.0 row, no bump.

## Context

- A source pack (`@rust/source`, `@javascript/source`) needs every definition in a project's source with its span and
  its parent, read by the host's reader (ADR 0344). A guest has no file system and no reader.
- A collector today must declare a report: with none it reads `.specforge/reports/<name>.json`, and no file is E045.
  A source pack runs no command and writes no report; its evidence is what it reads.
- `@rust/source` needs the packages `@rust/cargo` collected (to name an item's crate); a test pack needs the items.
  Kinds are declared by `@code/core`, not by the collector that writes them, so nothing says who produces a kind, and
  nothing orders collectors.

## D1. `specforge:host/syntax`

A fifth interface of `specforge:host@0.1.0`: `parse(request) -> result<syntax-answer, host-error>`.

```text
request  { source: worktree(root) | revision(commit), files: paths(list) | glob(text), language, query }
answer   { files: [ { path, digest?, outcome: parsed { matches, errors } | not-parsed(reason) } ],
           truncated, parsed, cached }
match    { role: definition | reference, kind, span, parent?, captures: [ { name, text, cut, span } ] }
reason   deadline | too-large | missing | unreadable | denied | capped
```

- **It is granted on the fs port's read scope, no new port, no new capability kind.** At the worktree, each path is
  one grant-checked fs read (a whole-file read, held to the 64 MiB file ceiling) and a glob is one grant-checked walk
  of the root, whose `truncated` carries to the answer. A refusal by the gate, the declaration or the ceiling is the
  whole request's `denied(scope)` and E090, as an fs read's is. Per file: `missing`, `too_large`, `denied` (what the fs
  port refuses to read: a withheld name, a link out of the root, a file with a second name) or `unreadable`. At a revision, the git read scope (`blobs`, and `tree` for a glob, matched with the hook glob);
  a withheld path is `denied`, a blob over the git port's blob cap `too_large`. fs and git reads need no approval.
- **Backing (E089):** a `syntax` import is backed by an `fs` or a `git` capability of the extension.
- **The answer is held to the call's answer cap** (a quarter of guest memory): once the encoded answer passes it, the
  later files are `not_parsed(capped)`, and a pack asks for them again by path. `truncated` says only that a glob's
  walk stopped short: the files past it are not in the answer, and no glob can ask for them. `parsed` and `cached`
  are the request's.
- **What did not parse is said once per call.** W196 "N source files were not parsed: `<path>` (deadline), …" and I211
  "N source files parsed with syntax errors: `<path>` (E errors), …", each naming its first 8 paths, kept with the
  call's denials. The call's `SyntaxWork` adds up across its requests for the caller to read.
- A language no grammar answers fails `no_grammar` naming the three; a query fault fails `invalid_query` with its row,
  column and reason. A pure call kind is `denied(call-kind)`.
- Revision reads are kept though no R2 pack asks for one: they fall out of `git.blobs`/`git.tree`, and a later LSP
  view of a file at HEAD and history views use them; leaving them out would change the request's shape later.

## D2. A collector with no command and no report reads through its ports

A collector that names no process capability, no report and no `check_outcomes` input reads through its ports: its
evidence source is the ports (`Source::Ports`), it is called under a run and under `--no-run` alike, its result says
`ran: false`, and finding nothing is not E045 (what it read is its answer). `--report` naming it is E058 ("reads no
report"). It needs no approval. The default report `.specforge/reports/<name>.json` applies only to a collector with a
command. A `capture` on a collector with no command is W021.

## D3. `answers`, the `facts` input, the order, E119

- **A collector declares the collected kinds it answers** (`CollectorDescriptor.answers`, SDK
  `CollectorBuilder::answers(&["item"])`). An `answers` kind that no loaded extension declares collector-written is
  W021.
- **A collector may declare a `facts` input** (`{"input": "facts", "kinds": ["package"]}`, SDK
  `Input::facts(&["package"])`), served to a collector only. W021: empty `kinds`, a kind no loaded extension declares
  collector-written, a second `facts` input.
- **Serving.** `CollectInput.facts` carries, sorted by kind and id, for each kind of the input: the admitted facts of
  that kind that collectors which ran earlier in this collect answered; a kind none of them answered is served from
  the recorded stores (the stores the ledger records). `--runner rust-source` reads the stored packages. No closure
  pulls producers into a named collect: `--runner` runs what it names.
- **Order.** The selected collectors run in a stable topological order over the edges producer → consumer (a kind in
  one's `answers` and the other's `facts`), declaration order between independent ones. A collector never waits on
  itself: a kind it both reads and answers adds no edge, and unless a collector that ran before it answered that kind,
  it is served the kind from the recorded stores, its own last answer among them. A cycle is **E119**, naming the
  collectors of the cycle: reported by the registry build over the loaded collectors, and refused by `collect` over
  its selection before anything runs.

## D4. Admission by `answers`

A fact of a kind its collector does not list in `answers` is refused with E116, and none of the collector's facts is
stored. `answers` is therefore truthful, and the order reads it.

## Rejected

- **A `syntax` port with its own capability kind.** Parsing reads files the fs or git scope already judges; a second
  scope would be a second approval surface for the same bytes.
- **"Consumers last" instead of `answers`.** Two consumers in a chain (`@rust/source`, then a pack reading items)
  would be unordered.
- **Pulling producers into a named collect.** `--runner` would run collectors nobody named; the recorded stores
  already hold what the producer last answered.
