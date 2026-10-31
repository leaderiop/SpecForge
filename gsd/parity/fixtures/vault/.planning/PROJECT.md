# Vault

## What This Is

A small encrypted store for secrets, driven from the command line. A user sets a named secret, reads it back and lists
the names. It is a Node 20 script with no dependencies, built to prove the GSD security model in SpecForge.

## Core Value

A secret never touches the disk in the clear.

## Requirements

### Active

- [ ] A user can store and read back a secret by name
- [ ] A user can share a secret with another person

## Context

Built to prove the GSD security capability in SpecForge. Small enough that one person reads all of it.

## Constraints

- **Tech stack**: Node 20, no dependencies — it ships as one script

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| AES-256-GCM from `node:crypto` | the auth tag detects a tampered file | — Pending |

---
*Last updated: 2026-10-01 after initialization*
