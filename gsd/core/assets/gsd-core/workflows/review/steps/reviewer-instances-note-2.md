<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/review/steps/reviewer-instances-note-2.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Reviewer instances (#1517, optional):** instances resolve *through* a lane and are not lanes
themselves (ADR-2782 D8). Each selected instance invokes its base `cli` with its own `model`/`agent`
as opaque argv. Exact invocation in `gsd-core/references/reviewer-instances.md`.
