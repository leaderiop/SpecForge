# COMMUNITY & OPEN SOURCE STRATEGY

## 1. Open Source Licensing Strategy

### License Selection: Apache 2.0

SpecForge core (compiler, CLI, LSP) is licensed under Apache 2.0. This is a deliberate strategic choice informed by the licensing decisions of our closest analogues.

| Project | License | Stars | Enterprise Adoption |
|---------|---------|-------|---------------------|
| Terraform | BSL 1.1 (was MPL 2.0) | 354K | Massive, but license change caused OpenTofu fork |
| Prisma | Apache 2.0 | 40K | Strong, trust-based adoption |
| Buf | Apache 2.0 | 10K | Growing enterprise presence |
| Tree-sitter | MIT | 19K | Universal editor adoption |
| SpecForge | Apache 2.0 | Target 5K Y1 | Building trust from day one |

**Why Apache 2.0 over MIT:**

- **Patent grant.** Apache 2.0 includes an explicit patent grant, protecting contributors and users. MIT does not. For a specification compiler whose Graph Protocol is consumed by AI agents and third-party tools, this matters.
- **Attribution preservation.** Apache 2.0 requires NOTICE files be preserved. This ensures SpecForge attribution survives redistribution by enterprises and AI tool vendors.
- **Contributor clarity.** Apache 2.0 has well-understood contribution terms. Combined with a CLA, it creates an unambiguous IP chain that enterprise legal teams approve faster.
- **Terraform lesson.** HashiCorp's BSL relicense fractured the Terraform community and spawned OpenTofu. We make an irrevocable commitment: the SpecForge compiler will never be relicensed. Apache 2.0 forever for the core.

**What is Apache 2.0:**

| Component | License |
|-----------|---------|
| `specforge-cli` binary | Apache 2.0 |
| `specforge-lsp` binary | Apache 2.0 |
| All parser/compiler crates | Apache 2.0 |
| `@specforge/product` extension | Apache 2.0 |
| `@specforge/governance` extension | Apache 2.0 |
| First-party extensions | Apache 2.0 |
| First-party providers | Apache 2.0 |
| Documentation and examples | CC BY 4.0 |
| SpecForge Cloud (future) | Proprietary |

**The bright line:** Everything that runs on your machine is Apache 2.0. Everything that runs on our servers is proprietary. This is the Grafana model, the GitLab model, and the model that enterprise legal teams understand.

### Contributor License Agreement (CLA)

All contributions require a CLA. We use the Apache ICLA (Individual Contributor License Agreement) as our template, managed through CLA Assistant (GitHub App) for frictionless onboarding.

**CLA terms:**

- Contributors grant a perpetual, irrevocable license to their contributions under Apache 2.0.
- Contributors retain copyright of their work.
- The CLA covers patent rights (standard Apache ICLA clause).
- Corporate contributors sign a CCLA through their employer's legal team.

**CLA workflow:**

1. First-time contributor opens a PR.
2. CLA Assistant bot comments with a link to sign.
3. Contributor signs electronically (GitHub OAuth, one click).
4. Bot marks the PR as CLA-signed. All future PRs are auto-approved.
5. Signature stored in a public `.clabot` file in the repository.

**Why require a CLA:** A clean IP chain is required if we ever need to sublicense (e.g., dual-license a component for a partner integration) or defend against patent claims. Every major Apache Foundation project requires one. The cost is 30 seconds per contributor; the benefit is legal clarity forever.

### Contribution Guidelines

The `CONTRIBUTING.md` file is the front door of the project. It covers:

| Section | Contents |
|---------|----------|
| **Code of Conduct** | Rust-style Code of Conduct (adapted from Rust RFC 1023). Zero tolerance for harassment. Enforcement by core team. |
| **Getting Started** | Clone, install Rust toolchain, `cargo build`, `cargo test`. Under 5 minutes on any machine. |
| **Issue Taxonomy** | Labels: `bug`, `enhancement`, `good-first-issue`, `help-wanted`, `extension`, `provider`, `renderer`, `docs`, `performance`. |
| **PR Process** | Fork - branch - commit - PR - review - merge. All PRs require 1 core team review. CI must pass (clippy, tests, formatting). |
| **Commit Style** | Conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`. Scope optional: `feat(parser):`. |
| **Architecture Decision Records** | Significant changes require an ADR (stored in `spec/decisions/`). Template provided. |
| **Testing Requirements** | All new validation codes require snapshot tests (insta). All parser changes require grammar tests. |
| **Documentation Requirements** | New entities require a doc page in `docs/entities/`. New CLI commands require help text and a guide entry. |

---

## 2. Contributor Model

### Four-Tier Structure

```
                    ┌───────────────┐
                    │   CORE TEAM   │  4-6 people
                    │  (governance) │
                    ├───────────────┤
                    │  MAINTAINERS  │  10-20 people
                    │ (merge rights)│
                    ├───────────────┤
                    │ CONTRIBUTORS  │  100-300 people
                    │  (code + docs)│
                    ├───────────────┤
                    │    USERS      │  5,000+ people
                    │  (issues/Qs)  │
                    └───────────────┘
```

### Tier 1: Users

**Population target:** 5,000+ Y1 / 50,000+ Y2

**Activities:**
- File bug reports and feature requests
- Ask questions on GitHub Discussions and Discord
- Star and share the project
- Write blog posts and tutorials about SpecForge
- Use SpecForge in their projects

**Recognition:**
- Public thanks in release notes for high-quality bug reports
- "Community Highlight" in monthly newsletter for interesting use cases

**Pathway to Contributor:** File a `good-first-issue` PR or contribute documentation.

### Tier 2: Contributors

**Population target:** 50+ Y1 / 300+ Y2

**Requirements:**
- Signed CLA
- 1+ merged PR (code, docs, or tests)
- Follows contribution guidelines

**Privileges:**
- Listed in `CONTRIBUTORS.md`
- `contributor` role on Discord (access to `#contributors` channel)
- Eligible for Pioneer/Ambassador program
- Can request assignment on unassigned issues
- Invited to monthly contributor call (30 min, recorded)

**Expectations:**
- No minimum activity level. One-time contributions are valuable.
- Respond to review comments on their open PRs within 7 days.

### Tier 3: Maintainers

**Population target:** 5+ Y1 / 20+ Y2

**Requirements:**
- 10+ merged PRs with sustained quality over 3+ months
- Deep expertise in at least one subsystem (parser, graph, validator, LSP, Wasm runtime)
- Demonstrated sound judgment in code review
- Nominated by a core team member, approved by majority vote

**Privileges:**
- Merge rights on their subsystem
- `maintainer` role on Discord (access to `#maintainers` private channel)
- Listed on the project website as a maintainer
- Voting rights on RFCs affecting their subsystem
- Access to project infrastructure (CI, release pipeline)
- Annual SpecForge swag pack

**Expectations:**
- Review 3+ PRs per week in their subsystem
- Triage new issues in their area within 48 hours
- Participate in bi-weekly maintainer sync (30 min)
- Mentor at least 1 new contributor per quarter
- 6-month minimum commitment; graceful emeritus process if stepping back

**Emeritus process:** Maintainers who are inactive for 90+ days are moved to "maintainer emeritus" status with honor. They retain Discord access and recognition but lose merge rights. They can return to active status by re-engaging for 30 days.

### Tier 4: Core Team

**Population target:** 2 founding members + 2-4 community-elected by Y2

**Requirements:**
- 6+ months as active maintainer
- Cross-subsystem understanding of the compiler architecture
- Demonstrated leadership in community and technical decisions
- Elected by existing core team (unanimous consent)

**Privileges:**
- All maintainer privileges
- Release authority (can publish new versions)
- Governance voting rights (roadmap, RFC approval, policy changes)
- Admin access to GitHub org, Discord server, and infrastructure
- Invited to annual in-person core team summit (travel covered)

**Expectations:**
- Drive the project roadmap and prioritization
- Final arbitration on disputed technical decisions
- Represent SpecForge at conferences and in press
- Ensure the project remains healthy, inclusive, and sustainable

**Governance model:** Lazy consensus. Any core team member can propose a decision. If no objection within 72 hours, the decision is approved. Contested decisions go to majority vote. The project founder holds a temporary tie-breaking vote for the first 2 years, after which all core team votes are equal.

---

## 3. Community Platforms

### GitHub (Primary)

**Repository structure:**

| Repository | Purpose |
|------------|---------|
| `specforge/specforge` | Main monorepo (compiler, CLI, LSP, official extensions) |
| `specforge/tree-sitter-specforge` | Tree-sitter grammar (separate for editor integration) |
| `specforge/vscode-specforge` | VS Code extension |
| `specforge/specforge.dev` | Documentation site source |
| `specforge/awesome-specforge` | Community-curated list of extensions, providers, renderers, articles |
| `specforge/rfcs` | RFC proposals for language and compiler changes |

**GitHub Discussions categories:**

| Category | Purpose | Moderation |
|----------|---------|------------|
| `Announcements` | Release notes, roadmap updates, events | Core team only (post), all (comment) |
| `Q&A` | Technical questions, troubleshooting | Community-moderated, maintainer escalation |
| `Ideas` | Feature requests, brainstorming | Open discussion, core team labels promising ideas |
| `Show and Tell` | Community projects, extensions, blog posts | Open, highlighted in newsletter |
| `RFC Discussion` | Structured discussion on open RFCs | Open, linked from `specforge/rfcs` |
| `Extensions & Providers` | Extension development help, showcase | Open, extension/provider/renderer authors active |

**Issue labels (standardized):**

| Label | Color | Meaning |
|-------|-------|---------|
| `bug` | `#d73a4a` | Confirmed bug |
| `enhancement` | `#a2eeef` | Feature request |
| `good-first-issue` | `#7057ff` | Suitable for new contributors |
| `help-wanted` | `#008672` | Maintainers want community help |
| `parser` | `#e4e669` | Tree-sitter grammar or CST/AST |
| `graph` | `#e4e669` | Entity graph or edge resolution |
| `validator` | `#e4e669` | Validation codes |
| `lsp` | `#e4e669` | Language server |
| `wasm-runtime` | `#e4e669` | Wasm extension runtime |
| `extension` | `#fbca04` | Extension system |
| `provider` | `#fbca04` | Provider system |
| `renderer` | `#fbca04` | Renderer extension system |
| `docs` | `#0075ca` | Documentation |
| `performance` | `#ff9f1c` | Performance improvement |
| `breaking` | `#b60205` | Breaking change |
| `rfc` | `#d4c5f9` | Requires RFC |

### Discord Server

**Server name:** SpecForge Community

**Channel structure:**

| Category | Channel | Purpose |
|----------|---------|---------|
| **Welcome** | `#welcome` | Rules, CoC, getting started links |
| | `#introductions` | New members introduce themselves |
| | `#announcements` | Release notes, events (read-only, mirrored from GitHub) |
| **General** | `#general` | Open discussion about SpecForge |
| | `#show-and-tell` | Share what you built with SpecForge |
| | `#ai-workflows` | SpecForge + AI agent integration tips and tricks |
| | `#off-topic` | Non-SpecForge technical chat |
| **Help** | `#help-getting-started` | Installation, first spec, basic questions |
| | `#help-compiler` | Parser, validation, error messages |
| | `#help-lsp` | LSP setup, editor integration |
| | `#help-extensions` | Extension/provider/renderer questions |
| **Development** | `#dev-compiler` | Compiler internals discussion |
| | `#dev-lsp` | LSP development |
| | `#dev-extensions` | Extension/provider/renderer SDK development |
| | `#dev-rfcs` | RFC discussion and drafting |
| | `#pr-feed` | Bot-posted PR activity (read-only) |
| | `#ci-status` | Build status notifications (read-only) |
| **Ecosystem** | `#extensions-showcase` | Announce and discuss community extensions |
| | `#renderers` | Renderer extension development and integration |
| | `#providers` | Provider development and integration |
| **Private** | `#contributors` | Verified contributors only (Tier 2+) |
| | `#maintainers` | Maintainers only (Tier 3+) |
| | `#core-team` | Core team only (Tier 4) |

**Roles:**

| Role | Color | Permissions |
|------|-------|-------------|
| `@Core Team` | `#e74c3c` | Admin, all channels |
| `@Maintainer` | `#e67e22` | All channels, pin messages |
| `@Contributor` | `#2ecc71` | `#contributors` access |
| `@Pioneer` | `#9b59b6` | Pioneer program badge, `#pioneers` access |
| `@Ambassador` | `#3498db` | Ambassador badge, `#ambassadors` access |
| `@Community` | `#95a5a6` | Default role, public channels |

**Moderation:**
- AutoMod rules for spam, slurs, and excessive self-promotion.
- 2 community moderators recruited from active contributors by Q2.
- Moderation log reviewed weekly by core team.
- Escalation path: Community mod - Maintainer - Core team.

**Bots:**
- GitHub bot: Posts PR/issue activity to `#pr-feed`.
- CI bot: Posts build status to `#ci-status`.
- Welcome bot: DMs new members with getting-started guide.
- Starboard bot: Pins messages with 5+ reactions to `#show-and-tell`.

### Reddit

**Subreddit:** `r/specforge`

**Strategy:** Low-effort, high-signal. Reddit is a discovery channel, not a support channel.

| Activity | Cadence | Owner |
|----------|---------|-------|
| Release announcements | Every release | Core team |
| Weekly digest | Weekly (automated) | Bot + core team review |
| Cross-posts to `r/rust`, `r/programming`, `r/devtools` | Major releases only | Core team |
| Community AMAs | Quarterly | Core team |

**Subreddit rules:**
1. No low-effort posts. Questions go to GitHub Discussions or Discord.
2. Self-promotion limited to 1 post per user per week.
3. Benchmark claims must include reproducible methodology.

### Twitter/X

**Handle:** `@speclorehq`

**Content strategy:**

| Content Type | Cadence | Format |
|-------------|---------|--------|
| Release announcements | Per release | Thread: what's new, why it matters, upgrade instructions |
| Feature deep-dives | 2x/week | Single tweet + code screenshot or 30s video |
| Community highlights | 1x/week | RT + quote with context |
| Benchmark results | Monthly | Infographic + link to methodology |
| Memes / dev humor | 1-2x/week | On-brand humor about spec drift, AI hallucinations, context management |
| Conference live-tweets | At events | Thread per talk with key takeaways |

**Engagement rules:**
- Reply to every mention within 4 hours during business hours.
- RT community content at minimum 3x per week.
- Never engage in tool-bashing or competitor negativity. Compete on merits.
- Thread length: 3-7 tweets maximum. Respect the timeline.

**Growth targets:**

| Metric | Q2 2026 | Q4 2026 | Q4 2027 |
|--------|---------|---------|---------|
| Followers | 1,000 | 5,000 | 20,000 |
| Avg. impressions/tweet | 2,000 | 10,000 | 50,000 |
| Engagement rate | 3%+ | 4%+ | 4%+ |

### Hacker News

**Strategy:** Organic, high-impact launches only. HN is unpredictable but can drive 10,000+ visitors in a day.

| Event | HN Post |
|-------|---------|
| Initial public launch | "Show HN: SpecForge - A specification compiler for AI-assisted development" |
| v1.0 stable release | "SpecForge v1.0: validated spec graphs that cut AI agent errors by 70%" |
| Major benchmark report | "We compiled 10,000 spec files in 500ms: how SpecForge's parser works" |
| Self-hosting milestone | "How SpecForge compiles its own specifications (self-hosting story)" |

**Rules:** Never astroturf. Never ask for upvotes. Let the work speak.

---

## 4. Documentation Strategy

### Documentation Site Architecture

**URL:** `docs.specforge.dev`

**Engine:** Astro + Starlight (static site, deployed on Cloudflare Pages)

**Six-section structure:**

```
docs.specforge.dev/
├── getting-started/        # Section 1: Onboarding
│   ├── installation        # brew, npx, cargo, GitHub releases
│   ├── quickstart          # First spec in 5 minutes
│   ├── concepts            # Entity model, graph, traceability
│   ├── your-first-project  # Step-by-step: init, write, check, compile
│   └── ai-integration      # Feeding spec output to AI agents
│
├── guides/                 # Section 2: Task-oriented guides
│   ├── writing-specs       # Best practices for spec authoring
│   ├── ci-integration      # GitHub Actions, GitLab CI, etc.
│   ├── test-traceability   # verify/scenario, tests field, reports
│   ├── multi-file-projects # File organization, imports, namespaces
│   ├── lsp-setup           # VS Code, Neovim, JetBrains, Zed
│   ├── migration           # Moving from CLAUDE.md / plain context
│   └── error-cookbook       # Common errors and how to fix them
│
├── reference/              # Section 3: Exhaustive reference
│   ├── cli                 # Every command, flag, and output format
│   ├── spec-language       # Formal grammar, generic entity block syntax
│   ├── validation-codes    # All 36 codes with examples
│   ├── edge-types          # All 20 edges with semantics
│   ├── configuration       # specforge.toml options
│   └── output-formats      # agent-context, json, human-readable
│
├── extensions/             # Section 4: Extension ecosystem
│   ├── overview            # Extension/provider/renderer model
│   ├── product-extension   # @specforge/product (5 entities)
│   ├── governance-extension # @specforge/governance (3 entities)
│   ├── github-provider     # @specforge/gh
│   ├── jira-provider       # @specforge/jira
│   ├── building-extensions # Extension SDK guide
│   ├── building-providers  # Provider SDK guide
│   └── building-renderers  # Renderer extension SDK guide
│
├── examples/               # Section 5: Complete working examples
│   ├── todo-app            # Minimal: 10 entities, core only
│   ├── saas-platform       # Medium: 50 entities, product extension
│   ├── healthcare-system   # Large: 150 entities, all extensions + providers
│   ├── rust-library        # Library project with test traceability
│   └── self-hosting        # SpecForge's own specifications
│
└── contributing/           # Section 6: Contributor resources
    ├── setup               # Dev environment setup
    ├── architecture        # Compiler architecture overview
    ├── testing             # Snapshot testing with insta
    ├── coding-standards    # Rust style, conventions
    ├── rfc-process         # How to propose language changes
    └── release-process     # How releases are cut and published
```

### Documentation Quality Standards

| Standard | Measurement | Target |
|----------|-------------|--------|
| **Every CLI command documented** | Automated check: CLI help text matches docs | 100% coverage |
| **Every validation code has an example** | Automated check: E/W/I codes in reference | 100% coverage |
| **Every entity type has a dedicated page** | Manual review | 16/16 |
| **Every guide is tested end-to-end** | CI runs guide steps quarterly | 100% pass rate |
| **Freshness** | Last-updated date on every page; stale = 90+ days without review | 0 stale pages |
| **Search** | Algolia DocSearch integration | <3s results for any concept |

### Documentation Cadence

| Activity | Cadence | Owner |
|----------|---------|-------|
| New feature docs | Ships with the feature (same PR) | Feature author |
| Guide updates | Monthly review cycle | Docs maintainer |
| Example project maintenance | Quarterly | Community contributors |
| Grammar/language reference updates | Per spec format change | Core team |
| Translation | Y2+ (community-driven, starting with Chinese, Japanese, Spanish) | Community |

### Documentation Principles

1. **Every page answers one question.** No page tries to be a guide and a reference simultaneously.
2. **Code examples are copy-pasteable.** Every snippet runs as-is. No `...` elisions in runnable examples.
3. **Error messages link to docs.** Every `specforge check` diagnostic includes a `--explain E001` flag that opens the relevant docs page.
4. **AI agents can read the docs.** The docs site exposes a `/llms.txt` and `/llms-full.txt` endpoint with machine-readable content for AI agent consumption.
5. **Docs are tested.** Code examples in guides are extracted and compiled in CI. Broken examples are build failures.

---

## 5. Pioneer & Ambassador Program

### Phase 1: Founding Pioneers (Q1-Q2 2026)

**Goal:** Recruit 25 early adopters who will use SpecForge on real projects, provide feedback, and become the nucleus of the community.

**Selection criteria:**
- Active developer with a public project suitable for SpecForge
- Willing to commit 2-4 hours/week for 3 months
- Willing to share feedback publicly (blog post, tweet thread, or conference talk)

**Recruitment channels:**
- Direct outreach to AI-first developers on Twitter/X (50 invitations)
- Rust community leaders who maintain specification-heavy projects
- Early GitHub stargazers who open substantive issues
- Developer tool bloggers and content creators

**Benefits:**

| Benefit | Details |
|---------|---------|
| `@Pioneer` Discord role | Access to `#pioneers` private channel |
| Direct Slack/Discord line to core team | 24-hour response guarantee during pioneer period |
| Name in launch credits | Listed on specforge.dev/pioneers page |
| Pioneer swag pack | T-shirt, stickers, laptop decal (mailed Q2) |
| Early access to new features | Preview builds 2 weeks before public release |
| Co-authoring opportunity | Invited to co-author blog posts with the core team |
| Conference speaking opportunity | Priority submission support for SpecForge-related talks |

**Pioneer obligations:**
- Use SpecForge on a real project (minimum 20 entities)
- File 5+ issues (bugs or feature requests) over 3 months
- Publish 1 piece of content (blog post, tweet thread, or video)
- Participate in bi-weekly pioneer feedback call (30 min)

**Pioneer cohort timeline:**

| Date | Milestone |
|------|-----------|
| Q1 2026 W4 | Pioneer applications open |
| Q1 2026 W6 | Cohort 1 selected (15 pioneers) |
| Q1 2026 W8 | Pioneer onboarding call |
| Q2 2026 W2 | Cohort 2 selected (10 pioneers) |
| Q2 2026 W12 | Pioneer program concludes; graduates invited to Ambassador track |

### Phase 2: Ambassadors (Q3 2026 - Q2 2027)

**Goal:** Build a global network of 30-50 SpecForge advocates who actively promote the tool through content, talks, and mentoring.

**Selection criteria:**
- Pioneer graduates, OR
- Community members with 5+ contributions and public SpecForge content, OR
- Developer advocates at companies using SpecForge in production

**Benefits:**

| Benefit | Details |
|---------|---------|
| `@Ambassador` Discord role | Access to `#ambassadors` private channel |
| SpecForge Ambassador badge | For GitHub profile, blog, and conference talks |
| Conference sponsorship | Up to $500 travel stipend per approved talk (2 per year) |
| Early access to all features | Preview builds 4 weeks before public release |
| Quarterly call with core team | Roadmap preview, feedback session |
| LinkedIn/Twitter amplification | Core team shares all ambassador content |
| Annual summit invitation | Virtual in Y1, in-person in Y2 (travel covered) |

**Ambassador obligations:**
- Publish 1 piece of content per month (blog, video, or talk)
- Mentor 2+ new users per quarter (on Discord or GitHub)
- Represent SpecForge at 1+ local meetup per quarter
- Provide quarterly feedback on roadmap and developer experience

### Phase 3: Champions (Q3 2027+)

**Goal:** Recognize 10-15 top community leaders who drive SpecForge adoption at organizational scale.

**Selection criteria:**
- 12+ months as active Ambassador
- Drove SpecForge adoption at 1+ organization (team of 5+)
- Created a high-impact community resource (popular extension, widely-used example project, or conference workshop)

**Benefits:**

| Benefit | Details |
|---------|---------|
| `@Champion` Discord role | Access to all channels including `#core-team` (observer) |
| Advisory board seat | Quarterly meeting with CEO/CTO on product direction |
| Paid conference appearances | Full travel + speaker fee for approved events |
| Free SpecForge Cloud team plan | When launched (up to 25 seats) |
| Co-design sessions | Input on major features before they are built |
| Annual in-person summit | Travel and accommodation covered |
| Public recognition | Featured on specforge.dev/champions, blog interview |

---

## 6. Conference Strategy

### Three-Tier Conference Model

#### Tier 1: Flagship Conferences

Major industry conferences with 2,000+ attendees. Goal: brand awareness and credibility with senior engineering leaders.

| Conference | Audience | Target Dates | Activity |
|------------|----------|--------------|----------|
| **RustConf** | Rust ecosystem | Q3 annually | Talk proposal + sponsored booth |
| **KubeCon NA** | Cloud-native engineers | Q4 annually | Talk proposal (spec-as-code for cloud) |
| **QCon** | Senior architects | Q2/Q4 | Talk proposal (AI-assisted specification) |
| **Strange Loop** (or successor) | Polyglot developers | Q3 | Talk proposal (compiler architecture) |
| **AI Engineer Summit** | AI-native developers | Q2 annually | Talk + booth (primary persona event) |

**Budget per Tier 1 event:**

| Line Item | Cost |
|-----------|------|
| Conference ticket (1-2 people) | $1,000-2,500 |
| Travel + accommodation | $2,000-4,000 |
| Booth (if sponsored) | $3,000-8,000 |
| Swag for booth | $500-1,000 |
| **Total per event** | **$6,500-$15,500** |

**Y1 target:** Attend 2-3 Tier 1 conferences. Propose talks at all 5. Budget: $25,000-$35,000.

#### Tier 2: Regional and Specialized Conferences

Mid-size conferences (500-2,000 attendees) targeting specific communities or regions.

| Conference | Audience | Activity |
|------------|----------|----------|
| **RustNation UK** | European Rust community | Talk proposal |
| **EuroRust** | European Rust community | Talk proposal |
| **DevOpsDays** (multiple cities) | DevOps engineers | Lightning talks |
| **All Things Open** | Open source enthusiasts | Talk + sponsor |
| **FOSDEM** | European open source | Devroom talk |
| **Dev Tool Conf** (various) | Developer tooling community | Talk + demo |

**Budget per Tier 2 event:**

| Line Item | Cost |
|-----------|------|
| Conference ticket | $200-500 |
| Travel + accommodation | $1,000-2,500 |
| Swag | $200-500 |
| **Total per event** | **$1,400-$3,500** |

**Y1 target:** Attend 4-6 Tier 2 conferences. Budget: $8,000-$18,000.

#### Tier 3: Meetups and User Groups

Local developer meetups (20-200 attendees). Highest ROI for early-stage community building.

| Meetup Type | Target Cities | Activity |
|-------------|---------------|----------|
| Rust meetups | SF, NYC, London, Berlin, Tokyo | 20-min talk + live demo |
| AI/ML meetups | SF, NYC, Seattle, Austin | Lightning talk on AI + spec traceability |
| DevTools meetups | SF, NYC, London | Product demo + Q&A |
| General dev meetups | Any city with an Ambassador | Ambassador-led talks |

**Budget per Tier 3 event:**

| Line Item | Cost |
|-----------|------|
| Pizza/drinks sponsorship | $200-500 |
| Swag (stickers, T-shirts) | $100-200 |
| **Total per event** | **$300-$700** |

**Y1 target:** 10-15 meetup talks (mix of core team and ambassadors). Budget: $4,000-$8,000.

### Conference Content Library

Maintain a shared library of talk materials that anyone in the community can adapt:

| Asset | Format | Use |
|-------|--------|-----|
| "Intro to SpecForge" | 20-min deck + speaker notes | Meetup lightning talk |
| "SpecForge for AI-Native Development" | 40-min deck + demo script | Conference talk |
| "Building SpecForge Extensions" | 30-min workshop materials | Hands-on tutorial |
| "From CLAUDE.md to .spec" | 20-min deck + before/after examples | Migration-focused talk |
| "Compiler Architecture Deep Dive" | 45-min deck + architecture diagrams | Technical conference talk |

All materials stored in `specforge/community-talks` repository under CC BY 4.0 license.

---

## 7. Content Strategy

### Blog

**URL:** `specforge.dev/blog`

**Cadence:** 2 posts per week.

**Content mix:**

| Category | Cadence | Examples |
|----------|---------|---------|
| **Release notes** | Per release (~biweekly) | "SpecForge 0.3: Watch Mode and 40% Faster Compilation" |
| **Technical deep-dives** | 2x/month | "How Tree-sitter Error Recovery Works in SpecForge," "Inside the Entity Graph: petgraph at Scale" |
| **Use case stories** | 1x/month | "How [Company] Reduced AI Hallucinations by 73% with SpecForge" |
| **Tutorial / how-to** | 2x/month | "Setting Up SpecForge CI in GitHub Actions," "Writing Your First Extension" |
| **Community spotlight** | 2x/month | Interviews with contributors, ambassador highlights, extension showcases |
| **Opinion / thought leadership** | 1x/month | "Why Every AI Agent Needs a Specification Compiler," "The End of CLAUDE.md" |
| **Benchmark reports** | Quarterly | Token reduction measurements, compile performance, agent task-completion rates |

**Content calendar (sample month):**

| Week | Tuesday | Thursday |
|------|---------|----------|
| W1 | Release notes: v0.4 | Deep dive: validation code design |
| W2 | Tutorial: multi-file specs | Community spotlight: Pioneer project |
| W3 | Use case: AI startup adoption | Deep dive: provider architecture |
| W4 | Thought leadership: spec-driven dev | Benchmark report: Q2 performance |

### Case Studies

**Cadence:** 1 per month starting Q3 2026 (after pioneer program produces results).

**Format:**

| Section | Content |
|---------|---------|
| Company/project overview | Who, what, team size |
| The problem | Context management pain, AI agent issues |
| Why SpecForge | Decision criteria, evaluation process |
| Implementation | Timeline, migration approach, entity counts |
| Results | Token reduction %, error reduction %, time saved |
| Lessons learned | What worked, what was hard, what they would do differently |
| Testimonial quote | Named individual, with permission |

**Target case study sources:**

| Source | Timeline |
|--------|----------|
| Pioneer program projects | Q2-Q3 2026 |
| Self-hosting (SpecForge itself) | Q3 2026 |
| Early production adopters | Q4 2026 |
| Enterprise pilots | Q1-Q2 2027 |

### Benchmark Reports

**Cadence:** Quarterly.

**Standard benchmarks:**

| Benchmark | Methodology |
|-----------|------------|
| **Token reduction** | Same specification expressed as plain text (CLAUDE.md) vs. SpecForge agent-context output. Measured by token count (cl100k_base). |
| **Compile performance** | Wall-clock time for `specforge check` on standardized corpora: 10 entities, 50 entities, 200 entities, 1000 entities. Measured on GitHub Actions runner (Ubuntu, 4 vCPU). |
| **Agent task completion** | Controlled experiment: AI agent (Claude, GPT-4) given coding task with plain-text context vs. SpecForge context. Measured by test pass rate on first attempt. |
| **Error detection** | Number of specification errors caught by SpecForge that would have caused agent-generated code defects. Measured against historical bug reports. |

**Publication:** Blog post + full methodology on GitHub + social media infographic.

### Video Content

**Platform:** YouTube (`@specforge`)

**Content types:**

| Type | Length | Cadence | Examples |
|------|--------|---------|---------|
| Quick tips | 2-5 min | 2x/week | "Tip: Use --scope for 10x smaller context" |
| Tutorials | 10-20 min | 1x/week | "Building a REST API spec from scratch" |
| Deep dives | 30-45 min | 2x/month | "Tree-sitter Grammar Design for SpecForge" |
| Livestreams | 60-90 min | 1x/month | "Building a SpecForge Extension from Scratch (live)" |
| Conference talks | 20-45 min | As recorded | All SpecForge conference talks republished |

**Production quality:** Screen recordings with voiceover for tutorials. Face-to-camera for deep dives. Minimal editing; ship fast.

### Newsletter

**Name:** "The Spec Sheet"

**Platform:** Buttondown (developer-friendly, markdown-native)

**Cadence:** Biweekly.

**Format:**

| Section | Content |
|---------|---------|
| Lead story | Biggest news or feature of the past 2 weeks |
| Release digest | Changelog summary with links |
| Community highlights | 2-3 notable community contributions, blog posts, or projects |
| Extension of the week | Featured extension/provider/renderer |
| Tip of the week | One practical SpecForge tip with example |
| Upcoming events | Meetups, conferences, AMAs |
| Numbers | GitHub stars, downloads, contributor count (transparency) |

**Growth targets:**

| Metric | Q2 2026 | Q4 2026 | Q4 2027 |
|--------|---------|---------|---------|
| Subscribers | 500 | 2,000 | 10,000 |
| Open rate | 45%+ | 40%+ | 35%+ |
| Click rate | 15%+ | 12%+ | 10%+ |

---

## 8. Extension Ecosystem Seeding

### Year 1 First-Party Extension Target: 15-20 Extensions

Building the first wave of extensions ourselves establishes quality standards, proves the extension APIs, and gives users enough ecosystem to be productive.

**Planned first-party extensions:**

| Extension | Type | Priority | Target Quarter |
|-----------|------|----------|---------------|
| `@specforge/product` | Extension | P0 | Q1 (ships with core) |
| `@specforge/governance` | Extension | P0 | Q1 (ships with core) |
| `@specforge/rust` | Test collector | P0 | Q3 |
| `@specforge/rtm-renderer` | Renderer | P1 | Q3 |
| `@specforge/mermaid-renderer` | Renderer | P1 | Q4 |
| `@specforge/compliance` | Domain extension | P2 | Q4 |
| `@specforge/gh` | Provider | P0 | Q3 |
| `@specforge/jira` | Provider | P1 | Q4 |
| `@specforge/figma` | Provider | P2 | Q5 |
| `@specforge/gitlab` | Provider | P2 | Q5 |
| `@specforge/linear` | Provider | P2 | Q5 |
| `@specforge/vitest` | Test Runner | P0 | Q3 |
| `@specforge/pytest` | Test Runner | P1 | Q4 |
| `@specforge/go` | Test Runner | P2 | Q4 |
| `@specforge/playwright` | Test Runner | P2 | Q5 |
| `specforge-test` (crate) | Rust Proc Macro | P0 | Q3 |
| `specforge-test-macros` (crate) | Rust Proc Macro | P0 | Q3 |

### Extension SDK Quality Investment

The extension SDK is what determines whether the community builds extensions. We treat it as a product.

**SDK deliverables:**

| Deliverable | Target | Purpose |
|-------------|--------|---------|
| Extension SDK crate (`specforge-extension-sdk`) | Q3 | Rust API for building extensions |
| Extension Development Kit (EDK) | Q3 | Libraries + templates for building Wasm extensions |
| Provider SDK crate (`specforge-provider-sdk`) | Q3 | Rust API for building providers |
| `specforge extension init` CLI command | Q3 | Generate extension boilerplate |
| `specforge extension init` CLI command | Q3 | Scaffold new extension boilerplate |
| `specforge scaffold provider` CLI command | Q3 | Generate provider boilerplate |
| Extension testing harness | Q3 | Test extensions against mock spec graphs |
| Extension documentation template | Q3 | Standardized README and docs structure |
| Extension example repository | Q3 | 3 example extensions (one of each type) with detailed comments |

**SDK quality bar:**
- A new extension can be built in under 2 hours by a developer who has never seen the codebase.
- A new provider can be built in under 1 hour.
- A new extension (adding 1 entity type) can be built in under 4 hours.
- Every SDK function has a doc comment, an example, and a test.

### Bounty Program

Starting Q4 2026, offer bounties for community-built extensions.

**Bounty tiers:**

| Tier | Payout | Requirements |
|------|--------|-------------|
| **Gold** | $2,000 | High-demand extension (from wishlist), production-quality, tested, documented |
| **Silver** | $1,000 | Useful extension, good quality, basic documentation |
| **Bronze** | $500 | Working extension, minimal documentation |
| **Bug bounty** | $100-500 | Security or correctness bugs in existing extensions |

**Bounty process:**
1. Core team publishes a wishlist of desired extensions on GitHub Discussions.
2. Community members claim a bounty by commenting on the issue.
3. Claimed bounties have a 60-day deadline (extendable once by 30 days).
4. Submissions reviewed by 2 maintainers within 14 days.
5. Payouts via GitHub Sponsors or Open Collective.

**Y1 bounty budget:** $15,000 (enough for 10-15 bounties).
**Y2 bounty budget:** $30,000.

**High-priority bounty targets (Y1):**

| Extension | Type | Bounty |
|-----------|------|--------|
| `@community/api-design` | Domain extension | Gold ($2,000) |
| `@community/atomic-design` | Domain extension | Gold ($2,000) |
| `@community/openapi-renderer` | Renderer | Silver ($1,000) |
| `@community/asyncapi-renderer` | Renderer | Silver ($1,000) |
| `@community/linear` | Provider | Silver ($1,000) |
| `@community/notion` | Provider | Silver ($1,000) |
| `@community/azure-devops` | Provider | Gold ($2,000) |
| `@community/security` | Extension | Gold ($2,000) |

---

## 9. Community Health Metrics

### Core Health Indicators

| Metric | Definition | Target Y1 | Target Y2 | Measurement |
|--------|-----------|-----------|-----------|-------------|
| **GitHub Stars** | Total stars on `specforge/specforge` | 5,000 | 20,000 | GitHub API, weekly |
| **Weekly Active CLI Users** | Unique users running any `specforge` command (opt-in telemetry) | 5,000 | 50,000 | Anonymous telemetry, weekly |
| **Production Projects** | Projects with 20+ entities and CI integration | 100 | 1,000 | Self-reported + telemetry proxy |
| **Monthly Active Contributors** | Unique authors with merged PRs in the past 30 days | 15 | 50 | GitHub API, monthly |
| **Total Contributors (cumulative)** | All-time unique PR authors | 50 | 300 | `CONTRIBUTORS.md` |

### Responsiveness Metrics

| Metric | Definition | Target | Measurement |
|--------|-----------|--------|-------------|
| **Issue first response time** | Median time from issue creation to first human response | <24 hours | GitHub API |
| **PR first review time** | Median time from PR creation to first review comment | <48 hours | GitHub API |
| **PR merge time** | Median time from PR creation to merge (for accepted PRs) | <7 days | GitHub API |
| **Discord question response time** | Median time from question posted to first helpful response | <4 hours (business hours) | Discord analytics |
| **Bug fix time** | Median time from `bug` label to fix merged (P0/P1 bugs) | <72 hours (P0), <14 days (P1) | GitHub API |

### Engagement Metrics

| Metric | Definition | Target Y1 | Target Y2 |
|--------|-----------|-----------|-----------|
| **Discord DAU** | Daily active users in Discord server | 200 | 1,000 |
| **GitHub Discussions activity** | New discussions + replies per week | 30/week | 100/week |
| **Newsletter subscribers** | Active subscribers to "The Spec Sheet" | 2,000 | 10,000 |
| **Newsletter open rate** | % of subscribers who open each issue | 40%+ | 35%+ |
| **Blog unique visitors** | Monthly unique visitors to specforge.dev/blog | 10,000 | 50,000 |
| **YouTube subscribers** | Subscribers on YouTube channel | 500 | 3,000 |
| **Twitter/X followers** | Followers on @specforgehq | 5,000 | 20,000 |

### Ecosystem Metrics

| Metric | Definition | Target Y1 | Target Y2 |
|--------|-----------|-----------|-----------|
| **First-party extensions** | Published extensions by the core team | 15-20 | 25-30 |
| **Community extensions** | Published extensions by the community | 5-10 | 30-50 |
| **Extension downloads** | Total downloads across all extensions (monthly) | 2,000 | 20,000 |
| **Extension SDK satisfaction** | NPS among extension authors | 40+ | 50+ |
| **Extension bounties completed** | Bounties claimed and paid | 10 | 25 |

### Satisfaction Metrics

| Metric | Definition | Target Y1 | Target Y2 | Measurement |
|--------|-----------|-----------|-----------|-------------|
| **Developer NPS** | Net Promoter Score from opt-in CLI survey | 50+ | 60+ | Quarterly survey |
| **Contributor NPS** | NPS among active contributors | 60+ | 70+ | Quarterly survey |
| **CSAT (documentation)** | "Was this page helpful?" widget on docs | 80%+ positive | 85%+ positive | Continuous |
| **CSAT (support)** | Resolution satisfaction on Discord/GitHub | 85%+ positive | 90%+ positive | Monthly sample |

### Reporting Cadence

| Report | Audience | Cadence | Format |
|--------|----------|---------|--------|
| **Community Dashboard** | Public | Real-time | specforge.dev/community/stats (live dashboard) |
| **Monthly Community Report** | Newsletter subscribers | Monthly | Blog post + newsletter section |
| **Quarterly Health Review** | Core team + maintainers | Quarterly | Private document with action items |
| **Annual Community Report** | Everyone | Annually | Comprehensive blog post with year-in-review stats |

---

## 10. Budget

### Year 1 Budget: $150,000

| Program | Allocation | % of Total | Details |
|---------|-----------|------------|---------|
| **Conferences (Tier 1)** | $30,000 | 20% | 2-3 flagship events (tickets, travel, booths) |
| **Conferences (Tier 2)** | $15,000 | 10% | 4-6 regional events (travel, swag) |
| **Conferences (Tier 3 / Meetups)** | $6,000 | 4% | 10-15 meetup sponsorships |
| **Pioneer Program** | $8,000 | 5% | Swag packs (25 pioneers x $150), shipping, early-access infrastructure |
| **Ambassador Program** | $12,000 | 8% | Travel stipends ($500 x 12 approved talks), ambassador swag, virtual summit |
| **Content Production** | $20,000 | 13% | Blog illustrations, video production, infographic design |
| **Swag & Merch** | $8,000 | 5% | T-shirts, stickers, laptop decals (general inventory) |
| **Extension Bounties** | $15,000 | 10% | 10-15 bounties for community-built extensions |
| **Documentation Site** | $5,000 | 3% | Hosting, Algolia search, design |
| **Community Tools** | $4,000 | 3% | Discord Nitro (bots/emojis), GitHub org, Buttondown newsletter, analytics |
| **Contractor / Part-time DevRel** | $25,000 | 17% | Part-time developer advocate for content, community management |
| **Contingency** | $2,000 | 1% | Unplanned opportunities (viral moment, unscheduled event) |
| **Total** | **$150,000** | **100%** | |

### Year 2 Budget: $300,000

| Program | Allocation | % of Total | Details |
|---------|-----------|------------|---------|
| **Conferences (Tier 1)** | $50,000 | 17% | 4-5 flagship events (larger booth presence, more team members) |
| **Conferences (Tier 2)** | $25,000 | 8% | 8-10 regional events |
| **Conferences (Tier 3 / Meetups)** | $10,000 | 3% | 20+ meetups (ambassador-led with budget) |
| **Pioneer Program (Cohort 3-4)** | $10,000 | 3% | Ongoing recruitment of new pioneer cohorts |
| **Ambassador Program** | $25,000 | 8% | 30-50 ambassadors, higher travel stipends, in-person summit |
| **Champion Program** | $15,000 | 5% | Advisory board, paid appearances, Cloud credits |
| **Content Production** | $35,000 | 12% | Professional video, podcast launch, increased blog output |
| **Swag & Merch** | $12,000 | 4% | Expanded inventory for growing conference/event presence |
| **Extension Bounties** | $30,000 | 10% | 20-25 bounties, higher-value targets |
| **Documentation Site** | $8,000 | 3% | Translation infrastructure, improved search, versioned docs |
| **Community Tools** | $6,000 | 2% | Scaled community infrastructure |
| **Full-time DevRel Hire** | $65,000 | 22% | Partial-year salary for dedicated community/devrel lead |
| **Sponsorships** | $5,000 | 2% | Sponsor Rust community events, open source projects we depend on |
| **Contingency** | $4,000 | 1% | |
| **Total** | **$300,000** | **100%** | |

### Budget Principles

1. **No community spend without measurement.** Every program has metrics tied to the health indicators in Section 9. If a program is not moving metrics after 2 quarters, we reallocate.
2. **Favor many small bets over few large ones.** 15 meetup talks are more valuable than 1 platinum conference sponsorship. Spread the budget to maximize touchpoints.
3. **Pay community members fairly.** Bounties, travel stipends, and speaker fees are not gifts; they are compensation for work that benefits the project. Budget for them explicitly.
4. **Invest in the SDK.** The extension SDK is not a community expense; it is a product expense. But it is listed here because its quality directly determines community extension output. If we had to cut one line item, the SDK would be the last to go.
5. **Reinvest cloud revenue into community.** Starting Y2, 10% of SpecForge Cloud revenue is allocated back to the community budget (bounties, sponsorships, events).

### ROI Framework

| Investment | Leading Indicator (3 months) | Lagging Indicator (12 months) |
|-----------|------------------------------|-------------------------------|
| Conference talks | Twitter followers, GitHub star spikes post-event | Inbound enterprise leads citing conference talk |
| Pioneer program | Bug reports filed, feedback quality | Case studies published, production adoption |
| Ambassador program | Monthly content output, meetup frequency | Geographic reach of community, non-English adoption |
| Extension bounties | Bounties claimed, extension PRs opened | Community extension count, ecosystem completeness |
| Content production | Blog traffic, newsletter growth | SEO rankings for "specification compiler," inbound organic traffic |
| Documentation | Docs page views, CSAT scores | Reduced support volume, faster time-to-first-value |
