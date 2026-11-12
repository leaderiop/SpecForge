# Ledger

## What This Is

A ledger of money on the command line: a user adds an entry in integer cents with a memo, and asks for the total.
It is a Rust binary with no dependencies, built to give the GSD A/B a project whose CI is red.

## Core Value

A total is exact to the cent.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] A user can add an entry in integer cents
- [ ] The total is exact
- [ ] Entries survive a restart
- [ ] A user can list the entries in the order they were added

### Out of Scope

- Currencies other than one — a ledger is one currency

## Context

Built for the GSD A/B. Small enough that one person reads all of it.

## Constraints

- **Tech stack**: Rust 2021, the standard library only

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Amounts are integer cents | No float rounding in a total | — Pending |
| One entry per line, tab-separated | The store reads back with `split_once` | — Pending |

---
*Last updated: 2026-11-02 after initialization*
