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
///   (ADR 0350).
///
/// A change that adds to 2.0.0 (grants, host imports, value checks) adds its item here and to the 2.0.0 row of
/// `docs/extension-protocol.md` ("Protocol Versions"), which also keeps the 1.x history, 1.15.0 included; it does
/// not bump. A 2.0.0 describe category goes after `routes`.
pub const PROTOCOL_VERSION: &str = "2.0.0";
