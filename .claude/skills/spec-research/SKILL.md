---
name: spec-research
description: "Author research and exploration documents in a spec's research/ directory. Each file captures findings with RES-NN IDs from investigating tools, patterns, or technologies that inform the spec. Use when documenting exploration findings, technology evaluations, or pattern investigations."
---

# Spec Research

Rules and conventions for authoring **research and exploration documents** in a spec's `research/` directory. Research files capture findings, analysis, and recommendations from investigating tools, patterns, or technologies. They are inputs to the spec, not part of the formal contract.

## When to Use

- Documenting exploration findings that inform spec decisions
- Evaluating tools, technologies, or patterns for adoption
- Capturing analysis that may lead to ADRs or new behaviors
- Recording investigation results for future reference

## Directory Structure

```
research/
  index.yaml                    # Manifest of all research files
  RES-01-<topic>.md             # One file per research topic
  RES-02-<topic>.md
  ...
```

### index.yaml Schema

```yaml
kind: research
package: "@hex-di/<name>"
entries:
  - id: RES-01
    file: RES-01-agent-teams-orchestration.md
    title: Agent Teams Orchestration
    status: active              # active | draft | deprecated | superseded
    outcome: adr                # adr | behavior | deferred | rejected
    related_adr: ADR-003        # if outcome is 'adr'
  - id: RES-02
    file: RES-02-mcp-tool-ecosystem.md
    title: MCP Tool Ecosystem
    status: active
    outcome: deferred
```

**Rules:**
- Every `.md` file in `research/` MUST have a corresponding entry
- Every entry MUST have a corresponding file on disk
- No duplicate `id` values across entries
- `outcome` tracks what the research led to

## File Naming

- ID-prefixed: `RES-NN-<topic>.md`
- Two-digit prefix: `01-`, `02-`, etc.
- Kebab-case topic name
- Examples: `RES-01-agent-teams-orchestration.md`, `RES-05-mcp-tool-ecosystem.md`

## Content Rules

1. **YAML frontmatter** — Every research file MUST start with `---` frontmatter containing `id`, `kind: research`, `title`, `status`, `date`, `outcome`, `related_adr`.
2. **Exploration vs formal contract** — Research files are inputs to the spec, not formal contracts. They capture findings and recommendations, not MUST/SHALL statements.
2. **Outcome tracking** — Each research document should track its outcome: did it lead to an ADR, new behaviors, was it deferred, or rejected?
3. **ADR linkage** — When research leads to an ADR, link to the resulting decision.
