# A recorded source path has one reader

**Status:** accepted (2026-10-26, architecture round 28, plan 08). Extends [ADR 0253](0253-a-source-code-path-a-request-names-has-one-reader.md) D1 to recorded paths and meets its reopen clause; amends [ADR 0113](0113-inference-sources-are-walked-by-the-shared-discovery-policy.md) D5 (one key rule) and clarifies [ADR 0199](0199-a-source-item-is-covered-by-the-anchor-that-names-it.md) D4 ("outside the root").

## Context

Since ADR 0253 a source-code path a request names is read from the project root: an absolute path under the root is that file. A path a manifest records was still read by four other rules at nine sites: `SourceKey::of` (strict, no root), `SourceKey::root` (strict), `normal_path` (lenient, as spelled) and `match_file` (lenient, by components). Reproduced on the base binary:

- an anchor recorded `<root>/src/lib.rs` was `suffix_path` for `find_spec_for_source` and uncovered for `infer_gaps`;
- an index entry recorded `<root>/src/lib.rs` was `deleted` and, at once, `unanalyzed`; marking the file added a second row (`Files: 2/1 (200%)`);
- a source root recorded `<root>/src` walked nothing, though `infer_session start` writes such a root as its key;
- `files_analyzed` counted index rows, so an index holding only a deleted file read `Files: 1/1 (100%)` beside an unanalyzed file.

```text
BEFORE  anchor.file  ─┬▶ CoveredItems::of ── SourceKey::of       index path ─┬▶ indexed ── SourceKey::of
                      └▶ anchors_of_file ─── match_file(text)                ├▶ unanalyzed/upsert ── normal_path
        source_roots ───▶ walk ──────────── SourceKey::root                  └▶ files_analyzed ── rows
AFTER   request, anchor.file, scanner file, index path, source_roots ──▶ SourceKey::read / read_root(root, named)
```

## Decision

- **D1. One reader, `SourceKey::read` / `read_root(root, named)`** in the base module `source_key`: the mint a named spec file uses, then the source path rule. The project view, the inference manifest's readers (index entries, source roots, the mark's upsert) and the anchors read view (covered items, `anchors_of_file`) all call it. `normal_path` is deleted.
- **D2. A recorded path that is no key (`Unkeyed::Outside` or `Nothing`) is treated as no key:** it covers nothing, such an index entry is `deleted`, such a source root is not walked. Navigation still lists an anchor whose key lies outside the root, matched by its recorded text (ADR 0199 D4, ADR 0140 D3), where "outside" means "its key lies outside the project root".
- **D3. Progress reads each index entry once** (`Indexed::key`), and `files_analyzed = files_total - |unanalyzed|`: the inference sources the index records, current or changed. It never exceeds `files_total`.
- **D4. A recorded `..` is applied where it stands** (`src/../src/lib.rs` is `src/lib.rs`), as the request reader does. A `..` whose key leaves the root is still no key, so ADR 0113 R4 stays closed.

## Rejected

Keeping `SourceKey::of`'s lexical refusal and dropping such anchors from navigation too (an agent that writes absolute anchors gets "none" everywhere); rewriting entries at write (fixes only `specforge-infer.json`, only after the next write, and the anchors manifest has no writer here); counting only current files, or distinct keyed rows, as analyzed (the three numbers no longer sum, or the count can still exceed the total); keying the `file` a reply echoes (it is the manifest's own row, and every `./`-spelled manifest would change for no defect).

## Consequences

Breaking: `summary.files_analyzed` in `infer_progress`, `infer-status` and the infer plan scope; a manifest holding an absolute (or `..`-inside) spelling under the root now reads as that file (current or stale instead of deleted, a root walked, an anchor `exact` / `directory` and covering its item). Rust, `specforge-ops`: `InferenceManifest::upsert_source_entry` gains `root`; `compute_summary` and `normal_path` are removed; `InferenceSummary: Default`. No file on disk is rewritten: the next `mark` replaces a row with the key. No re-vendor. Reopen it if a writer of the anchors manifest appears (it would write keys) or if a scanner reports paths of its own.
