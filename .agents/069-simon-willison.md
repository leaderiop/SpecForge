# 069 — Simon Willison

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** LLM-tooling practitioner-critic; CLI tool design
**SpecForge anchors:** agent ergonomics of CLI tools (specforge-cli's 34 commands, diagnostics, machine-readable output)

## Why this engineer
Willison uses LLM tools daily and writes the sharpest public critiques of their ergonomics — from his `llm` CLI and Datasette to prompt-injection failure taxonomies. SpecForge's CLI is itself an agent surface: 34 commands whose flags, help text, JSON output, and diagnostic codes a model must parse on every turn. Willison is the reviewer who catches the frictions agents pay for in tokens — inconsistent flag shapes, prose-only errors, output that lacks a machine-readable mode — before they harden into the interface.

## References for SpecForge
**Key works**
- [simonwillison.net](https://simonwillison.net) — weblog, 2002-present. Continuous field notes on LLM tool ergonomics, tool-calling pitfalls, and agent failure modes.
- [simonw/llm](https://github.com/simonw/llm) — GitHub, 2023. His CLI for LLMs: consistent subcommands, plugins, pipe-friendly output — the command-shape model for specforge-cli.
- [simonw/datasette](https://github.com/simonw/datasette) — GitHub, 2017. Structured data exposed as queryable JSON; the let-the-machine-read-it ethos behind `export --format=context`.
- LLMs on the command line — conference talk, 2024. The argument that CLIs are a first-class agent interface, not a human leftover.

## Study first
1. Prompt-injection taxonomy: agents acting on untrusted tool output
2. `llm` CLI conventions (help, errors, JSON piping) vs specforge command consistency
3. His posts on structured output vs prose for machine consumers
