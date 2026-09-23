# 075 — Shreya Rajpal

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** Guardrails AI co-founder/CEO; LLM output validation
**SpecForge anchors:** validating agent outputs against spec contracts (specforge validate, validation_engine, PRD-007 validate loop)

## Why this engineer
Rajpal built Guardrails — deterministic validators that check every LLM output against declared schemas and rules, with structured re-ask loops on failure. PRD-007's agent workflow (write .spec files, call `specforge_validate`, fix, repeat) is the same loop with SpecForge's declarative validation_engine as the guard. Her core argument — deterministic checks for structural and safety properties, not judge-model vibes — is exactly SpecForge's design: orphans, missing fields, and dangling file refs are guard violations, and validation errors are the forcing function of agent iteration.

## References for SpecForge
**Key works**
- [guardrails-ai/guardrails](https://github.com/guardrails-ai/guardrails) — GitHub, 2023. Declared validators, deterministic checks, retry loops — closest prior art to validate-in-the-loop.
- Guardrails Hub — Guardrails AI, 2024. Composable, shareable validator packages; the model for extension-contributed validation rules.
- Building Guardrails for Enterprise AI Applications — InfoQ talk, 2024. Her thesis: deterministic structural validation over stochastic review.
- Guardrails AI joins Harvey — guardrailsai.com, 2026. Validation consolidating into enterprise stacks; a positioning signal for SpecForge's contracts.

## Study first
1. Guardrails' re-ask loop vs PRD-007's validate-fix-repeat cycle
2. Validator packaging (Hub) vs SpecForge extension-declared validations
3. Deterministic checks vs LLM judges for spec conformance boundaries
