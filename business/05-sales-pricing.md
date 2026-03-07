# SALES & PRICING STRATEGY

## 1. Pricing Philosophy

SpecForge pricing is built on five foundational principles that reflect both our open-core business model and our position as AI-native developer infrastructure.

**Value-Based, Not Cost-Based.** Pricing anchors to the economic value delivered --- token reduction savings, developer time recovered, and specification coverage achieved --- not to our infrastructure costs. A single developer using SpecForge Pro saves $50-200/month in LLM API costs alone (75-86% token reduction on an estimated $100-300/month AI spend). Our $29/month price captures 15-30% of that value, leaving the majority with the customer. This ensures a perpetual positive-ROI framing in every sales conversation.

**Developer-Friendly from Day One.** The free tier is a real product, not a trial. Developers can compile specs, validate graphs, export JSON, and integrate with AI agents indefinitely at no cost. We do not gate core compiler functionality behind a paywall. The free tier builds trust and community; the paid tiers unlock scale, collaboration, and operational features that matter when a single developer becomes a team.

**Transparent and Predictable.** Per-seat pricing with no hidden metering, no token surcharges, no surprise overages. Every tier has a public price on the website. Enterprise custom pricing is the only exception, and even there we publish starting-at ranges. Developers hate opaque pricing --- we will never be the tool where a team lead has to "talk to sales" before knowing whether they can afford it.

**PLG-First, Sales-Assisted Second.** 70%+ of revenue through Year 3 comes from self-serve adoption. The sales team accelerates enterprise deals; it does not create demand. Every paid conversion starts with a developer who already uses SpecForge free, experiences the value, and either self-upgrades or tells their manager to buy it. Sales exists to remove procurement friction, not to cold-call.

**Expansion-Native.** Pricing is designed so that the natural trajectory of a successful customer moves rightward across tiers: free individual developer becomes Pro user, brings in teammates (Team), organization standardizes (Enterprise). Net revenue retention above 130% is architecturally baked into the tier structure.

---

## 2. Pricing Tiers

### Tier Overview

| | **Free** | **Pro** | **Team** | **Enterprise** |
|---|---|---|---|---|
| **Price** | $0 | $29/dev/mo | $79/dev/mo | Custom (from $149/dev/mo) |
| **Annual Price** | $0 | $24/dev/mo (billed annually) | $67/dev/mo (billed annually) | Negotiated |
| **Target Buyer** | Individual developer | Power user / freelancer | Engineering lead (5-50 devs) | VP Eng / CTO (50+ devs) |
| **Billing** | N/A | Self-serve credit card | Self-serve or invoice | Annual contract, invoice |
| **Minimum Seats** | N/A | 1 | 5 | 25 |

### Detailed Feature Matrix

| Feature | Free | Pro | Team | Enterprise |
|---|:---:|:---:|:---:|:---:|
| **Core Compiler** | | | | |
| `specforge check` (parse + validate) | Yes | Yes | Yes | Yes |
| `specforge compile` (graph export) | Yes | Yes | Yes | Yes |
| `specforge init` (project scaffolding) | Yes | Yes | Yes | Yes |
| Tree-sitter parser with error recovery | Yes | Yes | Yes | Yes |
| All core validation codes + extension codes | Yes | Yes | Yes | Yes |
| Agent-context output format | Yes | Yes | Yes | Yes |
| Community extensions + providers | Yes | Yes | Yes | Yes |
| **Entity Limits** | | | | |
| Maximum entities per project | 500 | Unlimited | Unlimited | Unlimited |
| Maximum .spec files per project | 20 | Unlimited | Unlimited | Unlimited |
| Maximum projects | 3 | Unlimited | Unlimited | Unlimited |
| **Advanced Compiler** | | | | |
| `specforge trace` (test traceability) | Basic (5 entities) | Full | Full | Full |
| `specforge report` (coverage dashboard) | -- | CLI | CLI + JSON | CLI + JSON + HTML |
| AI-assisted spec authoring (LSP) | -- | Yes | Yes | Yes |
| `specforge query` (graph queries) | -- | Yes | Yes | Yes |
| Diff-aware context (`--diff`) | -- | Yes | Yes | Yes |
| Custom validation rules | -- | -- | Yes | Yes |
| **LSP & Editor** | | | | |
| LSP diagnostics + go-to-definition | Yes | Yes | Yes | Yes |
| Autocomplete for entity references | Yes | Yes | Yes | Yes |
| VS Code extension | Yes | Yes | Yes | Yes |
| **Cloud Dashboard** | | | | |
| Web-based graph visualizer | -- | Single project | Multi-project | Multi-project + cross-repo |
| Spec coverage trends over time | -- | 30-day history | Unlimited history | Unlimited + custom retention |
| Change impact analysis | -- | Yes | Yes | Yes |
| PR comment integration (GitHub/GitLab) | -- | -- | Yes | Yes |
| **Collaboration** | | | | |
| Shared workspaces | -- | -- | Yes | Yes |
| Role-based access control (RBAC) | -- | -- | Yes | Yes (custom roles) |
| Team activity feed | -- | -- | Yes | Yes |
| Cross-repository traceability | -- | -- | -- | Yes |
| **CI/CD & DevOps** | | | | |
| `specforge check` in CI (exit codes) | Yes | Yes | Yes | Yes |
| CI/CD pipeline templates | -- | GitHub Actions | GH Actions + GitLab CI + CircleCI | All + custom |
| Spec coverage gates (block merge) | -- | -- | Yes | Yes |
| Webhook notifications | -- | -- | Yes | Yes |
| **Security & Compliance** | | | | |
| SSO (SAML/OIDC) | -- | -- | Yes | Yes |
| Audit logging | -- | -- | -- | Yes (90-day) |
| Compliance report generation (RTM) | -- | -- | -- | Yes |
| Data residency options | -- | -- | -- | Yes (US, EU, custom) |
| **Deployment** | | | | |
| Cloud-hosted (multi-tenant) | -- | Yes | Yes | Yes |
| Self-hosted / on-premise | -- | -- | -- | Yes |
| Air-gapped deployment | -- | -- | -- | Yes |
| **Support** | | | | |
| Community forum + GitHub issues | Yes | Yes | Yes | Yes |
| Email support | -- | 48h response | 24h response | 4h response |
| Dedicated Slack channel | -- | -- | -- | Yes |
| Dedicated Customer Success Manager | -- | -- | 10+ seats | Yes |
| SLA (uptime guarantee) | -- | -- | 99.5% | 99.9% |
| Onboarding + training sessions | -- | -- | 2 sessions | Unlimited |
| **Extensions** | | | | |
| Community extension registry | Yes | Yes | Yes | Yes |
| Premium extensions (official) | -- | Yes | Yes | Yes |
| Private extension registry | -- | -- | -- | Yes |
| Custom extension development support | -- | -- | -- | Yes (professional services) |

### Tier Rationale

**Free (Developer Adoption Engine).** The free tier covers 90%+ of individual developer needs. The 500-entity limit accommodates most solo and small-team projects while creating a natural upgrade trigger when projects grow. The free tier is our primary acquisition channel --- every paid customer starts here.

**Pro (Power User Monetization).** $29/month targets the individual developer or freelancer who works on larger projects and wants the cloud dashboard, full traceability, and code-to-spec inference. At $29/month against $50-200/month in token savings, the ROI is immediate and obvious. This tier is 100% self-serve --- no sales involvement.

**Team (Collaboration Monetization).** $79/seat/month targets engineering teams of 5-50 developers. The key differentiators are collaboration features (shared workspaces, RBAC, SSO), CI/CD integration (spec coverage gates), and team-level support. This tier typically enters through a Pro user who champions the tool and requests team procurement.

**Enterprise (Organization Monetization).** Custom pricing starting at $149/seat/month targets organizations with 50+ developers. The key differentiators are self-hosted deployment, audit logging, compliance reporting, cross-repo traceability, and dedicated support. Enterprise deals are sales-assisted with proof-of-value periods.

---

## 3. Sales Motion

SpecForge employs three distinct sales motions, each optimized for a different buyer journey and deal size.

### Motion 1: Product-Led Growth (PLG) Self-Serve

**Target:** Individual developers and small teams (1-10 seats)
**Deal size:** $0-$3,500/year
**Revenue contribution:** 70% of customers, 25% of revenue by Year 3

| Stage | Trigger | Action | Owner |
|---|---|---|---|
| Awareness | Developer encounters SpecForge content (blog, tweet, conference talk, peer recommendation) | -- | Marketing / DevRel |
| Acquisition | `brew install specforge` or `npx specforge` | -- | Product |
| Activation | First successful `specforge check` with 5+ entities | In-app tip: "Export agent-context to reduce AI tokens by 75%" | Product |
| Engagement | Regular CLI usage (3+ days/week for 2+ weeks) | In-app prompt: "You have 487 entities --- Pro unlocks unlimited" | Product |
| Conversion | Self-serve upgrade via CLI or website | Credit card checkout, instant activation | Product |
| Expansion | User invites teammates, hits Team tier triggers | "Your team has 6 active users --- switch to Team for collaboration" | Product |

**Key PLG Metrics:**
- Install-to-activation rate (first `specforge check`): target 80%
- Activation-to-engaged rate (3+ days/week): target 40%
- Free-to-Pro conversion rate: target 8-12%
- Time to first paid conversion: median 45 days

### Motion 2: Team Expansion (Sales-Assisted PLG)

**Target:** Engineering teams (5-50 seats)
**Deal size:** $4,700-$47,400/year
**Revenue contribution:** 20% of customers, 40% of revenue by Year 3

| Stage | Trigger | Action | Owner |
|---|---|---|---|
| Signal detection | 3+ free/Pro users from same domain | Product-qualified lead (PQL) alert to SDR | RevOps |
| Outreach | SDR contacts the most active user | "Your team is already using SpecForge --- here is how Team tier helps you collaborate" | SDR |
| Champion enablement | Identified champion (usually the first adopter) | Send ROI calculator, internal business case template, pilot plan | AE |
| Technical evaluation | Champion runs proof-of-value with their team | Provide dedicated Slack channel, sandbox environment, spec migration guide | SE |
| Procurement | Champion takes proposal to engineering lead | Procurement-ready quote, security questionnaire, SOC 2 Type II report | AE |
| Close | Annual contract signed | Onboarding kickoff within 5 business days | CSM |
| Expand | Team grows, additional teams adopt | Quarterly business review, expansion proposal at 6-month mark | CSM + AE |

**Key Team Metrics:**
- PQL-to-opportunity rate: target 30%
- Opportunity-to-close rate: target 40%
- Average sales cycle: 30-45 days
- Average deal size at close: $15,800/year (12 seats x $79 x 12 months x annual discount)

### Motion 3: Enterprise Sales (Outbound + Inbound)

**Target:** Organizations with 50+ developer seats
**Deal size:** $90,000-$500,000+/year
**Revenue contribution:** 10% of customers, 35% of revenue by Year 3

| Stage | Trigger | Action | Owner |
|---|---|---|---|
| Prospecting | Inbound (executive requests demo) or outbound (target account list) | Discovery call: understand compliance needs, AI agent stack, team size | AE |
| Qualification | MEDDPICC qualification | Confirm: Metrics, Economic Buyer, Decision Criteria, Decision Process, Paper Process, Implications, Champion, Competition | AE |
| Technical deep-dive | Architecture review with customer's engineering team | Custom proof-of-value plan, integration assessment, security review | SE |
| Proof of Value | 30-60 day pilot on one team or repository | Define success criteria upfront, weekly check-ins, final report | SE + CSM |
| Executive alignment | Present pilot results to economic buyer (VP Eng / CTO) | ROI analysis: token savings, developer productivity, compliance value | AE |
| Negotiation | Contract terms, deployment model, SLA | Legal review, InfoSec review, procurement process | AE + Legal |
| Close | Multi-year contract (2-3 year typical) | Dedicated implementation team, 90-day onboarding program | CSM |
| Land & Expand | Initial team succeeds, other teams request access | Expansion pricing pre-negotiated, internal case study created | CSM + AE |

**Key Enterprise Metrics:**
- Qualified pipeline-to-close rate: target 25-35%
- Average sales cycle: 90-120 days
- Average initial deal size: $180,000/year (100 seats)
- Year 2 expansion: 140% of initial deal value

---

## 4. Conversion Funnel Targets

### Overall Funnel (Year 1 through Year 5)

| Funnel Stage | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| **Free users (cumulative)** | 3,500 | 12,000 | 30,000 | 65,000 | 120,000 |
| **Free-to-trial conversion** | 5% | 8% | 10% | 11% | 12% |
| **Trial-to-paid conversion** | 20% | 25% | 30% | 32% | 35% |
| **Overall free-to-paid** | 1.0% | 2.0% | 3.0% | 3.5% | 4.2% |
| **Paid seats (cumulative)** | 85 | 520 | 1,500 | 3,660 | 7,900 |
| **Net revenue retention** | 100% | 112% | 122% | 128% | 132% |
| **Logo churn (annual)** | 25% | 20% | 15% | 12% | 10% |
| **Revenue churn (annual)** | 20% | 15% | 10% | 8% | 6% |
| **Expansion rate** | 100% | 115% | 130% | 135% | 140% |

### Conversion by Tier Transition

| Transition | Target Rate | Typical Timeline | Key Trigger |
|---|---|---|---|
| Free -> Pro trial | 8-12% | 30-60 days of free usage | Entity limit reached (500+) or traceability need |
| Pro trial -> Pro paid | 25-35% | 14-day trial period | Agent-context value demonstrated, ROI realized |
| Pro -> Team | 15-20% of Pro accounts | 3-6 months after Pro conversion | 3+ colleagues start using SpecForge |
| Team -> Enterprise | 8-12% of Team accounts | 6-12 months after Team conversion | Compliance needs, self-hosted requirement, 50+ seats |
| Free -> Team (skip Pro) | 3-5% | 60-90 days | Engineering lead discovers SpecForge, immediate team pilot |
| Free -> Enterprise (skip tiers) | <1% | 90-180 days | Enterprise architect evaluates for org-wide rollout |

### Funnel Health Indicators

| Metric | Healthy | Warning | Critical |
|---|---|---|---|
| Free-to-trial conversion | >8% | 5-8% | <5% |
| Trial-to-paid conversion | >25% | 15-25% | <15% |
| Time to first value | <60 seconds | 60-300 seconds | >5 minutes |
| 30-day retention (free) | >40% | 25-40% | <25% |
| 90-day retention (paid) | >85% | 70-85% | <70% |
| NPS (paid users) | >50 | 30-50 | <30 |

---

## 5. Enterprise Sales Strategy

### Target Enterprise Profile

| Attribute | Ideal Customer Profile |
|---|---|
| Company size | 200-5,000 engineers |
| AI agent maturity | Using AI coding tools (Copilot, Cursor, Claude Code) in production |
| Existing pain | Token costs >$10K/month; context management is a known problem |
| Industry vertical | FinTech, HealthTech, SaaS platforms, automotive software, defense |
| Compliance needs | SOC 2, ISO 27001, FDA 21 CFR Part 11, HIPAA |
| Decision maker | VP Engineering, CTO, or Head of Developer Productivity |
| Champion profile | Staff+ engineer or engineering manager who already uses SpecForge free |

### Deal Size Segmentation

| Segment | Seats | Annual Contract Value | Sales Cycle | Sales Resources |
|---|---|---|---|---|
| **Mid-Market** | 25-100 | $45K-$180K | 60-90 days | 1 AE + 1 SE |
| **Enterprise** | 100-500 | $180K-$900K | 90-120 days | 1 AE + 1 SE + 1 CSM |
| **Strategic** | 500+ | $900K-$3M+ | 120-180 days | 1 AE + 2 SE + 1 CSM + exec sponsor |

### Enterprise Sales Cycle

```
Week 1-2:   Discovery & Qualification (MEDDPICC)
Week 3-4:   Technical Deep-Dive & Architecture Review
Week 5-8:   Proof of Value (30-day pilot on 1 team)
Week 9-10:  Pilot Results & ROI Presentation to Economic Buyer
Week 11-12: Negotiation, Security Review, Legal Review
Week 13-14: Contract Execution & Onboarding Kickoff
```

### Objection Handling Framework

| Objection | Response Strategy | Supporting Evidence |
|---|---|---|
| "We already use CLAUDE.md / .cursor rules" | "SpecForge compiles where text files drift. Show the validation codes that catch errors a text file never will. Offer side-by-side comparison pilot." | Token reduction benchmarks: 75-86% vs. 20-30% for plain text |
| "Our developers won't adopt another tool" | "SpecForge is a 60-second install that lives inside their existing workflow. No new IDE, no new UI. CLI + LSP in the editor they already use." | Time-to-first-value data: <60 seconds |
| "We're concerned about lock-in" | "The .spec format is open-source (MIT). The CLI compiler is open-source. Your specs are text files in your git repo. You can leave anytime." | Open-source codebase, no proprietary data format |
| "We can build this in-house" | "Building a zero-entity-core compiler with extension-defined entity types, tree-sitter parser, LSP, and Wasm extension system takes 18-24 months and 3-5 engineers. That is $1-2M in opportunity cost vs. $180K/year for SpecForge." | Compiler complexity estimates, Buf/Terraform development timelines |
| "Token costs are dropping" | "Token volume per agent run is increasing faster than per-token costs are dropping. Structured context improves accuracy, not just cost. The value compounds." | Industry data on token consumption growth |
| "We need SOC 2 / ISO compliance" | "SpecForge Enterprise generates Requirements Traceability Matrices (RTM) that map directly to ISO 27001 Annex A controls and SOC 2 trust criteria." | RTM export samples, compliance officer testimonials |
| "The price is too high" | "At $149/seat/month for 100 seats, the annual cost is $178K. Your current AI token waste at 100 engineers is $60K-$240K/year. SpecForge pays for itself in token savings alone, before counting productivity gains." | ROI calculator with customer-specific inputs |

### Proof of Value (PoV) Process

**Duration:** 30 days (extendable to 60 for strategic accounts)

| Phase | Duration | Activities | Success Criteria |
|---|---|---|---|
| **Setup** | Days 1-3 | Install SpecForge on pilot team (5-10 devs), migrate 1 existing project to .spec format, configure CI integration | All team members have working CLI + LSP |
| **Baseline** | Days 4-7 | Measure current AI agent token consumption, task completion rates, and context loading time | Baseline metrics documented |
| **Active Use** | Days 8-25 | Team writes specs for active features, uses agent-context output, tracks token usage and task quality | 20+ entities authored, 10+ AI agent tasks using spec context |
| **Measurement** | Days 26-28 | Compare token consumption, task completion rate, and developer satisfaction against baseline | Minimum 50% token reduction, positive developer NPS |
| **Report** | Days 29-30 | Present findings to economic buyer with ROI projection and rollout plan | Go/no-go decision |

**PoV Pricing:** Free for up to 10 seats for 30 days. Extended PoV (60 days, 25 seats): $5,000 credited against first-year contract.

---

## 6. Partnership Strategy

### 6.1 AI Tool Partnerships (Highest Priority)

These partnerships put SpecForge in front of the exact developers who need it most.

| Partner | Partnership Type | Value to SpecForge | Value to Partner | Timeline |
|---|---|---|---|---|
| **Anthropic (Claude Code)** | Native integration | Distribution to 500K+ Claude Code users | Better agent performance = higher retention | Year 1 |
| **Cursor / Anysphere** | Editor plugin + MCP server | Distribution to 1M+ Cursor users | Spec-aware completions outperform file-scanning | Year 1 |
| **GitHub Copilot** | Copilot extension | Distribution to 1.8M+ Copilot users | Structured context reduces hallucinations | Year 1-2 |
| **Continue.dev** | Open-source integration | Community credibility, OSS distribution | Spec graph as context provider | Year 1 |
| **Cody (Sourcegraph)** | Integration partnership | Enterprise distribution channel | Better context management for enterprise users | Year 2 |

**Partnership Economics:** No revenue share on integrations. SpecForge benefits from distribution; partners benefit from improved AI performance. Paid tiers remain SpecForge-direct.

### 6.2 System Integrators & Consultancies

| Partner Type | Examples | Deal Structure | Target Year |
|---|---|---|---|
| **DevOps consultancies** | Thoughtworks, Contino, Slalom | Referral fee (15-20% of Year 1 ACV) | Year 2-3 |
| **AI implementation firms** | Cognizant AI, Accenture AI | Co-sell with enterprise accounts, mutual training | Year 3-4 |
| **Compliance consultancies** | A-LIGN, Schellman | Joint offering: SpecForge + audit prep | Year 3 |

### 6.3 Technology Alliances

| Alliance | Integration | Mutual Benefit |
|---|---|---|
| **Atlassian (Jira)** | `@specforge/jira` provider --- bidirectional sync | Jira becomes spec-aware; SpecForge validates Jira references |
| **GitHub** | `@specforge/gh` provider + Actions marketplace | PR-level spec coverage; GitHub improves code review UX |
| **GitLab** | GitLab CI templates + provider | Enterprise SCM integration; GitLab differentiates in AI-native tooling |
| **Datadog / Grafana** | Spec coverage as observability metric | Novel metric for engineering health; deepens monitoring platform |
| **Terraform (HashiCorp)** | `@specforge/infrastructure` domain extension | SpecForge validates infra specs; HCP gains a specification layer |

### 6.4 Partnership Revenue Targets

| Partnership Type | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|
| AI tool co-marketing (pipeline influence) | $30K | $120K | $400K | $1.0M |
| SI/consultancy referrals (direct revenue) | $0 | $50K | $200K | $500K |
| Technology alliance co-sell | $0 | $30K | $150K | $400K |
| **Total partner-influenced revenue** | **$30K** | **$200K** | **$750K** | **$1.9M** |

---

## 7. Revenue Mix Projections

### 7.1 Revenue by Tier (5-Year)

| Tier | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| **Pro ($29/dev/mo)** | $18K | $88K | $226K | $490K | $940K |
| **Team ($79/dev/mo)** | $7K | $113K | $361K | $854K | $1,800K |
| **Enterprise (custom)** | $0 | $49K | $238K | $856K | $2,460K |
| **Subscription ARR** | **$25K** | **$250K** | **$825K** | **$2,200K** | **$5,200K** |
| **Professional Services** | $0 | $45K | $180K | $480K | $1,000K |
| **Marketplace Revenue** | $0 | $0 | $35K | $125K | $350K |
| **Total Revenue** | **$25K** | **$295K** | **$1,040K** | **$2,830K** | **$6,550K** |

### Subscription Mix Shift

| Tier | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| Pro | 72% | 35% | 27% | 22% | 18% |
| Team | 28% | 45% | 44% | 39% | 35% |
| Enterprise | 0% | 20% | 29% | 39% | 47% |

The Enterprise shift is deliberate: Enterprise revenue grows from 0% to 47% of subscription ARR by Year 5, driven by the land-and-expand motion and multi-year contracts with built-in expansion.

### 7.2 Revenue by Sales Motion

| Motion | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| **PLG Self-Serve** | $22K (88%) | $162K (55%) | $395K (38%) | $792K (28%) | $1,440K (22%) |
| **Sales-Assisted (Team)** | $3K (12%) | $88K (30%) | $337K (32%) | $878K (31%) | $2,080K (32%) |
| **Enterprise Sales** | $0 (0%) | $45K (15%) | $308K (30%) | $1,160K (41%) | $3,030K (46%) |
| **Total Revenue** | **$25K** | **$295K** | **$1,040K** | **$2,830K** | **$6,550K** |

### 7.3 Revenue by Geography

| Region | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| **North America** | 70% | 65% | 60% | 55% | 50% |
| **Europe (EMEA)** | 20% | 22% | 25% | 27% | 28% |
| **Asia-Pacific** | 8% | 10% | 12% | 14% | 17% |
| **Rest of World** | 2% | 3% | 3% | 4% | 5% |

Geographic expansion timeline:
- **Year 1-2:** North America focus. English-language content only. USD pricing.
- **Year 3:** EMEA expansion. EUR pricing option. GDPR-compliant EU data residency for Enterprise.
- **Year 4:** APAC expansion. JPY/AUD pricing. Singapore data center for Enterprise.
- **Year 5:** Multi-language documentation. Regional sales hires in London and Singapore.

### 7.4 Seat Count Projections

| Tier | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| **Pro seats** | 60 | 280 | 720 | 1,560 | 3,000 |
| **Team seats** | 25 | 140 | 380 | 900 | 1,900 |
| **Enterprise seats** | 0 | 100 | 400 | 1,200 | 3,000 |
| **Total paid seats** | **85** | **520** | **1,500** | **3,660** | **7,900** |
| **Revenue per paid seat (monthly avg)** | $24.51 | $40.06 | $45.83 | $50.09 | $54.85 |

The rising average revenue per seat reflects the increasing Enterprise mix, which carries the highest per-seat price point.

---

## 8. Customer Success & Retention

### 8.1 Onboarding Program

| Tier | Onboarding Model | Timeline | Key Activities |
|---|---|---|---|
| **Pro** | Self-serve + automated | Day 1-7 | Welcome email drip (5 emails), interactive CLI tutorial, example .spec templates, "Your first spec in 60 seconds" guide |
| **Team** | Guided + self-serve | Day 1-14 | Kickoff call with CSM, team workspace setup, CI/CD integration walkthrough, 2 live training sessions, migration plan for existing context files |
| **Enterprise** | White-glove | Day 1-90 | Dedicated implementation team, architecture review, spec migration for 1-2 pilot projects, custom plugin assessment, admin training, exec sponsor alignment |

### 8.2 Customer Health Score

Health score computed weekly on a 0-100 scale:

| Signal | Weight | Healthy (>70) | At-Risk (40-70) | Critical (<40) |
|---|---|---|---|---|
| **CLI usage frequency** | 25% | Daily active users >60% of seats | 30-60% of seats | <30% of seats |
| **Entity count growth** | 15% | Growing month-over-month | Flat | Declining |
| **Feature breadth** | 15% | Using 5+ commands regularly | Using 2-4 commands | Using only `check` |
| **Support ticket sentiment** | 15% | Positive or no tickets | Mixed sentiment | Negative sentiment |
| **Spec coverage trend** | 10% | Improving quarter-over-quarter | Flat | Declining |
| **Invoice payment** | 10% | Paid on time | 1-15 days late | >15 days late |
| **Champion engagement** | 10% | Responds to CSM outreach | Slow to respond | Unresponsive |

### 8.3 Expansion Playbook

| Expansion Vector | Trigger | Play | Expected Revenue Impact |
|---|---|---|---|
| **Seat expansion** | Team adds new developers | Quarterly seat true-up at renewal, prorated mid-cycle adds | +20-40% per account per year |
| **Tier upgrade (Pro to Team)** | 3+ Pro users from same domain | Team trial offer, champion enablement kit | +172% per-seat revenue lift |
| **Tier upgrade (Team to Enterprise)** | Compliance requirements or 50+ seats | Executive briefing on RTM + audit capabilities | +89% per-seat revenue lift |
| **Cross-department expansion** | Champion moves teams or promotes SpecForge internally | Internal case study, "SpecForge Ambassador" program | +100-300% account revenue |
| **Multi-year commitment** | Approaching annual renewal | 2-year: 10% discount, 3-year: 15% discount + locked pricing | +200-300% contracted value |

### 8.4 Churn Prevention

| Churn Signal | Detection Method | Intervention | Owner |
|---|---|---|---|
| Usage decline (>30% drop over 30 days) | Health score alert | CSM outreach: "We noticed lower usage --- let's schedule a check-in" | CSM |
| Champion leaves the company | LinkedIn monitoring + CRM signals | Identify new champion within 2 weeks, re-onboard if needed | CSM |
| Support escalation | Ticket flagged as P1 | Engineering engagement within 4 hours (Enterprise) / 24 hours (Team) | Support + Engineering |
| Competitor evaluation | Discovery during QBR or support interaction | Competitive displacement kit: feature comparison, migration risk analysis | CSM + AE |
| Budget cut signals | Champion mentions cost reduction | Present cost-savings analysis: token reduction ROI, consolidation of other tools | CSM + AE |
| Failed renewal negotiation | Renewal discussion stalls | VP Sales engagement, custom retention offer (max 20% discount for 1 year) | VP Sales |

### 8.5 Retention Targets

| Metric | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| **Gross logo retention** | 75% | 80% | 85% | 88% | 90% |
| **Gross revenue retention** | 80% | 85% | 90% | 92% | 94% |
| **Net revenue retention** | 100% | 112% | 122% | 128% | 132% |
| **Average customer lifespan** | 2.5 years | 3.0 years | 3.5 years | 4.0 years | 4.5 years |

---

## 9. Competitive Pricing Analysis

### 9.1 Direct Competitor Positioning

| Attribute | **SpecForge** | **Terraform Cloud** | **Datadog** | **Snyk** | **Buf** |
|---|---|---|---|---|---|
| **Category** | Spec compiler for AI agents | IaC platform | Observability | Security scanning | Protobuf compiler |
| **Free tier** | Full compiler, 500 entities | 500 resources | 5 hosts | 200 tests/month | Unlimited (CLI) |
| **Entry paid price** | $29/dev/mo | $0 (Plus plan deprecated) | $15/host/mo | $25/dev/mo | $0 (BSR free tier) |
| **Team price** | $79/dev/mo | Custom (Plus) | $23/host/mo | $52/dev/mo | Custom |
| **Enterprise price** | From $149/dev/mo | Custom | $34/host/mo + custom | Custom ($100+/dev/mo) | Custom |
| **Pricing model** | Per-seat | Per-resource | Per-host + usage | Per-seat | Per-seat |
| **Free-to-paid friction** | Low (CLI upgrade) | Moderate | High (agent install) | Low (CLI upgrade) | Low |
| **Open source core** | Yes (MIT) | Yes (BSL 1.1) | No | Partial (limited OSS) | Yes (Apache 2.0) |
| **PLG motion** | Strong | Moderate | Weak | Strong | Strong |

### 9.2 Price-Value Positioning Map

```
                            HIGH VALUE DELIVERED
                                    |
                         SpecForge  |  Terraform Cloud
                         Enterprise |  Enterprise
                                    |
            SpecForge Pro    -------+--------  Snyk Team
                                    |
        SpecForge Free       -------+--------  Datadog (free trial)
                                    |
                            LOW VALUE DELIVERED

        LOW PRICE -------- MEDIUM -------- HIGH PRICE
```

**Positioning Strategy:** SpecForge occupies the "high value, moderate price" quadrant. We are priced below Snyk and Datadog at equivalent team sizes but deliver measurable, immediate ROI through token reduction. The free tier is more generous than any competitor except Buf.

### 9.3 Total Cost of Ownership (TCO) Comparison for 50-Dev Team

| Cost Component | SpecForge (Team) | Terraform Cloud | Datadog | Snyk |
|---|---|---|---|---|
| Annual license | $47,400 | $90,000+ | $138,000+ | $31,200+ |
| Implementation cost | $5,000 | $25,000 | $40,000 | $10,000 |
| Training cost | $2,000 | $10,000 | $15,000 | $5,000 |
| Annual maintenance | Included | $15,000 | $20,000 | $8,000 |
| **Year 1 TCO** | **$54,400** | **$140,000** | **$213,000** | **$54,200** |
| **Year 2 TCO** | **$47,400** | **$105,000** | **$158,000** | **$39,200** |
| **Token savings (Year 1)** | -$60,000 to -$180,000 | N/A | N/A | N/A |
| **Net Year 1 cost** | **-$5,600 to -$125,600** | **$140,000** | **$213,000** | **$54,200** |

SpecForge is the only tool in this comparison that can achieve negative net cost in Year 1 due to direct token reduction savings.

---

## 10. Discounting Framework

### 10.1 Standard Discount Schedule

| Discount Type | Eligibility | Discount | Approval |
|---|---|---|---|
| **Annual commitment** | Any paid tier | 17% off monthly price | Automatic (self-serve) |
| **2-year commitment** | Team + Enterprise | 10% off annual price | AE approval |
| **3-year commitment** | Enterprise only | 15% off annual price + price lock | VP Sales approval |
| **Volume (25-99 seats)** | Team + Enterprise | 5% | AE approval |
| **Volume (100-249 seats)** | Team + Enterprise | 10% | AE approval |
| **Volume (250-499 seats)** | Enterprise only | 15% | VP Sales approval |
| **Volume (500+ seats)** | Enterprise only | 20% | CRO approval |

### 10.2 Strategic Discount Programs

**Startup Program**
- Eligibility: <50 employees, <$10M raised, <3 years old
- Offer: Team tier at 50% off for 12 months (then 25% off for 12 months, then full price)
- Application: Self-serve via website with verification (Crunchbase/LinkedIn)
- Goal: 200 startups enrolled by Year 3, converting 15-20% to full-price customers

**Academic & Research Program**
- Eligibility: Accredited educational institutions, non-commercial research labs
- Offer: Team tier free for up to 50 seats per institution
- Application: Email verification with .edu domain or institutional letter
- Goal: 50 universities by Year 3, creating a pipeline of developers trained on SpecForge

**Open Source Maintainer Program**
- Eligibility: Maintainers of open-source projects with 1,000+ GitHub stars
- Offer: Pro tier free for the maintainer + 50% off Team tier for project contributors (up to 20 seats)
- Application: GitHub verification
- Goal: 100 OSS projects using SpecForge by Year 3, driving community credibility

**Non-Profit Program**
- Eligibility: Registered 501(c)(3) or equivalent
- Offer: 40% off all tiers
- Application: Non-profit verification via TechSoup or equivalent

### 10.3 Discounting Guardrails

| Rule | Rationale |
|---|---|
| Maximum total discount: 30% (non-program) | Protects margins and prevents race-to-bottom |
| No discounts on Pro tier (except programs above) | Pro is already priced for individual ROI; discounting undermines value signal |
| Discounts require minimum 1-year annual commitment | Prevents discount arbitrage on monthly billing |
| Competitive displacement: up to 25% for 1 year only | Win the deal, then prove value at full price |
| Multi-year discounts must include expansion clause | Locked pricing applies to initial seats; expansion at list price |
| All discounts >15% require VP Sales written approval | Prevents margin erosion from field deals |
| Discount approval expires in 30 days | Creates urgency, prevents stale quotes |

### 10.4 Discount Impact Modeling

| Scenario | Blended Discount | Impact on Year 5 Subscription ARR | Impact on Gross Margin |
|---|---|---|---|
| No discounts | 0% | $5,200K (baseline) | 96% |
| Conservative discounting (10% blended) | 10% | $4,680K (-10%) | 95.5% |
| Moderate discounting (15% blended) | 15% | $4,420K (-15%) | 95.2% |
| Aggressive discounting (20% blended) | 20% | $4,160K (-20%) | 94.8% |

**Target:** Maintain blended discount rate below 12% across all customers. This preserves Year 5 subscription ARR above $4,576K while remaining competitive in enterprise procurement.

---

## Key Metrics Dashboard (VP Sales View)

| Metric | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|---|---|---|---|---|---|
| **Total Revenue** | $25K | $295K | $1,040K | $2,830K | $6,550K |
| **Subscription ARR** | $25K | $250K | $825K | $2,200K | $5,200K |
| **Total Paid Seats** | 85 | 520 | 1,500 | 3,660 | 7,900 |
| **Free Users** | 3,500 | 12,000 | 30,000 | 65,000 | 120,000 |
| **Free-to-Paid Conversion** | 1.0% | 2.0% | 3.0% | 3.5% | 4.2% |
| **Net Revenue Retention** | 100% | 112% | 122% | 128% | 132% |
| **Blended LTV:CAC** | 4.0:1 | 8.7:1 | 11.2:1 | 13.8:1 | 16.1:1 |
| **Average ACV (paid accounts)** | $490 | $2,460 | $5,500 | $9,480 | $14,200 |
| **Enterprise Accounts** | 0 | 2 | 8 | 24 | 60 |
| **Sales Team Size** | 0 (founder-led) | 2 (1 AE + 1 SDR) | 5 (2 AE + 2 SDR + 1 SE) | 10 | 18 |
| **Revenue per Sales FTE** | N/A | $148K | $208K | $283K | $364K |
