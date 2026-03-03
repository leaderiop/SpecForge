# OPERATIONS & SCALING PLAN

> **SpecForge** -- Compiler for Software Specifications
> Open-core dev tool startup. Remote-first. Rust-native.
> Revenue targets: $25K Y1, $295K Y2, $1.04M Y3

---

## 1. Hiring Plan

### Headcount Growth by Year

| Period | Headcount | Burn Rate (Monthly) | Funding Source |
|--------|-----------|---------------------|----------------|
| Year 1 (Months 1-12) | 4-5 | $45-65K | Bootstrap + angel ($200-400K) |
| Year 2 (Months 13-24) | 10-14 | $130-185K | Seed ($2-3M) |
| Year 3 (Months 25-36) | 25-35 | $350-500K | Series A ($8-12M) |

### Year 1 Role Breakdown (4-5 People)

| # | Role | Type | Rationale |
|---|------|------|-----------|
| 1 | Founder / CEO + Principal Engineer | Full-time | Architecture, compiler core, fundraising |
| 2 | Senior Rust Compiler Engineer | Full-time | Tree-sitter grammar, parser, resolver, validator |
| 3 | Senior Rust Systems Engineer | Full-time | Graph engine, LSP server, CLI, watch mode |
| 4 | Developer Advocate / Community Lead | Full-time | Docs, tutorials, community, launch campaign |
| 5 | Part-time Designer / Technical Writer | Contract | VS Code extension UX, website, brand identity |

### Year 2 Role Breakdown (10-14 People)

| # | Role | Count | Rationale |
|---|------|-------|-----------|
| 1 | Founding team (retained) | 4 | Core continuity |
| 2 | Plugin/Ecosystem Engineer | 2 | Build @specforge/vitest, gh, jira, typescript, rust generators |
| 3 | Full-Stack Engineer (Cloud) | 2 | SpecForge Cloud dashboard, spec registry, CI integration |
| 4 | Head of Growth / Marketing | 1 | Content marketing, conference strategy, developer relations |
| 5 | Solutions Engineer / Pre-Sales | 1 | Enterprise pilot support, customer onboarding |
| 6 | QA / Release Engineer | 1 | CI/CD pipeline, cross-platform builds, release automation |
| 7 | Designer (Product) | 1 | Cloud dashboard UX, VS Code extension, docs site |
| 8 | Office Manager / Operations | 0-1 | Finance, HR basics, vendor management (may be outsourced) |

### Year 3 Role Breakdown (25-35 People)

| Department | Headcount | Key Roles Added |
|------------|-----------|-----------------|
| Engineering | 14-18 | VP Engineering, 2 team leads (compiler, cloud), 4-6 senior engineers, 2-3 mid-level engineers |
| Product | 2-3 | Head of Product, Product Manager (Enterprise), Product Designer |
| Marketing & DevRel | 3-4 | Content Lead, 2 Developer Advocates, Events/Partnerships Manager |
| Sales | 3-5 | VP Sales, 2-3 Account Executives (Enterprise), 1 SDR |
| Operations | 2-3 | Head of Finance, People Lead, Customer Success Manager |
| Support | 1-2 | Technical Support Engineer(s) |

---

## 2. Organizational Structure Evolution

### Phase 1: Flat (Year 1, 4-5 people)

```
                  Founder / CEO
                       |
        +--------------+--------------+
        |              |              |
  Compiler Eng    Systems Eng    DevRel Lead
                                     |
                                 Contract Designer
```

**Characteristics:**
- No formal hierarchy. Everyone reports to the founder.
- Weekly all-hands is the only recurring meeting.
- All engineers commit to the same monorepo.
- Decisions made in Slack threads, ratified in weekly sync.
- Every engineer does code review for every other engineer.

### Phase 2: Functional Teams (Year 2, 10-14 people)

```
                     CEO
                      |
         +------------+------------+
         |            |            |
    Engineering    Product &     Growth &
    (6-8 people)   Design       Marketing
         |        (1-2 people)  (2-3 people)
    +----+----+
    |         |
  Core      Cloud &
  Compiler  Ecosystem
  (3-4)     (3-4)
```

**Characteristics:**
- Two engineering sub-teams: Core Compiler (parser, resolver, validator, graph, LSP) and Cloud/Ecosystem (plugins, generators, cloud dashboard, CI).
- Tech leads for each sub-team, reporting to CEO (who remains technical).
- Product and Design consolidated under one lead.
- Growth/Marketing operates semi-independently with weekly alignment.
- Bi-weekly sprint cadence introduced.

### Phase 3: Matrix Organization (Year 3, 25-35 people)

```
                        CEO
                         |
         +-------+-------+-------+-------+
         |       |       |       |       |
        VP      Head of  VP      Head of  Head of
        Eng     Product  Sales   Marketing Finance/Ops
         |       |       |       |         |
    +----+----+  |    AE Team  DevRel    People
    |    |    |  PM(s)   |     Content    Finance
   Core Cloud  QA/       SDR   Events     CS
   Team Team  Release
```

**Characteristics:**
- VP Engineering hired to manage engineering org, freeing CEO for strategy and fundraising.
- Product Managers embedded with engineering teams (matrix reporting).
- Sales and Customer Success track enterprise pipeline.
- Engineering teams organized around product areas: Core (compiler, LSP, CLI), Cloud (dashboard, registry, collaboration), Ecosystem (plugins, generators, integrations).
- Quarterly OKR cycle introduced. Monthly all-hands. Weekly team standups.

---

## 3. Critical First 5 Hires

### Hire #1: Senior Rust Compiler Engineer

**Why this hire matters:** SpecForge is a compiler. The tree-sitter grammar, parser, incremental resolution, and error recovery system form the technical foundation that everything else depends on. Shipping a fast, correct, developer-friendly compiler is the single highest-risk engineering task.

| Attribute | Detail |
|-----------|--------|
| **Title** | Senior Rust Compiler Engineer |
| **Timing** | Month 1-2 (co-founding or first hire) |
| **Salary Range** | $170,000 - $210,000 (US remote) |
| **Equity** | 2.0 - 4.0% (co-founder level if early enough) |
| **Location** | Remote (US/EU preferred for timezone overlap) |

**Requirements:**
- 5+ years Rust experience, 2+ years in compiler or language tooling.
- Experience with tree-sitter, LALR/PEG parsers, or similar parser generators.
- Strong understanding of incremental compilation, error recovery, and diagnostic reporting.
- Familiarity with graph data structures (petgraph or equivalent).
- Track record of shipping developer tools or language implementations.

**Key Deliverables (First 6 Months):**
- Complete tree-sitter grammar for the `.spec` DSL (16 entity types, all field syntaxes).
- Parser producing typed AST with source span tracking for ariadne diagnostics.
- Resolver with cross-file reference resolution, cycle detection, and fuzzy suggestions.
- All 36 validation codes (E001-E015, W001-W017, I001-I005) implemented and tested.

---

### Hire #2: Senior Rust Systems Engineer

**Why this hire matters:** The compiler is necessary but not sufficient. SpecForge must deliver a real-time IDE experience via LSP, a responsive CLI with watch mode, and a graph engine that supports incremental updates. This hire owns the runtime layer between the compiler and the user.

| Attribute | Detail |
|-----------|--------|
| **Title** | Senior Rust Systems Engineer |
| **Timing** | Month 2-3 |
| **Salary Range** | $160,000 - $200,000 (US remote) |
| **Equity** | 1.5 - 3.0% |
| **Location** | Remote (US/EU/LATAM) |

**Requirements:**
- 5+ years Rust experience, strong async programming skills (tokio ecosystem).
- Experience building LSP servers (tower-lsp, lsp-server, or similar).
- Familiarity with file watching (notify), incremental processing, and CLI frameworks (clap).
- Understanding of graph algorithms and incremental graph mutations (petgraph).
- Experience with cross-platform binary distribution (Linux, macOS, Windows).

**Key Deliverables (First 6 Months):**
- Fully functional `specforge-cli` binary: check, show, init, watch subcommands.
- `specforge-lsp` server with diagnostics, go-to-definition, hover, completion.
- Watch mode with sub-200ms incremental recompilation on file change.
- Graph engine (petgraph-based) with query API for downstream consumers.
- Cross-platform CI/CD pipeline producing release binaries for all targets.

---

### Hire #3: Developer Advocate / Community Lead

**Why this hire matters:** Developer tools live and die by community adoption. SpecForge needs someone who can write compelling content, build a community from zero, manage the open-source project, and execute the launch campaign. The research (RES-19) identifies "5-minute wow" and measurable ROI as critical success factors -- this hire makes both happen.

| Attribute | Detail |
|-----------|--------|
| **Title** | Developer Advocate / Community Lead |
| **Timing** | Month 3-4 |
| **Salary Range** | $130,000 - $170,000 (US remote) |
| **Equity** | 1.0 - 2.0% |
| **Location** | Remote (any timezone, English-fluent) |

**Requirements:**
- 3+ years in developer relations, technical writing, or open-source community management.
- Strong technical background (can read Rust, write code examples, understand compilers).
- Proven content creation skills: blog posts, tutorials, conference talks, videos.
- Experience managing open-source communities (GitHub issues, Discord, contributor onboarding).
- Existing presence in the Rust, AI tooling, or developer tools community is a strong plus.

**Key Deliverables (First 6 Months):**
- Documentation site: getting started guide, entity reference, plugin authoring guide.
- Launch campaign: Hacker News post, dev.to series, social media strategy.
- Token reduction benchmark with Claude Code (the primary marketing asset per RES-19).
- Community infrastructure: Discord server, contributor guide, issue templates, RFC process.
- 3-5 case studies with early adopter teams.

---

### Hire #4: Plugin / Ecosystem Engineer

**Why this hire matters:** The Terraform analogy (RES-19 Section 5.2) establishes that ecosystem is the moat. Plugins (@specforge/vitest, @specforge/gh, @specforge/typescript, @specforge/rust) are what make SpecForge sticky. Without at least 5-10 plugins at launch, the core compiler is a solution without integration points.

| Attribute | Detail |
|-----------|--------|
| **Title** | Plugin / Ecosystem Engineer |
| **Timing** | Month 4-6 |
| **Salary Range** | $150,000 - $190,000 (US remote) |
| **Equity** | 1.0 - 2.0% |
| **Location** | Remote (US/EU/LATAM) |

**Requirements:**
- 4+ years software engineering, polyglot (Rust + TypeScript or Python minimum).
- Experience building plugin systems, SDKs, or integration layers.
- Familiarity with GitHub API, Jira API, and CI/CD systems.
- Understanding of code generation patterns and template engines.
- Experience with npm/cargo publishing and developer toolchain distribution.

**Key Deliverables (First 6 Months):**
- Plugin SDK with stable API for entity registration, edge types, and validation rules.
- @specforge/vitest and @specforge/rust generator plugins (test scaffolding).
- @specforge/gh and @specforge/jira provider plugins (reference validation).
- @specforge/typescript generator plugin (type definitions from spec types).
- Plugin authoring documentation and example plugin template.

---

### Hire #5: Full-Stack Engineer (Cloud)

**Why this hire matters:** Revenue begins with SpecForge Cloud ($15-30/user/month). The cloud dashboard, spec registry, and team collaboration features are what convert free CLI users into paying customers. This hire bridges the gap between open-source adoption and commercial viability.

| Attribute | Detail |
|-----------|--------|
| **Title** | Full-Stack Engineer (Cloud Platform) |
| **Timing** | Month 8-10 (after Seed raise) |
| **Salary Range** | $155,000 - $195,000 (US remote) |
| **Equity** | 0.8 - 1.5% |
| **Location** | Remote (US/EU) |

**Requirements:**
- 4+ years full-stack development with TypeScript/React and a backend language (Rust preferred, Go/Python acceptable).
- Experience building SaaS dashboards with real-time updates.
- Familiarity with authentication (OAuth, SSO), multi-tenancy, and role-based access control.
- Understanding of CI/CD integration (GitHub Actions, GitLab CI).
- Experience with cloud infrastructure (AWS/GCP) and containerized deployments.

**Key Deliverables (First 6 Months):**
- Spec dashboard: visual entity graph, validation status, coverage metrics.
- Spec registry: publish/share spec packages (analogous to Buf Schema Registry).
- Team collaboration: shared workspaces, commenting, review workflows.
- CI integration: GitHub Action / GitLab CI template for `specforge check` in pull requests.
- Authentication and billing infrastructure (Stripe integration).

---

## 4. Remote-First Operating Model

### Async-First Communication Philosophy

SpecForge operates on the principle that **written communication is the default, meetings are the exception.** Every decision, discussion, and status update should be discoverable asynchronously.

| Principle | Implementation |
|-----------|----------------|
| Write it down | All decisions documented in Linear issues, Notion pages, or GitHub PRs. No "hallway decisions." |
| Default to async | Use Slack threads and document comments before scheduling a call. |
| Overlap windows | 4-hour daily overlap window (14:00-18:00 UTC) for synchronous collaboration. |
| Meeting-free mornings | No meetings before 14:00 UTC for any timezone. Protects deep work time. |
| Record everything | All synchronous meetings recorded and summarized in Notion within 24 hours. |

### Communication Tools

| Tool | Purpose | Guidelines |
|------|---------|------------|
| **Slack** | Real-time discussion, quick questions, social | Channels: #general, #engineering, #product, #random, #alerts. Threads mandatory. No DMs for work decisions. |
| **Linear** | Issue tracking, sprint planning, roadmap | Source of truth for all engineering work. Every PR links to a Linear issue. |
| **Notion** | Long-form documentation, RFCs, meeting notes | Knowledge base. RFCs require 48-hour async review before decision meetings. |
| **GitHub** | Code, PRs, CI/CD, releases | All code discussion happens in PR comments, not Slack. |
| **Loom** | Async video updates, demo walkthroughs | Weekly engineering update videos (5 min max). Replaces status meetings. |
| **Tuple / Screen.so** | Pair programming, debugging sessions | For real-time collaboration when async is insufficient. |
| **Slack Huddles / Zoom** | Scheduled synchronous meetings only | Calendar invites required. No ad-hoc calls without prior async context. |

### Meeting Cadence

| Meeting | Frequency | Duration | Attendees | Purpose |
|---------|-----------|----------|-----------|---------|
| All-Hands | Weekly (Y1), Bi-weekly (Y2+) | 30 min | Everyone | Company updates, wins, blockers |
| Engineering Standup | Async daily (Slack bot) | 5 min written | Engineering | Yesterday / today / blockers |
| Sprint Planning | Bi-weekly | 60 min | Engineering + Product | Prioritize next sprint |
| Sprint Retro | Bi-weekly | 45 min | Engineering | Process improvement |
| 1:1s | Weekly | 30 min | Manager + report | Career growth, feedback, blockers |
| Architecture Review | Monthly | 60 min | Senior engineers | Technical debt, architecture decisions |
| Community Sync | Weekly | 30 min | DevRel + Engineering | Open-source priorities, contributor support |

### Timezone Strategy

| Phase | Team Distribution | Overlap Window |
|-------|-------------------|----------------|
| Year 1 (4-5 people) | US + EU concentration | 14:00-18:00 UTC (6 hours effective overlap) |
| Year 2 (10-14 people) | US + EU + LATAM | 14:00-18:00 UTC primary, 16:00-20:00 UTC secondary |
| Year 3 (25-35 people) | Global (US, EU, LATAM, APAC) | Two overlap windows: 14:00-18:00 UTC (Americas+EU) and 06:00-10:00 UTC (EU+APAC) |

**Timezone Rules:**
- No hire requires working outside 08:00-20:00 in their local timezone.
- Pair programming sessions scheduled within overlap windows only.
- On-call rotation distributed across timezones for 24-hour coverage (Year 2+).
- Quarterly in-person offsites (3-4 days) for team bonding and strategic planning. Budget: $3,000-5,000 per person per offsite.

---

## 5. Engineering Culture & Practices

### Development Workflow

**Branching Strategy:** Trunk-based development with short-lived feature branches.

| Practice | Standard |
|----------|----------|
| Branch naming | `<type>/<linear-id>-<short-description>` (e.g., `feat/SF-142-scenario-parser`) |
| Branch lifetime | Maximum 3 days. Longer branches require daily rebases. |
| PR size | Target <400 lines changed. Larger changes split into stacked PRs. |
| PR review | Minimum 1 approval required. 2 approvals for compiler core changes. |
| Review SLA | First review within 4 business hours. Blocking feedback within 8 hours. |
| Merge strategy | Squash merge to main. Clean linear history. |
| Force pushes | Prohibited on main. Allowed on feature branches with caution. |

### Code Quality Standards

| Dimension | Requirement | Enforcement |
|-----------|-------------|-------------|
| Formatting | `cargo fmt` (rustfmt, default settings) | CI gate -- PR cannot merge if formatting fails |
| Linting | `cargo clippy -- -D warnings` | CI gate -- all warnings are errors |
| Type safety | No `unsafe` without RFC and audit trail | `#![forbid(unsafe_code)]` in all crates except tree-sitter bindings |
| Testing | All public APIs have unit tests | CI gate -- coverage must not decrease |
| Snapshot tests | `insta` for parser output, diagnostics, graph queries | `cargo insta review` required before merge |
| Documentation | All public items have doc comments | `#![warn(missing_docs)]` in all crates |
| Dependencies | New dependencies require team discussion | `cargo-deny` for license and vulnerability auditing |

### Testing Requirements

| Test Type | Scope | Tool | When |
|-----------|-------|------|------|
| Unit tests | Per-function, per-module | `cargo test` | Every PR |
| Snapshot tests | Parser output, diagnostics, graph serialization | `insta` | Every PR |
| Integration tests | Multi-crate workflows (parse -> resolve -> validate -> emit) | `cargo test --workspace` | Every PR |
| End-to-end tests | CLI invocation with real `.spec` files | Custom harness in `tests/` | Every PR |
| Fuzz tests | Parser robustness against malformed input | `cargo-fuzz` | Nightly CI |
| Performance benchmarks | Compilation time, memory usage, LSP latency | `criterion` | Weekly CI, reported in Notion |
| Cross-platform tests | Linux (x86_64, aarch64), macOS (aarch64), Windows (x86_64) | GitHub Actions matrix | Every PR |

**Coverage target:** 80% line coverage for compiler crates (parser, resolver, validator). No hard gate, but monitored weekly.

### Release Cadence

| Release Type | Frequency | Contents | Process |
|--------------|-----------|----------|---------|
| **Nightly** | Daily (automated) | Latest main, pre-release tag | Automated CI build, published to GitHub releases as pre-release |
| **Weekly** | Every Monday | Bug fixes, minor improvements | Changelog generated from merged PRs, semantic versioning |
| **Minor** | Every 4-6 weeks | New features, new entity support, new plugins | Full release notes, blog post for significant features |
| **Major** | 2-3 per year | Breaking changes, format version bumps | Migration guide, 4-week deprecation notice, blog post |

### CI/CD Pipeline

```
PR Created
    |
    v
[Formatting Check] --> cargo fmt --check
    |
    v
[Lint] --> cargo clippy -- -D warnings
    |
    v
[Build] --> cargo build --workspace (debug)
    |
    v
[Unit + Integration Tests] --> cargo test --workspace
    |
    v
[Snapshot Tests] --> cargo insta test
    |
    v
[Cross-Platform Matrix] --> Linux x86_64, Linux aarch64, macOS aarch64, Windows x86_64
    |
    v
[Dependency Audit] --> cargo deny check
    |
    v
[PR Approved + Merged to main]
    |
    v
[Release Build] --> cargo build --release (LTO + strip)
    |
    v
[Binary Packaging] --> tar.gz (Linux/macOS), .zip (Windows), .deb, .rpm
    |
    v
[Publish] --> GitHub Releases, Homebrew tap, npm wrapper, crates.io
```

**Infrastructure:** GitHub Actions (primary CI), self-hosted runners for aarch64 Linux. Build cache via `sccache` + GitHub Actions cache. Target: <10 minutes for full CI pipeline.

---

## 6. Legal & IP Considerations

### Open-Source Licensing

| Component | License | Rationale |
|-----------|---------|-----------|
| Core CLI (`specforge-cli`) | Apache 2.0 | Maximally permissive for adoption. Patent grant protects users. |
| Core library crates | Apache 2.0 | Enables embedding in commercial tools without friction. |
| Tree-sitter grammar | MIT | Standard for tree-sitter grammars. Enables editor integration. |
| Official plugins | Apache 2.0 | Consistent with core. Encourages ecosystem. |
| SpecForge Cloud | Proprietary | Revenue-generating. Source-available (BSL or similar) considered for transparency. |
| Enterprise features | Proprietary | SSO, audit trails, compliance reports, advanced RBAC. |

### Contributor License Agreement (CLA)

**Type:** Developer Certificate of Origin (DCO) for Year 1. Upgrade to full CLA (Apache-style) before any commercial offering.

| Phase | Mechanism | Rationale |
|-------|-----------|-----------|
| Year 1 | DCO (sign-off in commit message) | Low friction for early contributors. Proven at Linux, Rust, Kubernetes. |
| Year 2+ | CLA (CLA Assistant bot on GitHub) | Required before launching commercial products. Ensures the company can sublicense contributions. |

**CLA terms:** Contributors grant a perpetual, irrevocable, non-exclusive license to their contributions. Contributors retain copyright. The CLA explicitly permits the company to release contributions under both open-source and commercial licenses.

### Patent Strategy

| Action | Timeline | Detail |
|--------|----------|--------|
| Provisional patent application | Month 12-18 | Cover the specification compilation pipeline: DSL-to-graph compilation, incremental entity resolution, AI agent context optimization via compiled specs. |
| Patent search | Before provisional filing | Ensure no prior art conflicts. Focus on the novel combination of specification compiler + AI agent context delivery. |
| Full patent application | Month 18-24 | Convert provisional to full utility patent if defensible. |
| Defensive patent pledge | At filing | Commit to not asserting patents offensively against open-source projects. Only use defensively. |

**Budget:** $15,000-25,000 for provisional + search. $30,000-50,000 for full utility patent.

### Trademark Protection

| Mark | Type | Filing |
|------|------|--------|
| "SpecForge" (wordmark) | Federal trademark (US) | Month 1-3. Class 9 (software), Class 42 (SaaS). |
| SpecForge logo | Federal trademark (US) | After logo finalization. |
| ".spec" file extension | Not trademarkable | Rely on community standard / convention. |
| "@specforge/" plugin namespace | npm/crates.io namespace | Reserve on all package registries immediately. |

**Budget:** $2,000-4,000 for initial wordmark filing. $5,000-8,000 including attorney fees.

### Open-Core Boundary

The boundary between open-source and commercial is drawn at **individual developer vs. team/enterprise** functionality:

| Capability | Open-Source (Apache 2.0) | SpecForge Cloud (Paid) | Enterprise (Paid) |
|-----------|--------------------------|------------------------|-------------------|
| Compiler (parse, resolve, validate) | Yes | Yes | Yes |
| CLI (check, show, init, watch, gen) | Yes | Yes | Yes |
| LSP server | Yes | Yes | Yes |
| All core + plugin entities | Yes | Yes | Yes |
| All generators and providers | Yes | Yes | Yes |
| Plugin authoring SDK | Yes | Yes | Yes |
| Spec dashboard (visual graph) | No | Yes | Yes |
| Spec registry (publish/share) | No | Yes | Yes |
| Team collaboration (comments, reviews) | No | Yes | Yes |
| CI/CD integration (hosted) | No | Yes | Yes |
| SSO / SAML | No | No | Yes |
| Audit trails / compliance reports | No | No | Yes |
| On-premise deployment | No | No | Yes |
| Custom SLA / priority support | No | No | Yes |
| Advanced RBAC | No | No | Yes |

**Principle:** A solo developer or small open-source team should never need to pay. Payment begins when teams need collaboration, visibility, or compliance.

---

## 7. Operational Infrastructure

### Core Tools Stack

| Category | Tool | Cost (Year 1) | Purpose |
|----------|------|---------------|---------|
| **Source Control** | GitHub (Team plan) | $4/user/mo ($240/yr for 5) | Monorepo, PRs, Actions CI, Releases |
| **Issue Tracking** | Linear (Standard) | $8/user/mo ($480/yr for 5) | Sprint planning, roadmap, issue tracking |
| **Documentation** | Notion (Plus) | $10/user/mo ($600/yr for 5) | RFCs, meeting notes, knowledge base |
| **Communication** | Slack (Pro) | $7.25/user/mo ($435/yr for 5) | Real-time discussion, alerts, integrations |
| **Secrets Management** | 1Password (Business) | $7.99/user/mo ($480/yr for 5) | Credentials, API keys, shared secrets |
| **Email / Calendar** | Google Workspace (Starter) | $7/user/mo ($420/yr for 5) | Company email, calendar, Google Meet |
| **Design** | Figma (Professional) | $15/user/mo ($180/yr for 1) | UI design, brand assets |
| **Video Recording** | Loom (Business) | $12.50/user/mo ($750/yr for 5) | Async updates, demos, tutorials |
| **Error Monitoring** | Sentry (Team) | $26/mo ($312/yr) | CLI crash reporting (opt-in telemetry) |
| **Analytics** | PostHog (Open Source) | Free (self-hosted) | Usage telemetry (opt-in), funnel analysis |

**Year 1 total SaaS cost: ~$4,000-5,000/year** (excluding cloud infrastructure).

### Cloud Infrastructure (Year 2+, for SpecForge Cloud)

| Service | Provider | Estimated Cost | Purpose |
|---------|----------|---------------|---------|
| Compute | AWS (ECS Fargate) or Fly.io | $200-800/mo | API servers, background workers |
| Database | AWS RDS (PostgreSQL) or Neon | $50-200/mo | User data, spec metadata, team workspaces |
| Object Storage | AWS S3 | $20-50/mo | Compiled spec artifacts, registry packages |
| CDN | Cloudflare | Free-$20/mo | Docs site, dashboard static assets |
| DNS | Cloudflare | Free | DNS management, DDoS protection |
| Monitoring | Grafana Cloud (Free tier) | Free | Infrastructure metrics, dashboards |
| CI/CD Runners | GitHub Actions (included) + self-hosted | $50-100/mo | Cross-platform builds, release automation |

**Year 2 infrastructure cost: $3,000-10,000/year** (scaling with user base).

### Security Practices

| Practice | Implementation |
|----------|----------------|
| Dependency auditing | `cargo-deny` in CI, Dependabot alerts enabled |
| Secret scanning | GitHub secret scanning enabled, 1Password for all credentials |
| Binary signing | Release binaries signed with GPG/sigstore |
| SBOM generation | Generated with each release for supply-chain transparency |
| Access control | Principle of least privilege. GitHub branch protection on main. |
| Incident response | Documented runbook in Notion. PagerDuty (Year 2+) for on-call. |

---

## 8. Risk Management & Business Continuity

### Key Person Risk

| Person | Bus Factor Risk | Mitigation |
|--------|----------------|------------|
| Founder / CEO | **Critical** -- sole architect, fundraiser, decision-maker | Document all architecture decisions (ADRs in Notion). Ensure at least 2 people understand every subsystem by Month 6. Hire VP Engineering by Year 3 to distribute technical leadership. |
| Compiler Engineer | **High** -- tree-sitter grammar and parser are specialized knowledge | Pair programming sessions recorded. Grammar documented in `docs/grammar.md`. Snapshot tests capture expected behavior for every construct. |
| Systems Engineer | **High** -- LSP and graph engine are complex subsystems | Architecture docs for LSP and graph modules. Integration tests serve as executable specifications. |
| DevRel Lead | **Medium** -- community relationships are personal | Community processes documented. Multiple team members active in Discord. Content calendar in Notion. |

### Bus Factor Improvement Plan

| Metric | Year 1 Target | Year 2 Target | Year 3 Target |
|--------|---------------|---------------|---------------|
| Minimum people who understand each subsystem | 2 | 3 | 4+ |
| Architecture Decision Records (ADRs) documented | All major decisions | All decisions | All decisions + alternatives considered |
| Onboarding time for new engineer | <2 weeks | <1 week | <3 days |
| Code ownership coverage (CODEOWNERS) | 100% of crates | 100% of crates + cloud | 100% of all code |

### Data Backup & Recovery

| Data Type | Backup Strategy | RPO | RTO |
|-----------|----------------|-----|-----|
| Source code (GitHub) | Git distributed + GitHub redundancy | 0 (distributed) | <1 hour (clone from any developer) |
| Issue tracking (Linear) | Linear's built-in backup + weekly JSON export | 1 week | <4 hours |
| Documentation (Notion) | Notion's built-in backup + monthly HTML export | 1 month | <8 hours |
| User data (SpecForge Cloud, Year 2+) | Automated daily database backups to S3, 30-day retention | 24 hours | <2 hours |
| Spec registry artifacts (Year 2+) | S3 with cross-region replication | <1 hour | <1 hour |
| Secrets (1Password) | 1Password's built-in vault redundancy | Real-time | <30 minutes |

### Incident Response Framework

**Severity Levels (for SpecForge Cloud, Year 2+):**

| Severity | Definition | Response Time | Resolution Target |
|----------|-----------|---------------|-------------------|
| SEV-1 (Critical) | Service completely down, data loss risk | <15 minutes | <2 hours |
| SEV-2 (High) | Major feature broken, significant user impact | <1 hour | <8 hours |
| SEV-3 (Medium) | Minor feature broken, workaround available | <4 hours | <48 hours |
| SEV-4 (Low) | Cosmetic issue, no functional impact | <24 hours | Next sprint |

**For open-source CLI (Year 1+):**

| Issue Type | Response Time | Resolution Target |
|-----------|---------------|-------------------|
| Security vulnerability (CVE) | <24 hours acknowledgment | Patch release within 72 hours |
| Data corruption bug | <24 hours acknowledgment | Patch release within 1 week |
| Crash / panic bug | <48 hours acknowledgment | Fix in next weekly release |
| Feature request | <1 week acknowledgment | Triaged in next sprint planning |

### Business Continuity Scenarios

| Scenario | Impact | Mitigation |
|----------|--------|------------|
| Founder incapacitated | Critical -- leadership vacuum | Board advisor with CEO succession authority (post-Seed). Key decisions documented. Co-founder or VP Eng as interim. |
| Key engineer departure | High -- knowledge loss | Pair programming culture, comprehensive tests, ADRs. 4-year vesting with 1-year cliff retains talent. |
| GitHub outage | Medium -- development blocked | All developers have full repo clones. Can switch to self-hosted GitLab within 48 hours. |
| Funding gap | High -- runway exhaustion | Maintain 6+ months runway buffer. Revenue from Cloud offsets burn by Year 2. Cost-cutting plan documented (reduce to 60% headcount if needed). |
| Open-source fork | Low-Medium -- community split | CLA ensures commercial rights. Community goodwill is the real defense. Maintain responsiveness and transparency. |

---

## 9. Key Operating Metrics

### Engineering Velocity

| Metric | Year 1 Target | Year 2 Target | Year 3 Target | Measurement |
|--------|---------------|---------------|---------------|-------------|
| Sprint velocity (story points/sprint) | 40-60 | 80-120 | 150-250 | Linear (bi-weekly sprints) |
| PRs merged per week | 8-12 | 15-25 | 30-50 | GitHub metrics |
| Average PR review time | <8 hours | <6 hours | <4 hours | GitHub PR analytics |
| Average PR cycle time (open to merge) | <24 hours | <18 hours | <12 hours | Linear cycle time report |
| CI pipeline duration | <8 minutes | <10 minutes | <12 minutes | GitHub Actions |
| Deployment frequency | Weekly releases | Weekly releases | Weekly releases (Cloud: daily) | Release count |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Open bug count (P0/P1) | <5 at any time | Linear bug label |
| Mean time to resolve P0 bugs | <48 hours | Linear cycle time |
| Mean time to resolve P1 bugs | <1 week | Linear cycle time |
| Test suite pass rate | >99% on main | CI dashboard |
| Compiler crash rate (panics in production) | <0.01% of invocations | Sentry (opt-in telemetry) |
| Snapshot test coverage | 100% of parser constructs, 100% of diagnostics | `cargo insta` report |

### Community & Adoption Metrics

| Metric | Month 6 | Month 12 | Month 24 | Month 36 |
|--------|---------|----------|----------|----------|
| GitHub stars | 500-1,000 | 2,000-3,000 | 5,000-10,000 | 15,000-25,000 |
| npm weekly downloads | 100-300 | 500-1,000 | 2,000-5,000 | 10,000-20,000 |
| Monthly active CLI users (telemetry) | 50-100 | 200-500 | 1,000-3,000 | 5,000-10,000 |
| Discord community members | 50-100 | 200-500 | 1,000-2,000 | 3,000-5,000 |
| External contributors (lifetime) | 5-10 | 20-50 | 50-100 | 100-200 |
| Published plugins (community) | 0 | 2-5 | 10-20 | 30-50 |
| GitHub issues opened (monthly) | 20-50 | 50-100 | 100-200 | 200-400 |

### Customer & Revenue Metrics (Year 2+)

| Metric | Year 2 Target | Year 3 Target | Measurement |
|--------|---------------|---------------|-------------|
| SpecForge Cloud registered users | 500-1,000 | 2,000-5,000 | Product analytics (PostHog) |
| Paying customers (Cloud) | 50-100 | 200-500 | Stripe dashboard |
| Enterprise pilots | 3-5 | 10-20 | CRM (HubSpot) |
| Monthly recurring revenue (MRR) | $5K-15K | $30K-80K | Stripe |
| Customer churn (monthly) | <5% | <3% | Stripe analytics |
| Net Promoter Score (NPS) | >40 | >50 | Quarterly survey |
| Customer support response time | <4 hours (business hours) | <2 hours (business hours) | Intercom / Linear |
| Time to first value (new user) | <10 minutes | <5 minutes | Product analytics |

---

## 10. Compensation Philosophy

### Guiding Principles

1. **Competitive but not top-of-market on cash.** We cannot match FAANG base salaries. We offer 80-85% of FAANG cash compensation, offset by meaningful equity in a high-growth company.
2. **Equity is the equalizer.** Early employees receive significant equity grants (1-4%) that, at a successful outcome ($100M+ valuation), represent 10-40x the salary gap vs. FAANG.
3. **Pay fairly across geographies.** Single global pay band adjusted by cost-of-labor zone (not cost-of-living). Three zones: Tier 1 (US major metro, London, Zurich), Tier 2 (US non-major, Western EU, Canada, Australia), Tier 3 (Eastern EU, LATAM, SEA).
4. **Transparency.** All salary bands are published internally. Compensation philosophy is shared during recruiting.
5. **Annual review.** Compensation reviewed annually with market data (levels.fyi, Glassdoor, Rust community salary surveys). Adjustments for performance and market movement.

### Salary Benchmarks (Year 1-2, US Tier 1 Rates)

| Role | Level | Base Salary | FAANG Equivalent | Delta |
|------|-------|-------------|------------------|-------|
| Senior Rust Compiler Engineer | L5/Senior | $170,000 - $210,000 | $220,000 - $280,000 | -20 to -25% |
| Senior Rust Systems Engineer | L5/Senior | $160,000 - $200,000 | $210,000 - $260,000 | -20 to -23% |
| Plugin / Ecosystem Engineer | L4-L5/Mid-Senior | $150,000 - $190,000 | $180,000 - $240,000 | -17 to -21% |
| Full-Stack Engineer (Cloud) | L4-L5/Mid-Senior | $155,000 - $195,000 | $190,000 - $250,000 | -18 to -22% |
| Developer Advocate | L4-L5/Mid-Senior | $130,000 - $170,000 | $160,000 - $220,000 | -19 to -23% |
| Head of Growth / Marketing | Director | $150,000 - $190,000 | $200,000 - $260,000 | -25 to -27% |
| VP Engineering (Year 3) | VP | $200,000 - $260,000 | $300,000 - $400,000 | -33 to -35% |
| VP Sales (Year 3) | VP | $140,000 - $180,000 + commission | $180,000 - $250,000 + commission | -22 to -28% base |

### Geographic Adjustment Zones

| Zone | Adjustment | Example Locations |
|------|------------|-------------------|
| Tier 1 | 100% (base rate) | San Francisco, New York, Seattle, London, Zurich |
| Tier 2 | 85-90% | Austin, Denver, Portland, Berlin, Amsterdam, Toronto, Sydney |
| Tier 3 | 70-80% | Lisbon, Warsaw, Bucharest, Mexico City, Buenos Aires, Manila, Nairobi |

### Equity Structure

**Employee Stock Option Pool (ESOP): 10-15% of fully diluted shares.**

| Metric | Detail |
|--------|--------|
| ESOP size (post-Seed) | 12% of fully diluted shares |
| Vesting schedule | 4-year vesting, 1-year cliff |
| Exercise window | 10 years post-grant (extended post-termination: 90 days standard, negotiable to 1 year for employees with 2+ years tenure) |
| Option type | ISOs for US employees (tax-advantaged), NSOs for international |
| Refresh grants | Annual refresh grants for high performers (0.1-0.5% per year) |
| Acceleration | Single-trigger 25% acceleration on change of control. Double-trigger 100% acceleration (change of control + termination). |

### Equity Grant Ranges by Hire Order

| Hire # | Role Level | Equity Grant | Rationale |
|--------|-----------|-------------|-----------|
| 1-2 | Co-founder / first engineer | 2.0 - 5.0% | Founding risk. Building from zero. |
| 3-5 | Early senior hires | 1.0 - 3.0% | Pre-product, pre-revenue risk. |
| 6-10 | Post-Seed senior hires | 0.5 - 1.5% | Reduced risk, but still early stage. |
| 11-20 | Post-Seed mid-level hires | 0.2 - 0.8% | Growing team, product-market fit in progress. |
| 21-35 | Post-Series A hires | 0.1 - 0.5% | Scaled team, de-risked business. |

### Equity Upside Illustration

The compensation gap vs. FAANG is justified by equity upside. Here is the math for a Senior Rust Engineer (Hire #2):

| Scenario | SpecForge Valuation | Equity Value (2.0%) | 4-Year Total Comp (Salary + Equity) | FAANG 4-Year Total Comp |
|----------|--------------------|--------------------|--------------------------------------|------------------------|
| Bear case | $30M | $600K | $1.32M | $1.40M |
| Base case | $100M | $2.0M | $2.72M | $1.40M |
| Bull case | $300M | $6.0M | $6.72M | $1.40M |
| Home run | $1B | $20.0M | $20.72M | $1.40M |

*FAANG 4-year total comp assumes $250K/year all-in (base + RSU + bonus). SpecForge salary assumes $180K/year base.*

**The pitch:** "You take a $70K/year salary cut for a 3-30x upside on equity. If SpecForge becomes the Terraform of specifications, your equity is worth $2-20M. If we fail, you still earned a competitive salary and shipped a Rust compiler."

### Benefits (Year 1, Scaling with Funding)

| Benefit | Year 1 | Year 2 (Post-Seed) | Year 3 (Post-Series A) |
|---------|--------|---------------------|------------------------|
| Health insurance (US) | $500/mo stipend (ICHRA) | Full coverage (gold plan) | Full coverage + dental + vision |
| Health insurance (International) | Local market rate stipend | Local market rate stipend | Premium plan or equivalent stipend |
| Equipment budget | $3,000 one-time | $3,500 one-time + $1,000/yr refresh | $4,000 one-time + $1,500/yr refresh |
| Home office stipend | $500 one-time | $1,000 one-time | $1,500 one-time |
| Learning & development | $500/year | $1,500/year | $2,500/year |
| Conference travel | 1 conference/year | 2 conferences/year | 2-3 conferences/year |
| PTO | Flexible (minimum 3 weeks enforced) | Flexible (minimum 4 weeks enforced) | Flexible (minimum 4 weeks enforced) |
| Parental leave | 8 weeks (all parents) | 12 weeks (all parents) | 16 weeks (all parents) |
| 401(k) / retirement | None | Offered (no match) | Offered (3% match) |
| Internet stipend | $75/mo | $100/mo | $100/mo |
| Coworking stipend | $200/mo (optional) | $300/mo (optional) | $300/mo (optional) |

---

## Appendix: Year 1 Operating Budget Summary

| Category | Monthly | Annual |
|----------|---------|--------|
| Salaries (4 FTE + 1 contractor) | $38,000 - $52,000 | $456,000 - $624,000 |
| Benefits & stipends | $4,000 - $6,000 | $48,000 - $72,000 |
| SaaS tools | $350 - $450 | $4,000 - $5,400 |
| Legal (trademark, incorporation, CLA) | $500 - $1,500 | $6,000 - $18,000 |
| Cloud infrastructure | $100 - $300 | $1,200 - $3,600 |
| Travel (offsites, conferences) | $500 - $1,500 | $6,000 - $18,000 |
| Marketing (domain, design, content) | $200 - $500 | $2,400 - $6,000 |
| Miscellaneous / contingency | $500 - $1,000 | $6,000 - $12,000 |
| **Total** | **$44,150 - $63,250** | **$529,600 - $759,000** |

**Runway requirement (Year 1):** $550K - $760K. Funded via founder capital ($100-200K) + angel round ($200-400K) + early revenue ($25K target).

**Seed raise trigger:** Product-market fit signals (1,000+ stars, 200+ active users, 3+ enterprise inquiries). Target close by Month 10-14.
