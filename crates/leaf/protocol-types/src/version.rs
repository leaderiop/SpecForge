//! The protocol version the host and the guests speak, and what each minor added.

/// Protocol version for the extension wire format (semver). The host loads every guest of its major version whose
/// minor is not newer than its own; a guest of another major, older or newer, is refused (ADR 0326: E088, saying which
/// side is behind). The minor moves when a payload's values change meaning or an optional field is added; the major
/// moves only when an older guest could no longer be decoded or answered.
///
/// 2.0.0 is ADR 0337's (the ports round). A 1.x guest is refused as older. Its items:
/// - a pass, command, prompt, hook or collector declares what the host serves it as one tagged list, `inputs`
///   (`files`, `tests`, `state`, `graph`, `slash_commands`, `git`, `setting`), replacing the nine `reads*` keys
///   (ports plan 03);
/// - a hook's `git` input with `status` carries a typed `GitStatus`, not porcelain text (ports plan 03);
/// - a `codes` entry may declare `machine` (`CodeDescriptor.machine`): this machine decides its finding (its
///   approvals, its network or its effects journal), so it never fails `--strict` or the gate's warning baseline
///   (ports plan 03);
/// - an extension declares `capabilities` (a describe category): what it may make the host do, each on one port under
///   a scope name, and a collector names the process `capability` it runs instead of carrying `run` (ADR 0335 D2,
///   ports plan 02).
///
/// The rounds that add to 2.0.0 (grants, host imports, value checks) add their item here and to the 2.0.0 row of
/// `docs/extension-protocol.md` ("Protocol Versions"), which also keeps the 1.x history, 1.15.0 included; they do
/// not bump. A 2.0.0 describe category goes after `routes`.
pub const PROTOCOL_VERSION: &str = "2.0.0";
