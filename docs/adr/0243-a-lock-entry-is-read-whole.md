# A lock entry is read whole

**Status:** accepted (2026-10-25, architecture round 27, plan 01). **D3 amended by [ADR 0265](0265-a-registry-reply-is-read-as-values-where-it-is-read.md) (round 28):** a preview whose published digest is no SHA-256 is the reply's R009, not R-OPS-002; `Sha256` lives in `specforge-package`. Extends [ADR 0230](0230-a-lock-source-is-checked-once-and-a-remedys-command-quotes-its-values.md) D1 and [ADR 0036](0036-one-package-name.md) D1; supersedes [ADR 0201](0201-a-locked-package-is-fetched-to-the-pin-its-lock-entry-records.md) D4 and ADR 0036 A2; amends [ADR 0028](0028-installed-extensions-are-one-module.md) (W149).

## Context

- ADR 0230 read a lock entry's source where the lock is read; its version, hash, key id and peers stayed text. A lock a
  pull request edits wrote ESC/OSC sequences through E070 (twice), R014, R-TRUST-007, R-TRUST-003, the KEY CHANGE
  prompt, the extension listings and update's rows; `doctor` panicked on a non-ASCII hash (exit 101).
- Two integrity compares were spelling compares: the digest in uppercase was "changed after install" (E070) and R014;
  an empty one turned the hash check off (W149, exit 0).
- `add` recorded a local module's version unchecked (`0.1.x`), so `update` and the reinstall command re-judged it.

```text
BEFORE  name ✓  source ✓  version / wasm_hash / key_id / peers: text ─► 13 sinks, 7 compares, each its own rule
AFTER   every field a value, else E033 for the whole lock (escaped) ─► compares compare values, messages print values
```

## Decisions

- **D1.** A lock entry is read whole where the lock is read: its version a SemVer version, its wasm_hash a SHA-256 in
  lowercase hex (64 characters), its key_id a publisher key id (16 lowercase hex characters), its source `registry` or
  `local:<path>`, and its source path and recorded peers holding no character `remedy::printable` escapes (control and
  bidirectional control characters). Anything else makes the whole lock unreadable (E033, its one remedy), the text
  shown escaped.
- **D2.** A lock entry pins a hash: an entry without one is unreadable; W149 is retired to E033. (ADR 0201 D4
  superseded.)
- **D3.** `add` never writes what the read refuses. The entry's types forbid an invalid version, hash or key; a local
  module declaring a non-SemVer version or a peer the lock cannot record is E072, before anything is written; a registry
  package signed by a key that is no key id, or declaring such a peer, is E028; a preview whose published digest is no
  SHA-256 is R-OPS-002, as its download would be.
- **D4.** No lock version is unreadable: `LockedVersion` goes (ADR 0036 A2 superseded).
- **D5.** Peers are held to printable text, not to their grammar: the peer rule (ADR 0041) reports the grammar.

## Rejected

- Escaping at each sink (nine `printable` calls): the next sink forgets; ADR 0231 D4's "per site" rule is what let these
  through.
- Accepting an uppercase digest (two spellings on disk), a per-entry refusal (ADR 0230), a new code (E082).
- Typing peers by grammar at the read: it would refuse locks `add` writes for an optional peer.

## Consequences

- An existing lock with an empty, uppercase or malformed hash, a non-SemVer version, a key id of another form or an
  unprintable peer is E033 on check, doctor, the LSP and MCP until it is fixed, or deleted and the extensions added again.
- A local module declaring a non-SemVer version can no longer be `add`ed; it can be enabled as a `.wasm` file entry.
- `doctor` can no longer panic on a lock hash.
- Breaking in Rust: `Sha256`, `KeyId`, `recordable_peers`, `hex_sha256 -> Sha256`, `Module::digest -> &Sha256`,
  `LockedPin { bytes, key_id }`, `LockedVersion` removed; no re-vendor.

## What would reopen it

- A second hash algorithm or a lock format version 2: `Sha256` becomes a digest enum, still read at the lock.
- A registry key id of another form (`specforge_registry_wire::signature::key_id` changes): `KeyId` follows it.
