# A source-code path a request names has one reader

**Status:** accepted (2026-10-26, architecture round 27, plan 11). Reopens [ADR 0195](0195-a-spec-file-a-request-names-is-read-into-its-source-key-once.md) "Left alone: `match_file`, infer's `SourceKey`" and amends [ADR 0092](0092-a-path-an-mcp-request-names-resolves-inside-the-served-project.md) D5 (an ops operation no longer keeps its own outside refusal), [ADR 0113](0113-inference-sources-are-walked-by-the-shared-discovery-policy.md) D1/D5 (a request's source path is read by the project view) and [ADR 0140](0140-source-anchors-are-a-read-view.md) D1 (`anchors_of_file` refuses a path outside the project).

## Context

A spec file a request names has one reader since ADR 0195. A source-code path a request names (infer_session's
`source_file` and `source_roots`, find_spec_for_source's `file_path`, the infer prompt's `file:` scope) was read by two
other rules that disagreed with it and with each other: inference's `SourceKey` took no root and refused every absolute
spelling, even one inside the project, saying it was outside; `match_file` had no outside rule, so a path outside the
project answered `{"count":0,"match_mode":"none"}` (and the prompt told the agent to infer from `/etc/hosts`), against
the MCP invariant that such a path is refused as `invalid_input` on its argument. The outside fact was worded four ways.

```text
BEFORE  source_file ─▶ SourceKey::of (no root)      file_path ─▶ match_file (no rule)    spec file ─▶ ProjectView::source
AFTER   source_file ─┐                                file_path ─┤                      spec file ─▶ ProjectView::source
        source_roots ┴▶ ProjectView::source_path / source_root ◀┘ (one mint, one sentence shape)
```

## Decision

- **D1. `ProjectView::source_path` / `source_root` read a source-code path from the project root** by the mint a named
  spec file uses (`minted`: a `..` applied where it stands, an absolute path under the root its relative key), then by
  the source path rule (`SourceKey`, moved to the base module `source_key`: the view may not use `read`, ADR 0086). A key
  outside the **project root** (a spec root outside the project holds no source code) or one that names nothing is
  refused once, `not_a_source_path`, with no file opened.
- **D2. find_spec_for_source and the infer file scope read their query through `source_root`.** A path outside the
  project, or an empty one, is `invalid_input` on `file_path` (on `scope` for the prompt); an absolute spelling of an
  anchored file matches it. `anchors_of_file` returns `Refusal<FileAnchorsField>`; the reply's `file_path` stays the query
  as asked (it may be a suffix pattern).
- **D3. Two facts, two sentences, each written once in ops.** A key that lies outside: `lies_outside`,
  `'<named>' lies outside <roots>: <what> is named from the project root` (the spec reader says it for its two roots, the
  source reader for the project root). A name that lands outside when opened: `From<Outside>`'s `'<named>' resolves to
  <landed>, outside <root>, which bounds the request`, now also for MCP's call target (its own sentence is deleted) and
  the infer mark through a link. Both quote the caller's spelling; render's `out_dir` refusal no longer quotes the
  root-joined name.
- **D4. A marked directory is `invalid_input` on `source_file`** (`source_unreadable`), not `internal_error`.

## Rejected

Reusing `Containment` for the key (a link inside would key by its target, and find_spec_for_source's query is a pattern a
resolve turns into a missing file); keeping the absolute refusal and fixing only its sentence (leaves find_spec_for_source
unread); one sentence for both facts (the view would open the file system for a name it only keys, or the landing
refusals would drop the landing ADR 0092 D5 asks for); keeping `none` for a path outside.

## Consequences

Breaking, MCP: infer_session accepts an absolute `source_file` / `source_roots` entry under the root, its refusal messages
and codes change (`source_outside_root`, `source_root_outside_root` become `not_a_source_path` and `outside_root`; op
codes are not on the wire); find_spec_for_source and the infer `file:` scope refuse a path outside the project and an empty
`file_path`; render's and every call target's `path` refusal change text. Rust: `read::infer::{SOURCE_OUTSIDE_ROOT,
SOURCE_ROOT_OUTSIDE_ROOT}` removed, `view::NOT_A_SOURCE_PATH` and `read::navigate::FileAnchorsField` added. No re-vendor.
Reopen it when a recorded path (a manifest entry, an anchor file, a scanner path) must be read from a root, or when a CLI
command names a source-code path.
