# product-launch: a product team launching a desk lamp

A product team is launching **Halo**, a desk lamp. This project holds what the team knows about the
launch as one checked graph: who the lamp is for, where it is sold, what it does, what ships when, the
words the team agrees on, and the decisions and risks around it. There is no code in it.

Extensions: `@specforge/product` and `@specforge/governance`. A product team states its rules with
`invariant` from governance, which this project already loads.

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

The product fields that classify things take the project's own words (`artifact_type media`,
`interaction_model in_person`, `family workstream`); a team that wants its own *kinds* (campaign, asset,
retailer) writes its own extension, as [`../marketing/`](../marketing/) does.
