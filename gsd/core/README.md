# `@gsd/core`

**Status: being designed; nothing is built yet.** The unified GSD meta-model: project, requirements, milestones, phases, plans, tasks,
decisions, research, results, verification, UAT and validation, with GSD's checks as rules and its commands, agents,
prompts and hooks as contributions. Tracks gsd-core 1.18.0.

| Folder | What |
|---|---|
| [spec/](spec/) | the model: kinds, fields, identity, settings, rules |
| [contributions/](contributions/) | the command, agent and hook rosters |
| `src/` (later) | the Rust extension crate, built to `wasm32-wasip2` |
| `tests/` (later) | rule tests and golden graphs |
| `wasm/` (later) | the built component |

It declares no dependency on any `@specforge/*` extension and on no other `@gsd/*` extension; the capability
extensions and the profiles declare it as their peer.
