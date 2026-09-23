# 076 — Jason Liu

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** Instructor creator; structured outputs and typed LLM interfaces
**SpecForge anchors:** typed exports for agent consumption (emitter Json format, schema/ Graph Protocol, export --format)

## Why this engineer
Liu's Instructor made "Pydantic model in, validated object out" the default contract for LLM calls, complete with retries that feed validation errors back to the model. SpecForge's exports make the same bet at project scale: the Graph Protocol JSON Schema (draft 2020-12) and typed entity kinds are the repo's Pydantic models, so agents consume spec data as typed objects instead of parsing prose. He is the ergonomics reference for schema-first machine interfaces — field descriptions that teach the model, schemas that fail loudly and specifically.

## References for SpecForge
**Key works**
- [567-labs/instructor](https://github.com/567-labs/instructor) — GitHub, 2023. Structured extraction with validation-and-retry; the interaction pattern behind schema-first agent consumption.
- [jxnl.co](https://jxnl.co) — his writing and interviews on structured outputs, evals, and context design (e.g. why Cognition avoids multi-agents).
- Bad schemas could break LLMs — instructor blog, 2024. Schema-design failure modes to avoid when evolving the Graph Protocol schema.
- [pydantic/pydantic](https://github.com/pydantic/pydantic) — GitHub, 2017. The validation core Instructor builds on; the role SpecForge's emitted schema plays for agents.

## Study first
1. Validation-error feedback loops: Instructor retries vs PRD-007's validate step
2. Field descriptions as prompts — EntityKindDescriptor guidance for agents
3. Evolving a public schema without breaking agent consumers
