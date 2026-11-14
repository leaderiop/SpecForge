<!-- Ported from gsd-core v1.19.0 gsd-core/references/skeleton-template.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Walking Skeleton Template

> This is what `gsd-planner` works from once `WALKING_SKELETON=true`, which happens on Phase 1 of a brand-new project planned under `--mvp`. The Walking Skeleton is the Phase-1 tracer across the whole application. Each row of its Architectural Decisions table is recorded as a project `decision` (`specforge schema --kind decision`), the capability it proves is the first plan's objective, and its later slices are the roadmap's phases; the outline below is what the planner covers.

```bash
PROJECT="{the project's id, e.g. taskboard}"
DECISION_ID="{D- and the next free number of the project's decisions, e.g. D-03}"
IFS= read -rd '' CHOICE <<'TEXT' || true; CHOICE=${CHOICE%$'\n'}
{the choice, e.g. Postgres with Drizzle for the data layer}
TEXT
IFS= read -rd '' WHY <<'TEXT' || true; WHY=${WHY%$'\n'}
{why it fits the project}
TEXT
FILE=$(specforge query "$PROJECT" --depth 0 --pick '.nodes[0].file') || exit 1
[ -n "$FILE" ] || { echo "no project $PROJECT in the graph" >&2; exit 1; }
specforge new decision "$DECISION_ID" "$CHOICE" --in "$FILE" --origin project --area "Walking Skeleton" --text "$CHOICE" --rationale "$WHY" || exit 1
```

```markdown
# Walking Skeleton — [Project Name]

**Phase:** 1
**Generated:** {ISO date}

## Capability Proven End-to-End

> One sentence: the smallest user-visible capability that exercises the full stack.

Example: "A signed-in user can view their email on a dashboard page served by the deployed app."

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Framework | (e.g., Next.js 15 App Router) | Why this fits the project |
| Data layer | (e.g., Postgres + Drizzle) | Why |
| Auth | (e.g., session cookies + bcrypt) | Why |
| Deployment target | (e.g., Vercel preview) | Why |
| Directory layout | (e.g., feature-folders under src/features/*) | Why |

## Stack Touched in Phase 1

- [ ] Project scaffold (framework, build, lint, test runner)
- [ ] Routing — at least one real route
- [ ] Database — at least one real read AND one real write
- [ ] UI — at least one interactive element wired to the API
- [ ] Deployment — running on dev environment OR documented local full-stack run command

## Out of Scope (Deferred to Later Slices)

> Anything that is *not* in the skeleton. Be explicit — this list prevents future phases from re-litigating Phase 1's minimalism.

- (e.g., password reset, email verification, multi-tenancy)

## Subsequent Slice Plan

Each later phase adds one vertical slice on top of this skeleton without altering its architectural decisions:

- Phase 2: [next user capability]
- Phase 3: [next user capability]
- ...
```
