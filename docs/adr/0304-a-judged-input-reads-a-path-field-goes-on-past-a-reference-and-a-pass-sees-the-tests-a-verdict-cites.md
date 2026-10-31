# A judged input reads a path field, goes on past a reference, and a pass sees the tests a verdict cites

**Status:** accepted (2026-10-31, kernel round K20a, plan 13). Extends
[ADR 0290](0290-a-judged-rule-holds-while-a-fresh-verdict-says-so.md) and
[ADR 0293](0293-a-pass-or-command-declares-the-project-files-it-reads.md). Takes extension protocol 1.12.0.

## Context

A `file()` judged input had to end at a `file_reference` field, but gsd keeps plan, artifact and key-link paths plain
strings (E016), so E501 read no code file. A referrer selector could not go on past its reference, so E518 read every
requirement. A pass was not told the tests a verdict cites, so a rule over them could not count them.

## Decision

- **D1. A referrer step goes on.** `kinds<path` whose `path` goes on past its reference is split at the reference's
  end: the prefix finds the referrers, the rest is a forward path from each. No grammar or canonical text change, so
  existing digests are unchanged. W112 "goes on past a reference" goes for that shape.
- **D2. `file()` may end at a `string` or `string_list` field**, resolved from the project root; a missing file digests
  `absent`.
- **D3. `Judgement.cited_tests: Vec<String>`** (default, skipped when empty), filled from the verdict's cited tests.
- **Protocol.** 1.12.0 (one optional field).

## Rejected

A new `path` field type or flag (a wire change, and it re-types fields gsd keeps plain because of E016).

## Limits

A string that is not a project-relative path digests as an absent file.

## Consequences

E501 reads the files its truths, artifacts and key links name; E518 reads its milestone's requirements only.
