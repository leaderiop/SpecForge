# 070 — Andrej Karpathy

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** 'LLM OS' thesis; structured context for models
**SpecForge anchors:** README AI-cost thesis ("Why SpecForge? — The AI Agent Cost Problem"), RES-18 token economics

## Why this engineer
Karpathy framed LLMs as CPUs and their context windows as RAM — the "LLM OS" — making context management the operating-system problem of the era. SpecForge's README thesis (agents burn their budget exploring files; structured specs lift first-attempt accuracy from ~30% toward 70-85%) and RES-18's 70-90% reduction arithmetic are direct applications: the compiled spec graph is the memory hierarchy agents should page in, instead of grepping the file tree. Now founding Eureka Labs after OpenAI, Tesla, and his from-scratch teaching repos, he keeps grounding the physics: what window, attention, and token cost actually constrain.

## References for SpecForge
**Key works**
- [Software 2.0](https://karpathy.github.io/2017/11/16/software-2.0/) — karpathy.github.io, 2017. The original "AI changes what programs are" essay; precursor to treating context as a designed artifact.
- LLM OS — X/Twitter post, 2023. Context-as-RAM framing; the mental model behind RES-18's budget arithmetic.
- [karpathy/build-nanogpt](https://github.com/karpathy/build-nanogpt) — GitHub, 2024 (with nanoGPT, 2023). From-scratch internals that ground claims about what physically limits models when sizing context.
- Software 3.0: Software in the Age of AI — YC AI Startup School keynote, 2025 (YouTube). Prompts as programs and partial autonomy — the frame for agent-facing surfaces.

## Study first
1. LLM-OS analogy: what is RAM, what is paging, for a spec graph
2. Software 2.0 → 3.0: where compiled specs sit in the stack
3. Attention/window mechanics as the physical basis of RES-18
