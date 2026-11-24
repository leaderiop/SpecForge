//! The protocol version the host and the guests speak, and what each minor added.

/// Protocol version for the extension wire format (semver). The host loads every guest of its major version whose
/// minor is not newer than its own; a guest of another major, older or newer, is refused (ADR 0326: E088, saying which
/// side is behind). The minor moves when a payload's values change meaning or an optional field is added; the major
/// moves only when an older guest could no longer be decoded or answered.
///
/// 2.0.0 is ADR 0337's. A 1.x guest is refused as older. Its items:
/// - a pass, command, prompt, hook or collector declares what the host serves it as one tagged list, `inputs`
///   (`files`, `tests`, `state`, `graph`, `slash_commands`, `git`, `setting`), replacing the nine `reads*` keys;
/// - a hook's `git` input with `status` carries a typed `GitStatus`, not porcelain text;
/// - a `codes` entry may declare `machine` (`CodeDescriptor.machine`): this machine decides its finding (its
///   approvals, its network or its effects journal), so it never fails `--strict` or the gate's warning baseline;
/// - an extension declares `capabilities` (a describe category): what it may make the host do, each on one port under
///   a scope name, and a collector names the process `capability` it runs instead of carrying `run` (ADR 0335 D2);
/// - a command's input carries `nonce`, 32 hex digits of the host's entropy, fresh per call (ADR 0341);
/// - a guest may import `specforge:host/syntax`: the host parses source files with a pack's query (ADR 0344, 0345);
/// - a collector may declare a `facts` input (`kinds`), and `CollectInput.facts` carries the facts of those kinds that
///   earlier collectors answered or the recorded stores hold (ADR 0345 D3);
/// - a pass may declare the `facts` input: a pass is sent the authored entities and the facts of the kinds it names,
///   and only the edges between what it is sent; one that declares none is sent no fact (ADR 0353);
/// - a collector declares `answers`, the collected kinds whose facts it answers: they order the collect (E119 for a
///   cycle) and admit its facts (E116 otherwise);
/// - a collector with no `capability` and no `report` reads no report: it is called with no reports and reads through
///   its ports (ADR 0345 D2);
/// - a fact carries `at` (where it is written) and `named_at` (where each name a field holds is written), and a pass
///   entity carries `dangling`, a fact's references that name no entity, the wrong kind or several, judged by the
///   extension declaring the holder's kind (ADR 0346);
/// - an unheld reference carries `kinds`, the collected kinds of its scheme its field admits, and a fact store
///   `held`, the kinds of its facts by scheme, replacing `schemes` (ADR 0347 D9);
/// - a field may declare `checks_while` (`complete`, `pending`), and a check row's status may be `planned` or `closed`
///   (ADR 0350);
/// - the source scanners' describe category and their operation are gone: source code reaches the graph as the facts
///   a source pack collects (ADR 0199).
/// - a `process` capability may declare one `config` (`name`, `template`): the host writes it before a collector's
///   run, `{report}` expanded, and `{config}` in the argv names it; a collector may declare an `install` line, said
///   when its program is not found (ADR 0348).
///
/// 2.1.0 adds: a contribution's `tools` may name `specforge:<tool>`, `specforge:<group>` and
/// `specforge:<short>.<command>` (ADR 0334); `specforge_protocol_types::engine_tools` is the catalogue they name.
/// An extension declares `contexts`, the fifteenth describe category, read last: for one entity of a subject kind,
/// the ordered sections a role reads, each a named selector, core or enrich (ADR 0356).
/// A selector kind may carry a field filter, `kind[field=value|…]`, in judged inputs, context sections and route
/// steps: only entities whose authored scalar field holds one of the values (ADR 0356 D13).
/// A field may declare writer `context`: only the host's record of a context route's pack writes it, ledgered and left
/// out of every digest (ADR 0356 D9). A context route may declare `records` (its `context` fields a pack is recorded
/// in, each with a subject chain and a suggestion) and `stale`, the code a record whose pack moved raises (D10).
/// An edit plan may `record_context` a route's pack on an entity; the host computes and writes it (D11).
/// A 2.0.0 guest still loads, being an older minor.
///
/// 2.2.0 adds:
/// - a kind field or a block member may be `number`, an exact decimal of at most 15 significant digits (ADR 0361),
///   written as one decimal word and carried as a JSON number; a `number` field's `minimum` and `maximum` are
///   decimals. [`Decimal`](crate::Decimal), `GraphNode::number` and `PassEntity::number` read one;
/// - a judged target may declare `judged_while`, the owner state its entities are judged in (ADR 0362): an entity in
///   the other state owes no verdict, decided after the compute passes;
/// - a `codes` entry may declare `reported_while`, `complete` or `pending` (ADR 0363): its findings speak only on an
///   entity in that owner state, and a typed edit that moves the entity into it reveals one rather than adding it;
/// - a `process` capability may declare `filter` ([`ProcessScope::filter`](crate::ProcessScope::filter)): the
///   arguments the host appends after `argv` only when a caller passes a filter (`specforge collect --filter`),
///   [`FILTER_PLACEHOLDER`](crate::FILTER_PLACEHOLDER) in exactly one of them; no guest request is admitted on such a
///   scope (ADR 0360);
/// - a declared `env` name may end in `*` after a namespace prefix: the program inherits every variable with that
///   prefix, and no guest sets a variable through it (ADR 0360);
/// - a program a guest runs and a collector's command the host runs go through one credential filter: a secret's
///   name and a cargo credential provider are withheld, and a URL's user and password are removed (ADR 0360);
/// - a collector's command told its report path through `{report}` in its argv or config runs with
///   `SPECFORGE_REPORT` removed from its environment; one told it no other way gets it set to its report path (the
///   host's `report_in_env`, ADR 0360).
///
/// A 2.1.0 guest still loads; a 2.2.0 guest is refused by a 2.1 host (E088, newer).
///
/// 2.3.0 adds:
/// - a pass may declare the `delta` input (`kinds`, ADR 0376 D6): its input then carries `delta`
///   ([`DeltaPayload`](crate::DeltaPayload)), what changed of those kinds since a base the host names. The base is
///   `none` (the empty graph, so every node of the named kinds is added), `revision` with the full sha of the commit
///   it is taken against, or `previous`, the graph of a long-lived session's last completed check (D7). Each changed
///   node names its kind, its id and its change; a modified node also names the fields and aspects that changed and
///   its content digests before and after, an added node its digest after, a removed node only its kind and id. The
///   edges added and removed are those with an end of a named kind. The full content stays in `entities`; a pass
///   that declares no `delta` input is sent no `delta`.
/// - a `codes` entry may declare `settled_later` (ADR 0385): a typed edit whose plan creates a finding's subject, or
///   an entity under it, and alters nothing held there adds no error by a finding of that code (E080); its severity
///   does not change.
/// - a pass entity carries `title` ([`PassEntity::title`](crate::PassEntity::title), ADR 0387), the title its header
///   writes, as a command's graph node does; absent for a fact written without one.
/// - a collector may declare `requires`, the other programs its command runs beside its argv's first, which
///   `specforge doctor` looks up on PATH (ADR 0389 D3); absent when empty.
/// - a file read may declare `text_max` with `text` ([`FileRead::text_max`](crate::FileRead::text_max), ADR 0390
///   D3): the most text of one file it is sent, in bytes, instead of 64 KiB, at most 4 MiB; its text budget is then
///   the larger of 1 MiB and four times it. Zero, over 4 MiB or without `text` is W021 and served empty.
/// - a command may declare `imports` ([`CommandDescriptor::imports`](crate::CommandDescriptor::imports), ADR 0392):
///   it imports what was recorded outside the spec, so the host writes a third-party extension's error its plan adds
///   and refuses a first-party one (E016 included); it must `writes`, else W021.
///
/// A 2.2.0 guest still loads; a 2.3.0 guest is refused by a 2.2 host (E088, newer).
///
/// A change that adds to the current minor (grants, host imports, value checks) adds its item here and to that
/// minor's row of `docs/extension-protocol.md` ("Protocol Versions"), which also keeps the 1.x history, 1.15.0
/// included; it does not bump. A new describe category goes last.
pub const PROTOCOL_VERSION: &str = "2.3.0";
