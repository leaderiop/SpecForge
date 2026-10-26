# Marketing: teaching SpecForge a new domain

SpecForge ships knowing nothing about marketing. This example teaches it. A small extension,
`@example/marketing`, declares what a marketing plan is made of and which mistakes to refuse.
The project next to it uses that extension to plan a real-looking launch.

You get a launch plan that is checked like code. A campaign with no audience, copy that skipped
the brand check, or a campaign that ends before it starts is reported on every check, and
refused when an AI agent tries to write it.

```
examples/marketing/
  extension/        the extension: one Rust file, and under wasm/ the WebAssembly component
                    built from it, committed so a fresh clone works as is
  specforge.json    the project: it loads extension/wasm/marketing.wasm
  spec/             the plan: brand rules, audiences, messages, campaigns
```

## For a marketer: what the extension teaches SpecForge

Four things a plan is made of:

| Thing | What it is | Example in `spec/` |
|---|---|---|
| `brand_rule` | A rule from the brand book | "Write 'Lumen Pro'. Never 'Lumen PRO'." |
| `audience` | A group of people you speak to | Freelance designers, about 42,000 |
| `message` | Something you say: a headline and its copy | "Lumen Pro edits a full shoot in half the time." |
| `campaign` | A push on one channel, to some audiences, with some messages, between two dates | Launch on social, 2 to 31 March, 15,000 euros |

And how they connect: a campaign **targets** audiences, a campaign **uses** messages, and a
message **obeys** brand rules. SpecForge keeps those links as a graph, so it can answer "which
campaigns reach small studios?" or "which messages were checked against the discount rule?"
without anyone searching a slide deck.

### What the checks catch

| Code | Level | Catches |
|---|---|---|
| E901 | error | A campaign that targets no audience: money spent on nobody |
| E902 | error | A message that cites no brand rule: copy that skipped the brand check |
| E903 | error | A campaign that ends before it starts: a calendar typo |
| W901 | warning | A brand rule no message obeys: a rule nobody checks against |

These codes are this extension's own (third-party extensions number theirs E900 to E998,
W900 to W998), so `specforge explain` does not know them; this table is their documentation.

The plan in `spec/` passes all of them:

```
$ specforge check
0 errors, 0 warnings, 0 infos
```

Here is a plan that breaks three rules at once. It is not in `spec/`; drop it in as
`spec/oops.spec` and add `use "oops"` to `spec/main.spec` to see it fail:

```spec
campaign summer_teaser "Summer teaser" {
  goal    "500 sign-ups to the summer waitlist."
  channel social
  starts  2027-06-30
  ends    2027-06-01
  messages [teaser]
}

message teaser "Something big is coming" {
  headline "Something big is coming to Lumen Pro."
}
```

What `specforge check` says:

```
[E901] Error: campaign 'summer_teaser' targets no audience
   ╭─[ spec/oops.spec:3:1 ]
   │
 3 │ ╭─▶ campaign summer_teaser "Summer teaser" {
   ┆ ┆
 9 │ ├─▶ }
   │ │
   │ ╰───────
───╯

[E902] Error: message 'teaser' cites no brand rule
    ╭─[ spec/oops.spec:11:1 ]
    │
 11 │ ╭─▶ message teaser "Something big is coming" {
    ┆ ┆
 13 │ ├─▶ }
    │ │
    │ ╰───────
────╯

[E903] Error: campaign 'summer_teaser' ends (2027-06-01) before it starts
   ╭─[ spec/oops.spec:3:1 ]
   │
 3 │ ╭─▶ campaign summer_teaser "Summer teaser" {
   ┆ ┆
 9 │ ├─▶ }
   │ │
   │ ╰───────
───╯
3 errors, 0 warnings, 0 infos
  E901 (1)
  E902 (1)
  E903 (1)
```

### The rules hold for every editor, people and agents alike

A typed edit that would break a rule is refused, and nothing is written. Moving the upgrade
campaign's end date before its start:

```
$ specforge set customer_upgrade ends 2027-02-01
error[E080]: the edit would leave the project with 1 error it does not have; nothing was written
  error[E903]: campaign 'customer_upgrade' ends (2027-02-01) before it starts
```

### How an AI agent plans a campaign over MCP

Today a marketer reaches SpecForge through an AI agent (a web or chat interface for business
users is planned, not built). Point an MCP client such as Claude at this folder with
`specforge mcp examples/marketing`, then ask in plain words: "Plan a partner campaign for small
studios in May, reusing the team-seats message."

The agent works through SpecForge's MCP tools, and the extension adds its own:

1. **Read the plan.** `specforge.list` and `specforge.inspect` return the audiences, messages and
   brand rules; `specforge.marketing.calendar` (the extension's command) returns the campaigns in
   date order, so the agent sees what is already running in May.
2. **Write the change.** `specforge.edit` creates the `campaign` with its goal, channel, dates,
   budget, audiences and messages. The change is validated before it is written: a campaign
   with no audience, or dates the wrong way round, is refused with the rule's message, and the
   agent fixes it and tries again.
3. **Confirm.** `specforge.validate` reports 0 errors, and the new campaign is in the graph for
   the next person, or agent, to read.

The agent is the brain; SpecForge holds the plan and enforces the rules. The extension cannot
reach your ad accounts or CRM: extensions are sandboxed and read only the plan, and live
connectors to business systems are not built.

## Technical: how the extension is declared and built

The extension is one file, [`extension/src/lib.rs`](extension/src/lib.rs), written with the
extension SDK ([docs/extension-sdk.md](../../docs/extension-sdk.md)). It was scaffolded with
`specforge extension init --name @example/marketing` and then filled in:

- **Kinds**: `c.kind("campaign", ..)` and three more, each field typed (`FieldType::Date` for
  `starts`/`ends`, a `Vocabulary` for `channel`, `FieldType::ReferenceList` for links).
- **Edges**: a reference field names its edge (`.edge("CampaignTargetsAudience")`), and
  `c.edge(..)` declares each one with its source and target kinds.
- **Rules**: E901, E902 and W901 are declarative (`CheckKind::NoOutgoingEdges` and
  `NoIncomingEdges` on one edge type). E903 is a custom rule: `CheckKind::Custom` with a
  `validate` function that compares the two dates.
- **Command**: `c.command("calendar", ..)` is both `specforge marketing calendar` on the command
  line and the MCP tool `specforge.marketing.calendar`.

Build and check it (needs Rust and the `wasm32-wasip2` target):

```bash
specforge extension build --dir examples/marketing/extension     # cargo build --release --target wasm32-wasip2
specforge extension validate --dir examples/marketing/extension  # loads it and checks the declaration
```

The component is about 350 KB. The project loads it through a `.wasm` file entry in
`specforge.json`, a path relative to the project root:

```json
"extensions": ["@example/marketing=extension/wasm/marketing.wasm"]
```

The name before `=` must be the name the component declares. In this repository the component
is a committed blob, like the builtins: after changing `lib.rs`, run
`cargo run -p xtask --bin revendor`. It rebuilds `extension/wasm/marketing.wasm` reproducibly
and records what it was built from in `extension/wasm/inputs.json`; `cargo gate` fails until you
do. In a project of your own, build with `specforge extension build` and point the entry at
the output under `extension/target/`, or copy it next to the project and commit it so a fresh
clone checks without building. (`specforge add <file>.wasm` is the other way to install a
component: it copies it under `.specforge/extensions/` and records it in `specforge.lock`.)

Inside this repository the crate takes the SDK by path; outside it, depend on the published
`specforge-extension-sdk` at your specforge's version, as `extension init` writes it (a build from after
the last release: [Building against an unreleased SDK](../../docs/extension-sdk.md#building-against-an-unreleased-sdk)).
