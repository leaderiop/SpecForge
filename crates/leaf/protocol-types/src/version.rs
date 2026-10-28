//! The protocol version the host and the guests speak, and what each minor added.

/// Protocol version for the extension wire format (semver). The host loads
/// every guest of its major version. The minor moves when a payload's
/// values change meaning or an optional field is added; the major moves
/// only when an older guest could no longer be decoded or answered.
/// `docs/extension-protocol.md` ("Protocol versions") lists what each
/// version guarantees; 1.2.0 is ADR 0050's: the declared categories are the
/// only describe categories, and the handshake says `providers` instead of
/// carrying contribution flags; 1.3.0 is ADR 0060's: an enhancement declares
/// no edge types (an edge type is declared once, in `edges`); 1.4.0 is
/// ADR 0069's: a recorded test's status is `passed`, `failed` or `skipped`,
/// and a pass's test results carry `duration_ms`; 1.5.0 is ADR 0094's: an
/// entity's `verify` statements are no field text, so a pass's and a validator's
/// `fields` carry no `verify` entry; 1.6.0 is ADR 0234's: a field's type may be `date`, and
/// the primitives a validator gets include `date`; 1.7.0 is ADR 0284's: a kind may declare the
/// reference field its entities are scoped by (`scoped_by`), and a pass's entity ids and edges are
/// paths; the same 1.7.0 is ADR 0285's: a field's type may be `enum_list` or `block_list`, and a field
/// may declare `members`, `target_kinds`, `minimum`, `maximum` and `file_base`; and the same 1.7.0 is ADR 0286's: an extension catalogs its codes (`codes`), and a
/// rule's `description` (added under 1.6.0 without a bump, ADR 0278) is gone, superseded by `codes`; 1.8.0 is ADR
/// 0287's: a field declares who writes it (`writer`) and its history (`immutable`, `append_only`), a kind may be
/// `append_only`, a pass declares `phase: "compute"`, the fields it `computes` and the `baseline` kinds it reads
/// and answers `values`, and a pass's `previous` is the git baseline; the same 1.8.0 is ADR 0288's: an extension declares typed `settings`
/// (a new describe category), and a call input may carry the extension's own `settings`; 1.9.0 is ADR 0289's: a command or collector may
/// `writes` and answer an edit `plan` the host applies, a kind or field may be written by a `human` only, a pass may be
/// a `guard`, and a writing command's input carries the `revisions` of the sources it was compiled from; the same 1.9.0 is ADR 0290's:
/// a rule may be `judged` (it holds while a fresh verdict says so), a field may be written by a `verdict`, and a pass
/// entity and a command's graph node carry the `judgements` of the rules that judge them; 1.10.0 is ADR 0291's: an
/// extension declares `prompts` in its surfaces and the host calls their `prompt__*` export, a command declares the
/// `gates` it waits on and the `subject` that scopes them, a pass declares the codes it `reports`, and a guard pass's
/// input says what `trigger`ed it; ADR 0292 adds to the same 1.10.0 the `agents`, `skills`, `slash_commands` and `hooks` of an
/// extension's surfaces, the `render__` and `hook__<id>` exports, and the `state` a hook or command keeps.
pub const PROTOCOL_VERSION: &str = "1.10.0";
