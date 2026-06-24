# SpecForge — Complete Pitch Support Document

> Generated 2026-04-22. Four expert agents contributed: Problem Analyst, Solution Architect, GTM Strategist, Pitch Coach.

---

# PART 1: THE PROBLEM

## 1.1 Problem Statement

Every AI coding agent in production today — Cursor, Copilot, Claude Code, Devin, Windsurf — operates under the same fundamental handicap: it understands programming languages perfectly but understands your project not at all. It knows how to write Go, Java, Python, TypeScript. It does not know your domain model, your event taxonomy, your aggregate boundaries, your invariants, your architectural constraints, or how any of those things connect. The result is a tool that is syntactically fluent and semantically deaf. It generates code that compiles, passes linting, follows general patterns — and is wrong. Not wrong in a way that a test catches immediately. Wrong in a way that takes a senior engineer three days to untangle, because the AI confidently produced thirty files that were "locally correct, globally incoherent."

This is not a model quality problem. It is a missing infrastructure problem. AI agents consume tokens, not semantics. There is no structured, validated, machine-readable layer between human domain knowledge and AI execution — and until that layer exists, agents will continue to guess at what you meant instead of knowing it.

## 1.2 Who Feels This Pain

### Persona A: The AI-Native Developer

**Profile:** Full-stack or backend engineer, 3-8 years experience. Uses Cursor, Claude Code, or Copilot daily. Personal LLM spend: $40-150/month.

**Pain intensity: 8/10.** Spends 3-4 hours per week fixing AI output and 2-3 hours per week assembling context — curating CLAUDE.md files, pasting relevant code snippets into prompts, writing ad-hoc rules files. Maintains a `.cursorrules` file that has grown to 800-2,000 lines over months, approximately one-third outdated. The AI routinely ignores or misinterprets sections.

**Characteristic quote:** "The AI understood Go syntax and general microservice patterns, but had zero understanding of OUR domain model, OUR event taxonomy, OUR aggregate boundaries."

**Typical incident:** Lost three full working days on a single feature where the AI generated 30 files that compiled, passed linting, and were architecturally wrong. The code was "locally correct, globally incoherent."

### Persona B: The Engineering Lead

**Profile:** Tech lead or staff engineer at a 5-25 person engineering team. Responsible for AI tool adoption. Company stage: Series A through Series C.

**Pain intensity: 9/10.** Bears the compounding cost of inconsistent AI output across the entire team. AI quality varies wildly by seniority: seniors get 60-70% first-attempt accuracy, mid-level 30-40%, juniors 15-20%. Context provision is "an entirely manual, individual skill" — no way to standardize, enforce, or measure it.

**Typical incident:** AI-generated settlement matching logic got merged with the wrong domain model — 1 week of engineering time lost to discovery, revert, and rebuild.

### Persona C: The Legacy Codebase Developer

**Profile:** Mid-to-senior engineer maintaining a large codebase (200K-1M+ LOC). Industry: often regulated (healthcare, fintech, insurance).

**Pain intensity: 9/10.** Spends 60-90 minutes every working day on AI correction and context-setting. Maintains a massive context document (847 lines, one-third outdated). AI failures are not merely annoying — they are dangerous. In a healthcare codebase, an AI generated a claims state transition that would have been a HIPAA audit finding.

**Characteristic quote:** "The AI can write Java fine. It just writes the WRONG Java because it doesn't understand my domain."

### Persona D: The Product Manager

**Profile:** Product manager at a company using AI coding tools. Writes PRDs, feature specs, acceptance criteria.

**Pain intensity: 7/10.** The PRD-to-engineer-to-AI handoff is a black hole. The PM writes a detailed specification. The engineer reads it, forms a mental model, then prompts an AI agent. The AI never sees the original specification. Ever. The PM's carefully crafted domain language and business rules are laundered through a human intermediary.

**Characteristic quote:** "The spec should BE the deliverable, not a side artifact."

## 1.3 The Pain Quantified

### Per-Developer Time Loss

| Activity | Hours/Week |
|----------|-----------|
| Fixing AI output that is semantically wrong | 3-4 hrs |
| Assembling/curating context for AI agents | 2-3 hrs |
| Reviewing AI output for domain correctness | 2-3 hrs |
| Maintaining context documents | 1-2 hrs |
| **Total per developer per week** | **8-12 hrs** |

### Per-Team Cost (7-person engineering team)

| Metric | Calculation | Annual Cost |
|--------|------------|-------------|
| Developer hours wasted | 7 devs × 8 hrs/week × 50 weeks | 2,800 hours/year |
| Direct labor cost | 2,800 hrs × $100/hr | **$280,000/year** |
| AI token waste | $500-2,000/month in exploration tokens | **$6,000-$24,000/year** |
| Major incident cost (1-2/year) | 1-2 weeks lost per incident | **$28,000-$56,000/year** |
| **Total annual cost per 7-person team** | | **$314,000-$360,000/year** |

### Extrapolation to Company Sizes

| Company Size | Engineering Headcount | Annual Cost |
|-------------|----------------------|-------------|
| Startup (seed/A) | 5-10 engineers | $200K-$500K |
| Growth-stage (B/C) | 25-50 engineers | $1M-$2.5M |
| Mid-market | 100-200 engineers | $4M-$10M |
| Enterprise | 500+ engineers | $20M-$50M |

### Token-Level Economics

A typical AI coding task consumes 115K-410K tokens without structured context. With structured specification context, the same task requires 18K-57K tokens — a **75-86% reduction**.

| Scale | Annual Token Spend (Without) | Annual Token Spend (With) | Savings |
|-------|------------------------------|--------------------------|---------|
| Solo developer (20 tasks/week) | $2,400-$10,000 | $360-$1,250 | $2,000-$8,750 |
| 10-person team | $24,000-$100,000 | $3,600-$12,500 | $20,000-$87,500 |
| 100-person org | $80,000-$200,000 | $10,000-$25,000 | $70,000-$175,000 |

## 1.4 Why Existing Solutions Fail

### .cursorrules / CLAUDE.md

No validation — rules can contradict each other. No graph structure — no entity model, no relationship tracking. Glob-scoped, not semantic. At 10+ files, agents ignore sections. Platform-specific. Manually maintained. Nothing detects drift.

### RAG (Retrieval-Augmented Generation)

Retrieval is approximate, not precise. No validation — retrieves stale content alongside current. No graph structure — returns flat text chunks. Chunk boundary problems lose context. Hallucination amplification when partially relevant chunks are treated as ground truth. No traceability.

### Expanding Context Windows

"Lost in the Middle" (Liu et al., 2023) — LLMs perform worst on information buried in the middle of long contexts. More tokens = more cost ($2.50 per task at 500K tokens on Opus). A codebase is not a specification — reading all code tells the AI what IS, not what SHOULD BE. Signal-to-noise ratio drops as codebases grow.

### Enterprise RM Tools (DOORS, Jama)

Not developer-native (GUI-first, web-based). No AI integration. Cost prohibitive ($200+/user/month). Wrong granularity (system-level, not code-level).

## 1.5 The Root Cause

1. **AI tools consume tokens, not semantics.** The model knows "settlement" is a word. It does not know that in YOUR system, settlements require multi-leg reconciliation with partial matching.

2. **Context is assembled per-prompt, not per-project.** Each developer, for each task, manually decides what context to provide. There is no project-level, validated, queryable specification.

3. **Domain knowledge is implicit, not explicit.** The most critical knowledge — domain model, architectural constraints, business invariants — exists primarily in senior engineers' heads. No model improvement fixes this.

## 1.6 The Cost of Inaction

**AI adoption is accelerating into more complex tasks.** An autocomplete suggestion with wrong domain assumptions costs 30 seconds to fix. An autonomous agent implementing a 30-file feature with wrong domain assumptions costs 3 days.

**Software entropy acceleration.** AI agents generate code faster, but without domain understanding, they generate the *wrong* code faster. Tornhill and Borg (2024): low-quality code takes 2x longer to resolve issues and exhibits 15x higher defect density. GitClear (2024): AI-generated code churn projected to double versus pre-AI baselines.

**The cost curve inverts.** Total AI spend increases, but return on spend decreases as the gap between "what the AI can do" and "what the AI knows about your project" widens.

## 1.7 Supporting Evidence

- **Ambig-SWE (ICLR 2026):** Clarified requirements improve AI agent performance up to 74%
- **SWT-Bench (2024):** Explicit test specifications double precision of AI-generated fixes
- **SWE-bench+ (2024):** 32.67% of "successful" patches involved solution leakage — true resolution rates far lower than benchmarks
- **Cursor:** $500M→$1.2B ARR, 1M+ DAUs — market validated that devs pay for AI coding
- **Stack Overflow 2025:** 84% of developers use AI tools, 46% distrust accuracy — 38-point adoption-trust gap
- **Honeycomb (2024):** Chaining five AI calls at 90% individual accuracy yields ~59% cumulative accuracy
- **GitClear (2024):** AI-assisted development associated with increased code churn

---

# PART 2: THE SOLUTION

## 2.1 How It Works: Write Specs → Compile Graph → Feed Agents

### Step 1: Write `.spec` files

```
behavior rate_limited_auth "Rate-Limited Authentication" {
  category   command
  invariants [auth_token_expiry, rate_limit_per_ip]
  types      [AuthRequest, AuthResponse, RateLimitConfig]
  ports      [UserStore, TokenService, RateLimiter]
  produces   [auth_attempted, auth_succeeded, auth_failed]

  requires {
    user_store_available "UserStore port is reachable"
    rate_limiter_configured "RateLimiter has valid config"
  }

  ensures {
    token_on_success "A valid JWT is returned on successful auth"
    rate_limit_enforced "Requests exceeding threshold return 429"
  }

  contract """
    When a client submits credentials, the system MUST check the
    rate limiter before authenticating. If exceeded, return 429
    without touching the UserStore.
  """

  verify unit        "valid credentials return JWT"
  verify unit        "rate limit exceeded returns 429"
  verify integration "failed attempts emit auth_failed event"
}
```

This is not documentation. It is a compiler-checked contract. Every reference is resolved against other entities. If `auth_token_expiry` does not exist as a declared invariant, the compiler produces error E001. **Which keywords exist depends entirely on which extensions you install.** The word "behavior" comes from `@specforge/software`, not the compiler.

### Step 2: Compile the graph

```bash
$ specforge check
```

The compiler parses (Tree-sitter), resolves all references across files, runs extension-declared validation rules, and builds a typed entity graph. Reports all diagnostics in a single pass.

### Step 3: Feed the graph to agents

```bash
$ specforge export --format=context
```

Three export resolutions for different token budgets:

- **`graph`** — Full fidelity. Every field, every source location.
- **`context`** — Agent-optimized. Contracts, status, verify declarations only.
- **`brief`** — Minimal. ID, kind, title, edges. Entire projects in a few thousand tokens.

Scoped subgraph extraction: `specforge export --scope=rate_limited_auth` returns only the relevant neighborhood.

## 2.2 Why a Typed Graph Beats Prose

1. **Validated references eliminate hallucination.** The graph contains no dangling pointers, no stale references, no contradictions.
2. **Typed edges enable traversal.** "What invariants does this behavior depend on?" is a graph query with a deterministic answer.
3. **Multi-resolution queries control token cost.** Typically under 9,000 tokens instead of 40,000-80,000 tokens of unstructured context.
4. **Validation catches drift.** If a behavior references a deleted invariant, the build breaks.
5. **Deterministic output enables reproducibility.** Same specs → same graph → same results for every team member.

## 2.3 Architecture: Zero-Entity Core + Extensions

The compiler has **zero built-in entity types.** It is a pure typed-graph engine. All domain vocabulary comes from extensions:

- **`@specforge/software`** (5 kinds): behavior, invariant, event, type, port
- **`@specforge/product`** (9 kinds): journey, deliverable, milestone, module, term, feature, persona, channel, release
- **`@specforge/governance`** (3 kinds): decision, constraint, failure_mode
- **`@specforge/formal`** (5 kinds): property, axiom, protocol, refinement, process

Extensions are Wasm plugins (Extism runtime) — sandboxed, language-agnostic, AOT-cached, portable. The ManifestV2 declares 18 contribution categories.

**The test:** If a new domain requires a compiler change, the architecture has failed. A maritime logistics company should be able to write `@specforge/shipping` without anyone at SpecForge having anticipated that use case.

## 2.4 Key Differentiators

| vs. | Why SpecForge is different |
|-----|--------------------------|
| `.cursorrules` / `CLAUDE.md` | Compiled, validated, graph-structured, multi-resolution, cross-platform |
| RAG pipelines | Deterministic traversal (not probabilistic retrieval), validated, graph-structured |
| Documentation generators | Works forward (intent→code), not backward (code→docs). Blueprint vs. photograph. |
| Architecture-as-code (C4) | Machine-readable graph for agents, not visual diagrams for humans |
| Schema languages (OpenAPI) | Domain semantics (why it exists, what invariants it satisfies), not API surfaces (what endpoints exist) |

## 2.5 "Structure is a Spectrum"

- **Day 1:** Write one behavior spec. See improved AI output in 10 minutes.
- **Week 1:** Add 5-10 specs for error-prone behaviors. Each reduces correction rounds from 3-4 to 0-1.
- **Month 1:** Cover the core domain model. Cross-cutting constraints enforced automatically.
- **Quarter 1:** Add product-level entities. Graph becomes a planning tool, not just implementation guide.

Value at every stage. No comprehensive coverage required.

## 2.6 Agent Integration

### MCP Server (native agent protocol)

24 tools, 7 resources, 5 prompts. Every extension contributes additional tools and resources. Works with Claude, Cursor, Windsurf, any MCP-compatible agent.

### CLI export (pipe-based)

```bash
specforge export --format=context --scope=rate_limited_auth | pbcopy
```

Works with any agent that reads JSON. Supports `--token-budget=8000` for context window management.

### LSP (IDE integration)

Real-time diagnostics, hover info, go-to-definition, completions, find-all-references. Sub-100ms incremental recompilation.

## 2.7 The Graph Protocol as a Standard

The Graph Protocol is a JSON schema defining the entity graph structure. **The Graph Protocol is the product. The compiler is an implementation detail.**

- Schema is versioned with semantic versioning
- Specification is public — anyone can implement a producer or consumer
- Self-describing: agents discover domain vocabulary from the graph itself

**Success criterion:** Ten compilers producing the same graph is success. The network effect is in the schema: every new agent that reads the Graph Protocol makes every existing `.spec` file more valuable.

## 2.8 Demo Walkthrough (3 Minutes)

**Minute 0:00-0:30 — Init:**
```bash
$ specforge init --name myproject --extensions @specforge/software
```
Time elapsed: 8 seconds.

**Minute 0:30-1:15 — Write one behavior** (6 entities in one file: behavior, invariant, 2 ports, 2 events)

**Minute 1:15-1:45 — Check:**
```bash
$ specforge check
# All references resolved. 6 entities, 5 edges. 0 errors, 0 warnings.
```

**Minute 1:45-2:15 — Export and feed:**
```bash
$ specforge export --format=context --scope=rate_limited_auth
```

**Minute 2:15-3:00 — Before/after:**

**Without SpecForge:** Agent reads 15-20 source files (45,000 tokens). Forgets the rate limiter, uses 24-hour token expiry, doesn't emit `auth_failed` event. Three rounds of corrections.

**With SpecForge:** Agent reads 6 entities, 5 edges (2,400 tokens). Calls RateLimiter, uses 15-minute expiry, emits `auth_failed` with correct payload. Zero corrections.

---

# PART 3: GO-TO-MARKET

## 3.1 Launch Strategy

### Wave 1: Seed the ground (Days -14 to -1)

- DM 30-50 senior developers vocal about AI coding frustrations on Twitter/X
- Post 2-3 "problem-framing" threads that do NOT mention SpecForge
- Ship GitHub repo with polished README, 90-second demo GIF, 3 starter templates

### Wave 2: Public launch (Day 0-3)

- **Hacker News** — Primary launch. Title: "Show HN: SpecForge — a spec DSL that gives AI coding tools structured context about your codebase." Post 8am ET, Tuesday or Wednesday.
- **Twitter/X** — 8-10 tweet thread with screen recording of before/after
- **Reddit** — r/programming (technical), r/ExperiencedDevs (team angle), r/cursor (integration), r/ClaudeAI (MCP)

### Wave 3: Sustained (Days 4-30)

- Product Hunt (1-2 weeks after HN, PM-focused framing)
- Long-form post: "How we went from 30% to 78% first-attempt accuracy"

### The Demo Is Everything

E-commerce codebase with event-sourced order service. States: pending→confirmed→shipped→delivered→cancelled→refunded. Invariant: "refunds only within 30 days of delivery."

**Without SpecForge:** 4 domain rules broken. **With SpecForge:** All 4 respected. Same model, same prompt.

## 3.2 Adoption Funnel

| Stage | Target | Key Tactic |
|-------|--------|-----------|
| **Awareness** | They learn it exists | Launch, SEO, podcasts, meetup talks |
| **Try** | Install + first run | `brew install specforge`, zero config, 60-second value |
| **Adopt** | Write own specs, use daily | LSP feedback, export-to-Cursor pipeline, weekly spec tips |
| **Expand** | Team adopts | Shared specforge.json in repo, junior devs get senior context |
| **Champion** | Advocate externally | `specforge export --format=svg`, GitHub badges, blog templates |

**Critical "aha moment":** Export first spec → paste into AI tool → get noticeably better response. Must happen within **10 minutes** of installation.

## 3.3 Growth Loops

1. **Developer-led bottom-up:** One dev → team sees .spec files in PRs → team adopts → org standardizes
2. **Content loop:** Dev has "bad AI output" frustration → finds SpecForge content → tries it → shares before/after
3. **Extension ecosystem:** Domain expert builds extension → publishes to registry → domain community discovers it
4. **Open standard:** Other tools consume Graph Protocol → more users produce graph files → more tools adopt

## 3.4 Distribution Channels

1. **Package managers:** `brew install specforge`, `cargo install specforge-cli`, `npm install -g specforge`
2. **VS Code / Cursor Marketplace:** Extension with syntax highlighting, diagnostics, completions
3. **MCP Server Registries:** Anthropic's directory, mcp.so, Smithery
4. **GitHub Template Repos:** `specforge/starter-saas`, `specforge/starter-api`, `specforge/starter-fintech`
5. **GitHub Action:** `specforge/check-action` — runs in CI, posts PR comment with entity graph diff

## 3.5 First 90 Days

### Week 1-2: Launch
- Verify all package managers work on clean machines
- Record demo video (90-sec GIF + 3-min narrated)
- HN → Twitter → Reddit → fix every bug in first 48 hours

### Week 3-4: First Community Milestones
- Targets: 500+ GitHub stars, 100+ Discord, 50+ users who've run `specforge export`
- Publish 2 "spec pattern" blog posts
- Seed 3 potential extension authors (healthcare, fintech, gaming)

### Month 2: Traction
- 200+ weekly active CLI users
- 15+ public repos with specforge.json
- 5+ teams (multi-contributor repos)
- 1 community extension published

### Month 3: Raise Signal
- Ready to raise if: 500+ WAU, 20+ teams, 3+ community extensions, 5+ companies interested in paid tier, >5% WoW organic growth for 4+ weeks

## 3.6 Key Metrics

| Metric | Target | Danger Signal |
|--------|--------|--------------|
| Install → First Spec (activation) | >40% | <20% |
| First Spec → First Export (aha) | >25% | <10% |
| First Export → Week 2 active (retention) | >30% | <15% |
| Solo → Team adoption (expansion) | >15% in 60 days | <5% |
| Time from install to first export | <10 minutes | >15 minutes |

---

# PART 4: PITCH NARRATIVE AND STORYTELLING

## 4.1 Opening Hooks (Choose One)

### The Stat Hook (for investors)
> "Last year, developers wrote 4 billion lines of AI-generated code. In codebases with more than 50 entities, between 40 and 70 percent of that generated code is locally correct but globally wrong. It compiles. It passes lint. And it violates your domain model in ways you won't catch until production. The problem isn't the model. The problem is that the model never saw your architecture."

### The Story Hook (for conferences)
> "A developer I interviewed showed me an 847-line markdown file. He pastes it into Claude before every coding session. Every single day, he manually builds a bad version of what should be infrastructure. And here's the thing — he's the BEST CASE. His teammates just prompt and pray."

### The Insight Hook (for technical audiences)
> "Why do AI coding tools produce better React components than domain services? Same model, same user. The answer: React has a rigid, well-documented structure. Your domain model exists in one person's head and a Notion doc the AI has never seen. The next unlock isn't a better model. It's structured context."

## 4.2 Narrative Arc

### ACT 1: "The Illusion" (2 min)
AI coding tools are everywhere and impressive — until entity 50. At scale, AI code is "locally plausible, globally incoherent."
**Emotional beat: RECOGNITION.** Every senior dev has felt this.

### ACT 2: "The Root Cause" (2 min)
Context today is: (1) prose — ambiguous, (2) ephemeral — dies when chat closes, (3) personal — quality depends on individual skill, (4) unvalidated — nobody checks if it's right.
**Emotional beat: SURPRISE.** The reframe from "model problem" to "context problem."

### ACT 3: "The Insight" (1.5 min)
Context should be a compiled artifact. What type systems did for code, we do for context.
**Emotional beat: EXCITEMENT.** "The type system for AI context."

### ACT 4: "The Solution" (3 min)
Write specs. Compile graphs. Feed agents. Three commands. Sixty seconds to value.
**Emotional beat: CONFIDENCE.** Clear, simple, well-architected.

### ACT 5: "The Future" (1.5 min)
The Graph Protocol becomes the standard for structured development context. Like OpenAPI for APIs.
**Emotional beat: URGENCY.** The window is 18 months.

## 4.3 The 10-Slide Deck

| # | Title | Key Message |
|---|-------|------------|
| 1 | SpecForge | The Type System for AI Context |
| 2 | AI Coding Works — Until It Doesn't | Locally correct, globally incoherent at scale |
| 3 | The Context Problem | Prose, ephemeral, personal, unvalidated |
| 4 | Validated Pain | Four personas, one finding: context is the bottleneck |
| 5 | What If Context Were Code? | Type systems for code → type systems for context |
| 6 | Write Specs. Compile Graphs. Feed Agents. | Three commands, sixty seconds to value |
| 7 | Demo / Before-After | Same model. Same prompt. Better context. Better code. |
| 8 | Architecture | Domain-agnostic core. Extension-driven vocabulary. |
| 9 | The Standard Is the Moat | Open schema. Ten compilers = success. |
| 10 | Team + Ask | What we've built, what we need, the window |

## 4.4 One-Liners

| Audience | One-Liner |
|----------|-----------|
| **Developers** | "SpecForge compiles your domain model into a typed graph that AI coding tools actually understand." |
| **CTOs** | "SpecForge gives every AI tool in your org structured, validated context about your architecture — so output is architecturally correct, not just syntactically correct." |
| **Investors** | "SpecForge is building the Graph Protocol — the open standard for structured AI coding context. Like OpenAPI defined how machines talk to APIs, we're defining how AI understands codebases." |
| **Product Hunt** | "Stop pasting architecture docs into ChatGPT. Write .spec files, compile them into typed graphs, and give AI tools real context about your domain. Open source." |
| **Twitter bio** | "Building SpecForge — the type system for AI context." |

## 4.5 Objection Handling

### "Won't bigger context windows solve this?"
Bigger windows make this problem worse. A million tokens of unstructured prose is incredibly confusing. We make context windows useful, not replace them.

### "Why would developers write specs?"
They already do — the 847-line markdown file IS a spec, just in a format no tool can validate. SpecForge asks them to do the same work in a format that's actually useful. ROI visible on the first file.

### "How is this different from documentation?"
Documentation describes. Specs declare. Documentation is ambiguous — the AI interprets it. A spec is a typed graph — the AI traverses it. Documentation can go stale silently. Specs are compiled.

### "What if Cursor/GitHub builds this in?"
We'd welcome it — if they adopt the Graph Protocol. Cursor will build Cursor-specific context. We're building the standard that works everywhere.

### "How do you make money on an open standard?"
Same as Red Hat on Linux, HashiCorp on Terraform, Postman on OpenAPI. Open standard creates the market. Commercial ecosystem layer captures value.

### "What if AI tools get good enough at code understanding?"
Models improve at syntax, not your business rules. GPT-7 will still not know your domain model unless someone tells it. Organizations still need provable requirement-to-test linkage regardless.

### "This seems like it's only for large codebases."
The pain scales with complexity, but value starts at one entity. One spec file with 3 behaviors already improves output.

### "What's the evidence structured context works?"
Ambig-SWE (ICLR 2026): clarified requirements improve performance up to 74%. SWT-Bench: explicit specs double precision. Plus: try it yourself. The difference is not subtle.

## 4.6 Closing Lines

### For investors:
> "The window for defining the open standard for AI coding context is the next 18 months. We intend to be the team that defines it. We'd like you to be part of that."
> *Then stop talking.*

### For conferences:
> "You can try this today. `cargo install specforge`. Write one spec file for your most complex domain entity. Export the graph. I promise you the difference is not subtle."

### For Product Hunt:
> "`specforge init` takes 30 seconds. Try it on your most complex entity first. You'll know within one session whether this changes your workflow."

---

# APPENDIX: THE TERRAFORM ANALOGY

| Property | Terraform | SpecForge |
|----------|-----------|-----------|
| Core language | HCL | .spec DSL |
| Core knowledge | Zero infrastructure knowledge | Zero domain knowledge |
| Domain concepts | Provider plugins (AWS, Azure, GCP) | Wasm extensions (@specforge/*) |
| Output format | State file + Plan | Graph Protocol (JSON) |
| Distribution | Single binary (Go) | Single binary (Rust) |
| Free tier | CLI + state management | CLI + graph export + LSP + MCP |
| Commercial layer | Terraform Cloud | SpecForge Cloud |
| Moat | Provider ecosystem (3,000+) | Extension ecosystem + Graph Protocol |
| IPO valuation | $5.3B | Target: standard-level adoption |
