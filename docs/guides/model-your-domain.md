# Model your domain

What you get: your business's own words — campaigns, batches, procedures, suppliers, whatever you work
with — as checked entities in one graph, with the rules that matter to you enforced on every change, and an
AI agent that can read and edit that graph without being able to break those rules.

SpecForge knows no domain. The `.spec` language has five structural keywords (`spec`, `ref`, `use`,
`defaults` and the reserved `define`); every other kind exists because an **extension** declares it. So
modelling a domain means one of two things:

1. **Use extensions that already exist** — the builtins, or someone else's — when their kinds fit.
2. **Write your own extension** when they don't.

Then you connect an agent, and the agent does most of the writing.

This guide walks through both, with a real example: `examples/marketing/`, a launch plan modelled with a
small custom extension. The [examples index](../../examples/README.md) lists the other domains (a product
launch, a regulated batch-release process, a spec-driven software method).

---

## Step 1 — Write down your domain on paper

Before any tool, answer three questions in plain words. Keep it short; you will refine it later.

| Question | Marketing example |
|---|---|
| **What things do you track?** (nouns, each with a few facts) | a *campaign* (goal, channel, dates, budget), an *audience* (who, how many), a *message* (headline, copy), a *brand rule* (the rule, why) |
| **How are they connected?** (verbs between nouns) | a campaign *targets* audiences and *uses* messages; a message *obeys* brand rules |
| **What must never happen?** (rules a reviewer would catch) | a campaign that targets nobody; copy that skipped the brand check; a campaign that ends before it starts; a brand rule no message follows |

The nouns become **kinds**, their facts **fields**, the verbs **edges** (reference fields), and the "never"
list becomes **validation rules**. That table is your domain model.

## Step 2 — Check what already exists

Install the builtins and look at what they declare before you write anything:

```bash
specforge init --extensions @specforge/product,@specforge/governance
specforge model          # every kind, field and link the enabled extensions declare
specforge extensions     # which extension declares which kinds
```

The builtin extensions are a starter set:

| Extension | Kinds | Fits |
|---|---|---|
| `@specforge/product` | feature, journey, persona, channel, module, milestone, deliverable, release, term | any product or programme plan: who it is for, what they need, how they get it, when it ships |
| `@specforge/governance` | invariant (a rule that must always hold), decision, constraint, failure_mode | rules, decisions and their reasons, limits with thresholds, risks (FMEA) — in any domain |
| `@specforge/software` | behavior, type, event, port | software design; `behavior` and `event` are useful well beyond code |
| `@specforge/formal` | property, axiom, protocol, refinement, process | state machines and machine-checkable limits (`specforge analyze --prove`) |
| `@specforge/testing` + runners | `verify` statements; test runners for Rust and vitest | proving software entities with test results |

If your table from step 1 maps onto these, stop here: write entities of those kinds and go to step 5.
`examples/product-launch/` (a team launching a desk lamp) and `examples/regulated-process/` (releasing a batch
of medicine, as an illustration) are built from builtins only.

If you find yourself bending a kind — a "feature" that is really a supplier, a `tags` field holding a status
— write your own extension instead. Bent kinds lose their checks.

> **Frameworks are domains too.** A software method such as GSD, BMAD, OpenSpec or SpecKit can be modelled as
> an extension family: its artefacts become kinds, its workflow rules become validation rules. A GSD family
> (`@gsd/*`) is being designed in this repository; none of these is built yet.

## Step 3 — Write your own extension

An extension is a small Rust crate compiled to WebAssembly. You declare kinds, fields, edges and rules; the
SDK does the rest. You need Rust with the `wasm32-wasip2` target (`rustup target add wasm32-wasip2`).

```bash
specforge extension init --name @acme/marketing --dir marketing-ext
```

That scaffolds `marketing-ext/marketing/`, a crate that builds as-is. Replace its `contribute` body with your model. Here is the shape of
the marketing extension (shortened; the full file is `examples/marketing/extension/src/lib.rs`):

```rust
use specforge_extension_sdk::prelude::*;

#[specforge_extension_sdk::extension(
    name = "@example/marketing",
    version = "0.1.0",
    short = "marketing",
    description = "Campaigns, audiences, messages and brand rules, checked"
)]
struct Marketing;

/// Where a campaign runs.
const CHANNELS: Vocabulary = Vocabulary::new(&["email", "social", "search", "events", "partners"]);

impl Contributions for Marketing {
    fn contribute(c: &mut ContributionsBuilder) {
        // A noun: a kind with its facts.
        c.kind("audience", |k| {
            k.description("A group of people a campaign speaks to").testable(false);
            k.field("description", |f| {
                f.field_type(FieldType::String).required().headline();
            });
            k.field("size", |f| {
                f.field_type(FieldType::Integer);
            });
        });

        c.kind("campaign", |k| {
            k.description("A push on one channel, to some audiences, between two dates")
                .testable(false);
            k.field("goal", |f| {
                f.field_type(FieldType::String).required().headline();
            });
            k.field("channel", |f| {
                f.vocabulary(&CHANNELS).required(); // only these words are accepted
            });
            k.field("starts", |f| {
                f.field_type(FieldType::Date).required();
            });
            k.field("ends", |f| {
                f.field_type(FieldType::Date).required();
            });
            // A verb: a reference field draws an edge to another kind.
            k.field("audiences", |f| {
                f.field_type(FieldType::ReferenceList)
                    .edge("CampaignTargetsAudience")
                    .target_kind("audience");
            });
        });

        c.edge("CampaignTargetsAudience", |e| {
            e.source_kind("campaign").target_kind("audience").description("who a campaign speaks to");
        });

        // A "never": a campaign nobody receives is an error.
        c.rule("E901", |r| {
            r.check(CheckKind::NoOutgoingEdges)
                .target_kind("campaign")
                .edge_type("CampaignTargetsAudience")
                .severity(ValidationSeverity::Error)
                .message_template("campaign '{id}' targets no audience");
        });
    }
}

specforge_extension_sdk::component_guest!(build = specforge_extension_build);
```

What each part buys you:

- **Fields with types.** `channel` takes only the words in `CHANNELS`; `starts` must be a real calendar day;
  a missing `goal` is an error. The compiler checks all of it with no code from you.
- **Reference fields.** `audiences [freelance_designers]` must name an existing `audience`, or the compile
  fails. These links are what an agent follows.
- **Rules.** Declarative checks (a missing link, a value that doesn't match, a cycle) cover most needs. A rule
  that needs logic — "ends before it starts" — is a small Rust function (`r.validate(..)`), as in the full
  example. Your codes come from the ranges reserved for you: `E900`–`E998`, `W900`–`W998`, `I900`–`I998`.
- **Commands (optional).** `c.command("calendar", ..)` adds `specforge marketing calendar` on the CLI and the
  MCP tool `specforge.marketing.calendar` for agents, answered from the graph.

Build it and check the declaration:

```bash
specforge extension build --dir marketing-ext
specforge extension validate --dir marketing-ext
```

The full builder reference is the [SDK reference](../extension-sdk.md); the step-by-step tutorial, with
compiler passes, commands and tests, is [Extending SpecForge](extending-specforge.md).

## Step 4 — Install it in a project

```bash
specforge add ./marketing-ext       # the crate directory, or the .wasm file
```

That copies the component into the project and names it in `specforge.json`. While you iterate, the config
can point at the built file instead (`"@example/marketing=extension/wasm/marketing.wasm"` in the example). Now
`specforge model` shows your kinds, and `.spec` files can use them:

```text
campaign customer_upgrade "Upgrade current customers" {
  goal      "5% of free users move to Lumen Pro."
  channel   email
  starts    2027-03-02
  ends      2027-03-16
  audiences [current_customers]
  messages  [upgrade_offer]
}
```

(The block is `text` here because it needs the marketing extension to compile; in `examples/marketing/` it
does, with zero errors.)

## Step 5 — Watch the rules work

Every change goes through the same check, from a person or an agent. In the marketing example:

```bash
specforge set customer_upgrade ends 2027-02-20
#   error[E080]: the edit would leave the project with 1 error it does not have; nothing was written
#     error[E903]: campaign 'customer_upgrade' ends (2027-02-20) before it starts

specforge new campaign retarget "Retarget visitors" --goal "Bring back 500 visitors." \
  --channel search --starts 2027-04-01 --ends 2027-04-30 --in spec/campaigns.spec
#   error[E080]: the edit would leave the project with 1 error it does not have; nothing was written
#     error[E901]: campaign 'retarget' targets no audience

specforge marketing calendar
#   2027-03-02 to 2027-03-16  email    customer_upgrade
#   2027-03-02 to 2027-03-31  social   launch_social
#   2027-03-09 to 2027-04-30  email    studio_outreach
```

Nothing was written for the two refused edits. The rule you wrote in step 3 is now part of the referee.

## Step 6 — Let an agent work on the graph

Connect Claude or any MCP client to `specforge mcp` ([Connect an agent](connect-an-agent.md)). The agent then:

1. **Learns your vocabulary** with `specforge.schema`: your kinds, fields and links, exactly as your extension
   declares them. You don't explain your domain to it in a prompt; it reads it.
2. **Reads what exists** with `specforge.query`, `specforge.search` and `specforge.list`.
3. **Proposes changes** through `specforge.edit`. An edit that breaks one of your rules is refused with the
   reason, and the agent fixes it and retries. It cannot save a campaign without an audience, whatever it
   was asked.
4. **Answers questions** with your commands (`specforge.marketing.calendar`) and the builtins' (for example
   `specforge.product.health`).

A first request, for a marketer who never opens a `.spec` file:

> Using the SpecForge tools, add a May campaign on social for small studios, reusing the team-seats message.
> Check it against the brand rules and tell me what you changed.

Today this is how non-developers use SpecForge: through an agent. A web interface for business users (AG-UI,
HTTP) is planned, not built.

## What is not built yet

Be clear about these before you plan around them:

- **No live connectors.** An extension is sandboxed: it gets no file system, network or environment, and
  answers only from what each call hands it. SpecForge does not read your CRM, ERP or quality system; an agent
  with its own tools for those systems can bring what it learns into the graph.
- **Evidence is test-centred.** `verify` statements and `specforge collect` record *test results* for software
  entities. Recording other evidence — a signed batch record, a campaign report — as proof is future work.
- **Framework extensions.** GSD is being designed; BMAD, OpenSpec and SpecKit extensions are not started.
- **Business-user interface.** AG-UI and HTTP surfaces are planned.

## See also

- [Extending SpecForge](extending-specforge.md) — the full extension tutorial.
- [SDK reference](../extension-sdk.md) and [Extension Protocol](../extension-protocol.md).
- [Authoring `.spec` files](authoring-spec-files.md) — the language, taught with the builtin kinds.
- [Registry trust](../registry-trust.md) — publishing an extension for others to install.
