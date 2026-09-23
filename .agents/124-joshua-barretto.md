# 124 — Joshua Barretto

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** ariadne & chumsky creator; compiler diagnostics rendering
**SpecForge anchors:** crates/specforge-validator/src/render.rs (ariadne Report/Label rendering — direct dep, ariadne 0.6)

## Why this engineer
Ariadne is SpecForge's direct dependency for turning validator and parse diagnostics into labeled, colorized, source-anchored reports, and Barretto (zesterer) designed its whole model: `Report`/`Label` spans, per-file sources, color/config profiles, deterministic ordering. His chumsky essays articulate why diagnostics must be span-native and error-tolerant rather than string-shaped — exactly the philosophy render.rs should enforce when SpecForge renders orphan checks, file-ref failures, and validation-engine rejections.

## References for SpecForge
**Key works**
- [zesterer/ariadne](https://github.com/zesterer/ariadne) — GitHub, 2021. The diagnostic renderer SpecForge links; Report/Label/Config API is the render.rs contract.
- [zesterer/chumsky](https://github.com/zesterer/chumsky) — GitHub, 2021. Parser-combinator library with error recovery; sibling project whose span model pairs with ariadne.
- Why can't error-tolerant parsers also be easy to write? — blog.jsbarretto.com, 2022. The creator's design essay behind chumsky/ariadne ergonomics.

## Study first
1. Report::build + Label span semantics (offset vs line/col) and multi-file sources
2. Config profiles: colors, character sets — terminal vs machine-readable output modes
3. Deduplication and ordering of overlapping diagnostics before render
