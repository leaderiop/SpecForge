# product-launch: a product team launching a desk lamp

A product team is launching **Halo**, a desk lamp. This project holds what the team knows about the
launch as one checked graph: who the lamp is for, where it is sold, what it does, what ships when, the
words the team agrees on, and the decisions and risks around it. There is no code in it.

Extensions: `@specforge/product` and `@specforge/governance`.

## Try it

Use a `specforge` built from this repository (`cargo build --release -p specforge-cli`, then
`target/release/specforge`).

```bash
cd examples/product-launch
specforge check                 # 0 errors, 0 warnings, 0 infos
specforge stats                 # 26 entities, 49 edges
specforge trace launch          # the release, down to deliverables, journeys and personas
specforge query preorder --depth 1
```

## See the referee at work

Say someone marks the retail milestone as done while its packaging feature is still in progress:

```bash
specforge set retail_sell_in status completed --dry-run
# warning[W154]: milestone 'retail_sell_in' is completed but its feature 'shelf_ready_box' is in_progress
```

The rule comes from `@specforge/product`, not from this project. It holds for anyone who edits the
files: a person, a script or an AI agent working over MCP.

## What is in it

| File | Kinds | What it says |
|---|---|---|
| `spec/audience.spec` | `persona`, `channel` | a remote worker and a retail buyer; the online store, retail partners, the newsletter |
| `spec/features.spec` | `feature` | what the lamp does for buyers, with acceptance criteria and owners |
| `spec/journeys.spec` | `journey` | from newsletter to pre-order; from sample to first retail order |
| `spec/plan.spec` | `module`, `milestone`, `deliverable`, `release` | two workstreams, two checkpoints, three deliverables, one launch release |
| `spec/glossary.spec` | `term` | sell-in, sell-through, ship week |
| `spec/decisions.spec` | `decision`, `constraint`, `failure_mode` | open online first; the launch budget; the stock-out risk |

Some of the product extension's words lean towards software: a deliverable's `artifact_type` is one of
`cli`, `service`, `web_app`, `documentation`, ... so the press kit is `documentation` and the store page
is a `web_app`. A team that wants its own words (campaign, asset, retailer) writes its own extension;
[`../marketing/`](../marketing/) does that.
