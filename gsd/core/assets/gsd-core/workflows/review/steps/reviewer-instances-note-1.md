<!-- Ported from gsd-core v1.18.0 gsd-core/workflows/review/steps/reviewer-instances-note-1.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
**Reviewer instances (#1517, optional):** if `review.reviewer_instances` is configured,
instance names in `review.default_reviewers` run as independent identities. Resolution rules
are in `gsd-core/references/reviewer-instances.md` — load it lazily only when instances are
configured. Unconfigured → default path unchanged.
