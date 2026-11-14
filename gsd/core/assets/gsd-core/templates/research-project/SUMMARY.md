<!-- Ported from gsd-core v1.19.0 gsd-core/templates/research-project/SUMMARY.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Research Summary Template

Template for the summary of the project research: the executive summary with roadmap implications.

---

## The Summary's Typed Edit

The summary is the `summary` field of the project research `RESEARCH` (a `research` entity in `spec/research.spec`; `specforge schema --kind research` lists its fields), set with `specforge set` once the four dimensions' items are recorded, with the overall `confidence`; nothing writes a summary file. The summary text goes in the quoted heredoc as it is, Markdown included. A refusal names its code: read it with `specforge explain <code>`.

```bash
CONFIDENCE="{high, medium or low: the overall confidence}"
IFS= read -r -d '' SUMMARY <<'SUMMARY_EOF'
{the summary, in Markdown: the sections below}
SUMMARY_EOF
specforge set RESEARCH summary "$SUMMARY" || exit 1
specforge set RESEARCH confidence "$CONFIDENCE" || exit 1
```

**The summary's sections:**

- `## Executive Summary`: written for someone who will only read this section. The key recommendation and the main
  risk, 2-3 paragraphs at most.
- `## Key Findings`: the recommended stack, the expected features (must have, should have, defer to v2+), the
  architecture approach and the critical pitfalls. Name the items (`SC1`, `PF2`) rather than repeat
  them, and keep to what the roadmap's decisions turn on.
- `## Implications for Roadmap`: the most important section, directly informing roadmap creation. Per suggested phase:
  its rationale (why this order), what it delivers, the features it addresses and the pitfalls it avoids, then the
  phase ordering rationale and the research flags (which phases need deeper research during planning, which have
  well-documented patterns).
- `## Confidence Assessment`: honest about uncertainty, per area (stack, features, architecture, pitfalls), with the
  gaps that need resolution during planning. High is verified with official sources, medium is community consensus
  with several sources agreeing, low is a single source or inference.
- `## Sources`: aggregated from the items, by confidence.

The roadmapper reads the summary as context: its phase suggestions start the roadmap, and phase planning follows
where its research flags point.
