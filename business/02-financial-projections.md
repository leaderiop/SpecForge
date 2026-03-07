# FINANCIAL PROJECTIONS

*CFO Perspective — Unit economics, revenue modeling, cost structure, and funding strategy for an open-core developer infrastructure company targeting the AI agent structured context market.*

---

## 1. Revenue Model & Unit Economics

### Revenue Streams

**Primary Revenue (80-85% of total)**
- **Pro Plan**: $29/developer/month ($24 billed annually)
- **Team Plan**: $79/developer/month ($67 billed annually)
- **Enterprise**: Custom pricing, from $149/developer/month

**Secondary Revenue (15-20% of total)**
- **Professional Services**: Implementation consulting, custom extension development, training
- **Extension Marketplace Revenue Share**: 20% commission on third-party paid extensions (Year 3+)
- **Certification Programs**: SpecForge extension author certification (Year 3+)

### Unit Economics (Year 2 Steady State)

| Metric | Pro | Team | Enterprise |
|--------|-----|------|------------|
| Monthly price per seat | $29 | $79 | $149+ |
| Effective annual price (blended) | $26 | $73 | $149 |
| Average seats per account | 1-3 | 8-20 | 50-200 |
| Average account MRR | $58 | $730 | $11,175 |
| CAC | $180 | $1,500 | $18,000 |
| Gross Margin | 88% | 90% | 85% |
| LTV (36 mo, adjusted for churn) | $1,680 | $21,024 | $321,300 |
| LTV:CAC Ratio | 9.3:1 | 14.0:1 | 17.9:1 |
| Payback Period | 3.5 months | 2.5 months | 2 months |

**Key assumptions:**
- Pro annual churn: 25% (monthly: 2.3%)
- Team annual churn: 12% (monthly: 1.1%)
- Enterprise annual churn: 5% (monthly: 0.4%)
- Enterprise NRR: 135-148% (seat expansion + tier upgrades)
- Free-to-Pro conversion: 2.0-3.5% (PLG motion)

---

## 2. Five-Year Revenue Projections

### Base Scenario (Most Likely)

| Metric | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|--------|--------|--------|--------|--------|--------|
| **Free Users** | 4,000 | 15,000 | 40,000 | 85,000 | 150,000 |
| **Pro Seats** | 80 | 375 | 960 | 2,100 | 4,000 |
| **Team Seats** | 30 | 180 | 500 | 1,200 | 2,600 |
| **Enterprise Seats** | 0 | 125 | 500 | 1,500 | 3,800 |
| **Subscription ARR** | $56K | $510K | $1.68M | $4.55M | $10.85M |
| **Services Revenue** | $0 | $55K | $220K | $580K | $1.2M |
| **Marketplace Revenue** | $0 | $0 | $45K | $165K | $450K |
| **Total Revenue** | **$56K** | **$565K** | **$1.95M** | **$5.30M** | **$12.50M** |
| **YoY Growth** | — | 909% | 245% | 172% | 136% |

### Conservative Scenario

| Metric | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|--------|--------|--------|--------|--------|--------|
| **Free Users** | 2,500 | 8,000 | 20,000 | 40,000 | 70,000 |
| **Total Revenue** | **$30K** | **$280K** | **$920K** | **$2.4M** | **$5.5M** |

### Optimistic Scenario

| Metric | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|--------|--------|--------|--------|--------|--------|
| **Free Users** | 8,000 | 30,000 | 80,000 | 170,000 | 300,000 |
| **Total Revenue** | **$95K** | **$1.1M** | **$4.2M** | **$12.0M** | **$28.5M** |

**Scenario drivers:**
- Conservative: Slower community adoption, 1.5% conversion, fewer enterprise pilots
- Base: PLG growth with category-creation momentum, 2.5% conversion
- Optimistic: AI agent adoption accelerates, SpecForge becomes de facto standard early, 4% conversion

---

## 3. Cost Structure (Base Scenario)

| Category | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|----------|--------|--------|--------|--------|--------|
| Engineering (compiler, extensions, cloud) | $480K | $960K | $1.9M | $2.9M | $4.0M |
| Sales & Marketing | $120K | $450K | $1.1M | $2.0M | $3.2M |
| Developer Relations & Community | $90K | $280K | $520K | $780K | $1.0M |
| Customer Success | $0 | $100K | $220K | $420K | $650K |
| G&A (legal, finance, ops) | $180K | $380K | $600K | $850K | $1.1M |
| Cloud Infrastructure | $30K | $70K | $160K | $340K | $620K |
| **Total OpEx** | **$900K** | **$2.24M** | **$4.50M** | **$7.29M** | **$10.57M** |
| **EBITDA** | **-$844K** | **-$1.68M** | **-$2.55M** | **-$1.99M** | **$1.93M** |
| **EBITDA Margin** | -1,507% | -297% | -131% | -38% | 15% |

### Gross Margin

| Metric | Year 1 | Year 2 | Year 3 | Year 4 | Year 5 |
|--------|--------|--------|--------|--------|--------|
| **Gross Margin** | 86% | 89% | 91% | 92% | 93% |

High gross margins reflect the open-core model: the compiler is MIT-licensed and self-hosted. Cloud costs scale sub-linearly (graph storage is lightweight). No per-query compute costs — SpecForge is deterministic infrastructure, not an AI SaaS.

---

## 4. Break-Even Analysis

| Milestone | Timeline | Monthly Metrics |
|-----------|----------|-----------------|
| **Gross Profit Positive** | Q2 Year 1 | From day one (near-zero COGS) |
| **Operating Cash Flow Positive** | Q1 Year 5 | $780K MRR, $9.4M ARR |
| **EBITDA Positive** | Q2 Year 5 | $880K MRR, $10.6M ARR |

**Break-even requires:**
- ~4,000 Pro seats + 2,600 Team seats + 3,800 Enterprise seats
- Or approximately 150,000 free users at 2.5% blended conversion
- Enterprise NRR above 130% (expansion via seat growth and domain-extension adoption)

---

## 5. Key Financial Metrics

### LTV:CAC Ratios (Improving Over Time)

| Segment | Year 2 | Year 3 | Year 5 |
|---------|--------|--------|--------|
| Pro | 7.5:1 | 9.3:1 | 12.0:1 |
| Team | 11.5:1 | 14.0:1 | 19.0:1 |
| Enterprise | 14.0:1 | 17.9:1 | 22.5:1 |
| **Blended** | **10.8:1** | **13.5:1** | **18.2:1** |

LTV:CAC improves as: (a) brand awareness reduces CAC, (b) extension ecosystem increases stickiness, (c) Graph Protocol adoption creates switching costs.

### Net Revenue Retention

| Segment | Year 2 | Year 3 | Year 5 |
|---------|--------|--------|--------|
| Pro | 95% | 98% | 102% |
| Team | 120% | 128% | 135% |
| Enterprise | 138% | 145% | 155% |
| **Company** | **115%** | **125%** | **138%** |

Enterprise NRR driven by: seat expansion (teams grow), extension adoption (compliance → software → data), cross-repo traceability upgrades.

---

## 6. Funding Requirements

| Round | Amount | Valuation | Dilution | Timeline | Gate |
|-------|--------|-----------|----------|----------|------|
| Bootstrap | $0 | N/A | 0% | Months 0-12 | CLI v1.0, 2K+ stars, 50+ external specs |
| Seed | $2M | $10-15M post | 15-20% | Month 12-18 | 100+ WAU, community traction |
| Series A | $10M | $50-80M post | 15-20% | Month 24-30 | $500K-1M ARR, 10K+ stars, 5+ enterprise pilots |
| Series B | $25M | $150-250M post | 12-18% | Month 42-54 | $5-10M ARR, 100+ enterprise, 130%+ NRR |
| **Total** | **$37M** | | **~50%** | | |

### Founder Equity Remaining: ~50% (after all rounds, assuming no secondary)

### Alternative: Bootstrap to Profitability Path

If the product reaches 150K free users organically and achieves 2.5% conversion with a lean team (8-12 people), the base scenario reaches profitability in Year 5 without external capital. This is the "Buf-before-funding" path — prove value with community before raising.

---

## 7. Use of Funds Per Round

### Seed ($2M)

| Category | Allocation | Amount | Hires |
|----------|-----------|--------|-------|
| Engineering | 55% | $1.1M | 3-4 engineers |
| Developer Relations | 20% | $400K | 1-2 DevRel |
| Operations | 10% | $200K | Tooling, legal, infra |
| Founder salaries | 10% | $200K | 2 founders |
| Buffer | 5% | $100K | Contingency |

**Seed deliverables:** Cloud beta, 5 first-party extensions, MCP server, 10+ community extensions.

### Series A ($10M)

| Category | Allocation | Amount | Hires |
|----------|-----------|--------|-------|
| Engineering | 50% | $5M | 8-10 engineers |
| Sales & Marketing | 25% | $2.5M | 3-5 sales, marketing team |
| Developer Relations | 10% | $1M | 2-3 DevRel |
| G&A | 10% | $1M | Finance, legal, HR |
| Buffer | 5% | $500K | Contingency |

**Series A deliverables:** Enterprise features (SSO, RBAC, audit), extension marketplace, 3+ agent platform integrations, federation, 50+ extensions.

---

## 8. Cash Flow Summary

| Year | Revenue | OpEx | Net Cash Flow | Capital Raised | Ending Cash |
|------|---------|------|---------------|----------------|-------------|
| Year 1 | $56K | $900K | -$844K | $2.0M | $1.16M |
| Year 2 | $565K | $2.24M | -$1.68M | $10.0M | $9.48M |
| Year 3 | $1.95M | $4.50M | -$2.55M | $0 | $6.93M |
| Year 4 | $5.30M | $7.29M | -$1.99M | $25.0M | $29.94M |
| Year 5 | $12.50M | $10.57M | +$1.93M | $0 | $31.87M |

Cash runway is always 18+ months. The Series B raise in Year 4 is conservative — if growth tracks the optimistic scenario, this round may not be needed.

---

## 9. Sensitivity Analysis

### Key Variables Impact on Year 5 ARR

| Variable | -30% | -15% | Base | +15% | +30% |
|----------|------|------|------|------|------|
| Free-to-Paid Conversion | $7.6M | $9.2M | $10.85M | $12.5M | $14.1M |
| Average Revenue Per Seat | $7.6M | $9.2M | $10.85M | $12.5M | $14.1M |
| Annual Churn Rate | $13.5M | $12.1M | $10.85M | $9.7M | $8.8M |
| Free User Growth Rate | $7.6M | $9.2M | $10.85M | $12.5M | $14.1M |
| Enterprise Seat Expansion | $9.4M | $10.1M | $10.85M | $11.6M | $12.3M |

### Break-Even Sensitivity

| Conversion \ Annual Churn | 10% | 15% | 20% (base) | 25% |
|---------------------------|------|------|------------|------|
| 1.5% | Q4 Y6 | Q2 Y6 | Q4 Y5 | Q2 Y5 |
| 2.0% | Q2 Y6 | Q4 Y5 | Q3 Y5 | Q1 Y5 |
| 2.5% (base) | Q4 Y5 | Q2 Y5 | Q2 Y5 | Q4 Y4 |
| 3.0% | Q2 Y5 | Q1 Y5 | Q4 Y4 | Q3 Y4 |
| 3.5% | Q1 Y5 | Q4 Y4 | Q3 Y4 | Q2 Y4 |

---

## 10. Financial Risk Factors

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **AI token costs drop 10x** | Reduces urgency of structured context | 20-30% | Value prop includes accuracy, not just cost. Graph improves first-attempt success from 30% to 70-85%. |
| **Slower-than-expected PLG** | Revenue delays 12-18 months | 30-40% | Extend bootstrap phase, reduce burn. Bootstrap path viable with 8-person team. |
| **Enterprise sales cycle > 6 months** | Cash flow pressure | 40-50% | PLG-first model means enterprise revenue is additive, not required. |
| **Extension ecosystem stalls** | Reduces switching costs | 20-30% | First-party extensions cover 80% of use cases. Community incentives (bounties, grants). |
| **Category fails to emerge** | No market for structured context | 15-25% | Acqui-hire value ($5-20M) from compiler + Rust team talent. |

---

## 11. Critical Success Metrics by Stage

| Stage | Metric | Target | Why It Matters |
|-------|--------|--------|----------------|
| **Bootstrap** | GitHub stars | 2,000+ | Social proof for seed |
| **Bootstrap** | External specs | 50+ | Product-market fit signal |
| **Seed** | Weekly active users | 500+ | Engagement, not just installs |
| **Seed** | Free-to-Pro conversion | >2.0% | PLG model validation |
| **Series A** | ARR | $500K-1M | Revenue predictability |
| **Series A** | Enterprise pilots | 5-10 | B2B scalability signal |
| **Series B** | NRR | >130% | Expansion revenue proves platform stickiness |
| **Series B** | Community extensions | 50+ | Ecosystem health |

---

*Financial model last updated March 2026. Projections assume: AI agent adoption continues current trajectory, token costs decline 2-3x (not 10x), PLG motion achieves 2-3.5% conversion. All figures in USD.*
