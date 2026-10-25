# A download is held to the size its reply declares

**Status:** accepted (2026-10-25, architecture round 27, plan 08). Extends [ADR 0213](0213-trust-is-decided-once-from-the-described-metadata-before-any-download.md) D1
(the download is held to the size the reply declares before it is held to the SHA-256). Amends
[ADR 0044](0044-the-package-registry-contract-is-one-crate.md) D2 (every answer is read through one bounded reader) and D4
(`download` takes the reply, and the client contract gains K14 and K15).

## Context

- The HTTP client read every body whole: the download, every JSON answer, every failure body (`resp.bytes()` three times).
  A reply declaring 10 bytes followed by a 256 MiB download cost 836 MB of resident memory and 21 s before R-OPS-002
  refused the hash; a 64 MiB metadata reply cost 227 MB on `add --dry-run`, which downloads nothing. Memory grew about
  three bytes per byte sent.
- `PackageMetadata::size_bytes` was written by both adapters, asserted by the contract (K4) and read by no client.
- The reference server accepts no upload over 64 MiB (a literal in its router); nothing tied a client to it. The largest
  committed extension binary is 0.76 MB.
- `reqwest`'s `bytes()` is one deadline over the whole body; its `Read` is one deadline per read. A reader built on `Read`
  without its own deadline lets a registry hold an install open for as long as it likes.

```text
BEFORE  download / answer / failure body ── resp.bytes() ── whole, any size        size_bytes ──▶ nothing
AFTER   every body ── body_of(resp, limit, timeout)
          Content-Length > limit ──▶ refused unread     byte limit+1 arrives ──▶ refused    past the deadline ──▶ R004
          limit = download_limit(): size_bytes, at most MAX_PACKAGE_BYTES; MAX_PACKAGE_BYTES when none
```

## Decisions

- **D1.** One bounded reader, `body_of`, reads every registry body: at most a limit, refused as soon as the `Content-Length`
  or the `limit + 1`th byte says it is over, within the client's timeout by its own deadline between reads. A body still
  arriving at the deadline is R004 (was R005).
- **D2.** A download's limit is the `size_bytes` its reply declares, at most 64 MiB, and 64 MiB when it declares none
  (`PackageMetadata::download_limit`). Every other answer is held to 64 MiB. `RegistryClient::download` takes the
  `PackageMetadata` it downloads for, so a caller cannot pass a wrong limit; the in-memory adapter holds the same limit
  and the contract checks it (K14: declared smaller than the binary is R009 naming `size_bytes`; K15: none declared reads).
- **D3.** An answer over its limit is R009 through `InvalidReply`: no new variant, no new code. R009 is "an answer the
  registry protocol does not define", and its message says so.
- **D4.** The ceiling is one constant, `specforge_registry_wire::MAX_PACKAGE_BYTES`, read by the client and by the reference
  server's upload limit.
- **D5.** No streamed SHA-256: the bytes are installed, so hashing while reading saves a pass, not memory. A body shorter
  than `size_bytes` is not refused by the reader; the hash refuses it (R-OPS-002).

## Rejected

- The 64 MiB ceiling only: a reply declaring 10 bytes would still make the client fetch 64 MiB.
- `size_bytes` required and exact: breaks the wire and duplicates what the hash refuses.
- R-OPS-002 through a new `RegistryError::Oversized`, or a new R015: two codes for one rule, or a code for one sentence.
- A tighter ceiling for a JSON answer: a manifest may be as large as an upload.
- `reqwest`'s `stream` feature: `Read` and `content_length` are in the feature already enabled.

## Consequences

- A body past the timeout is R004. An over-size answer is R009 before more of it is read, and a download longer than its
  `size_bytes` is R009 where it was R-OPS-002 after the whole transfer.
- A registry whose `size_bytes` is wrong but whose bytes are right is refused; both adapters serve it exactly (K4).
- The trait's `download` takes `&PackageMetadata`. Nothing new on the wire.

## What would reopen it

- A registry that serves packages over 64 MiB.
- A protocol version that signs `size_bytes`.
