# SpecForge Infer — Progress Tracker

## Status: ALL PHASES COMPLETE

| Phase | Description | Status | Notes |
|-------|-------------|--------|-------|
| 7 | Spec files | DONE | Written + updated with expert review fixes |
| 1 | Inference manifest types | DONE | 11 tests pass, 0 clippy warnings |
| 2 | MCP tools (progress + session) | DONE | 10 tests pass, 0 clippy warnings |
| 3 | CLI infer-status command | DONE | --gaps, --stale, --format json |
| 4 | Prompt plan + workflow scopes | DONE | plan + workflow scopes, 5 new tests |
| 5 | Inference gap analysis | DONE | Rust pub item scanner, MCP tool, CLI flag |
| 6 | Inference quality diagnostics | DONE | I200, I202 + --lint=inferred |

## Expert Review Results (2026-04-24)

10 experts reviewed. All issues addressed:
- source_root → source_roots (Vec) for workspace support
- Summary computed-on-load, not persisted
- Sessions deferred to Phase 2
- MCP tool split into infer_progress (read) + infer_session (mutate)
- scope: "workflow" prompt added
- Phase 5 renamed to "gap analysis", language heuristics scoped to Rust v1
- InferenceGap structured type (name, item_kind, file, line)
- Density threshold configurable
- Stale header comment fixed
- specforge-infer.json excluded from graph exports (invariant documented)

## Detailed Task Tracking

### Phase 7: Spec Files — DONE
- [x] Write `spec/types/infer.spec`
- [x] Write `spec/behaviors/infer.spec`
- [x] Write `spec/features/infer.spec`
- [x] Expert review (10 agents)
- [x] Apply all fixes from expert review
- [ ] Re-validate with `specforge check` — zero errors

### Phase 1: Inference Manifest Types — DONE
- [x] Add `InferenceManifest` struct to specforge-common (version, source_roots, source_index)
- [x] Add `SourceFileEntry` struct (path, content_hash, entities_produced, analyzed_at)
- [x] Add `InferenceSummary` struct (computed, not stored)
- [x] Implement `load_inference_manifest()` — default on missing, version check
- [x] Implement `save_inference_manifest()` — atomic write, sorted Vec, no summary
- [x] Implement `compute_summary()` from source_index
- [x] Implement `compute_content_hash()` (SHA-256)
- [x] Implement `detect_stale_entries()` (stale + deleted detection)
- [x] Implement `upsert_source_entry()` (idempotent, keeps sorted)
- [ ] Add `specforge-infer.json` to `.gitignore` in `specforge init` (deferred to Phase 3)
- [x] Unit test: round-trip serialization
- [x] Unit test: summary computation
- [x] Unit test: missing file returns default
- [x] Unit test: source_index sorted after upsert
- [x] Unit test: upsert replaces existing entry
- [x] Unit test: content hash is SHA-256
- [x] Unit test: stale and deleted detection
- [x] Unit test: unsupported version rejected
- [x] Unit test: save and load round-trip
- [x] Unit test: empty source_index not serialized

### Phase 2: MCP Tools (Progress + Session) — DONE
- [x] Add `InferenceSession` struct (session_id, started_at, ended_at, agent, status)
- [x] SessionStatus as string field (active, paused, completed) — simpler than enum
- [x] Register `specforge.infer_progress` tool in `default_tools()` + JSON Schema
- [x] Implement infer_progress handler (load manifest, discover files, compute summary)
- [x] Register `specforge.infer_session` tool in `default_tools()` + JSON Schema
- [x] Implement start action (create session, enforce single-active)
- [x] Implement mark_analyzed action (compute hash, record entry)
- [x] Implement end action (set status, timestamp)
- [x] Source file discovery (walk source_roots, respect .gitignore)
- [x] SHA-256 content hash computation
- [x] Unit tests per action (10 tests)
- [x] Integration test: full lifecycle
- [x] Fix: write_sessions_to_manifest preserves both manifest + sessions atomically

### Phase 3: CLI infer-status Command — DONE
- [x] Add subcommand to clap definition
- [x] Implement default summary view
- [x] Implement `--format json`
- [x] Implement `--gaps` flag (grouped by directory with counts)
- [x] Implement `--stale` flag
- [x] Handle missing specforge-infer.json gracefully (default manifest)
- [ ] Integration test (deferred — manual validation done)

### Phase 4: Prompt Plan + Workflow Scopes — DONE
- [x] Add `Some("plan")` match arm in prompt dispatch
- [x] Implement plan scope (kind priority, target_spec_directory, progress, unanalyzed/stale)
- [x] Add `Some("workflow")` match arm in prompt dispatch
- [x] Implement workflow scope (step-by-step protocol, tool names, retry pattern)
- [x] Integrate with specforge-infer.json (plan reads manifest for progress + unanalyzed)
- [x] Unit tests: plan kind priorities, target directory, progress; workflow protocol, tools/kinds

### Phase 5: Inference Gap Analysis — DONE
- [x] Rust pub item scanner (pub fn, pub async fn, pub struct, pub enum, pub trait, pub type, pub const, pub static)
- [x] Test file / build script exclusion
- [x] Cross-reference with graph entities via source_index + snake_case matching
- [x] InferenceGapReport construction with per-directory breakdown
- [x] MCP tool `specforge.infer_gaps` registration + handler
- [x] CLI `--gaps-detail` flag
- [x] Mark output as approximate
- [x] Unit tests (8 new: scanner, async fn, comments, gaps, snake_case, test exclusion, to_snake_case)

### Phase 6: Inference Quality Diagnostics — DONE
- [x] I200: stale source anchor detection
- [x] I202: high inference density detection
- [x] Configurable density threshold (inference.density_threshold in specforge.json)
- [x] `compute_inference_diagnostics()` in specforge-common
- [x] `--lint=inferred` flag on `specforge check`
- [ ] `"lint": ["inferred"]` in specforge.json (deferred — config-driven profiles not yet implemented)
- [x] Unit tests: I200 stale, I202 high density, I202 below threshold (3 tests)

## Session Log

| Date | Session | Work Done |
|------|---------|-----------|
| 2026-04-24 | Initial | Created master plan, progress tracker, 3 spec files |
| 2026-04-24 | Review | 10-expert review, applied all fixes to plan + specs |
| 2026-04-24 | Phase 1 | Inference manifest types in specforge-common (11 tests) |
| 2026-04-24 | Phase 2-6 | MCP tools, CLI, prompts, gap analysis, diagnostics (all phases) |
