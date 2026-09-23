# 072 — Jerry Liu

**Cluster:** C9 — MCP & AI-agent context engineering
**Roster role:** LlamaIndex co-founder/CEO; retrieval and context budgets
**SpecForge anchors:** retrieval budgets (RES-18), query --depth multi-resolution (crates/specforge-mcp tools/query.rs)

## Why this engineer
Liu built LlamaIndex around the insight that retrieval — what you fetch, at what granularity, within what budget — dominates LLM application quality. SpecForge's `query <id> --depth N` is the symbolic cousin of his hierarchical retrieval: zoom the spec graph in and out to fit a token budget instead of embedding-similarity search over files. He is the reference for budget allocation, index granularity, and the honest boundary cases where structured indexes beat vector search — exactly SpecForge's claim.

## References for SpecForge
**Key works**
- [run-llama/llama_index](https://github.com/run-llama/llama_index) — GitHub, 2022. Index/retriever/node abstractions — the stochastic counterpart of typed graph queries.
- [developers.llamaindex.ai](https://developers.llamaindex.ai) — official docs. Node parsers and hierarchical retrievers: design vocabulary for depth-bounded graph queries.
- Building Production-Ready RAG Applications — talks, 2023-2024 (e.g. Berkeley RDI). The advanced-RAG playbook: chunking, ranking, budget allocation.
- [LlamaIndex](https://www.llamaindex.ai) — company site, 2023-present. Evidence that retrieval/context layers become products (LlamaCloud, LlamaParse).

## Study first
1. Hierarchical retrieval vs `--depth N` graph traversal
2. Token-budget allocation across retrieved context — RES-18 crossover
3. When symbolic indexes outperform embeddings: the spec-graph case
