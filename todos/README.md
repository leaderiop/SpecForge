# Todos

One file per todo, `NNNN-slug.md`. The front matter is the tracking record; the body holds the details.

```yaml
id: 0001            # file number, never reused
title: ...
status: open        # open | in-progress | blocked | done | dropped
trigger: ...        # what makes it worth doing now (or "none")
created: 2026-11-18
updated: 2026-11-18
links: [...]        # ADRs, PRs, artifacts
```

Inside a todo, work items are `- [ ] T1 ...` and open questions are `- [ ] Q1 ...`. Tick them as they close.

List what is open:

```sh
grep -l '^status: \(open\|in-progress\|blocked\)' todos/*.md
```

Count unfinished items per file:

```sh
grep -c '^- \[ \]' todos/*.md
```

## Index

| id | title | status | trigger |
|---|---|---|---|
| [0001](0001-smt-pack-replaces-host-prove.md) | z3 becomes a collected-proof pack; the host's prove step and `@specforge/formal` go | open | `truth.expression` is built |
