# spec-driven-method: a spec-driven workflow as a graph

Spec-driven methods such as GSD or SpecKit plan software in layers: a roadmap of milestones, phases with
success criteria, requirements, and a verification step before anything counts as done. They usually
keep those layers in Markdown files. This project keeps them in one checked graph, using only the
builtin extensions, and applies the method to a small link shortener.

Extensions: `@specforge/software`, `@specforge/testing`, `@specforge/product` and `@specforge/governance`.

## The mapping

| Method layer | Kind | File |
|---|---|---|
| milestone | `milestone` | `spec/product/roadmap.spec` |
| phase, with goal and success criteria | `feature` (`problem`, `solution`, `acceptance`) | `spec/product/phases.spec` |
| what a milestone ships | `deliverable` | `spec/product/roadmap.spec` |
| who it serves, end to end | `persona`, `channel`, `journey` | `spec/product/users.spec` |
| requirement | `behavior` (`contract`) | `spec/behaviors/links.spec` |
| must-hold truth | `invariant` (`guarantee`) | `spec/invariants/links.spec` |
| verification | `verify unit`, `verify property`, `verify integration` | on behaviors and invariants |
| the method's words | `term` | `spec/product/glossary.spec` |

A dedicated GSD extension family, with GSD's own kinds (phase, plan, task, UAT), is being designed;
it is not built. This example shows how far the builtins already go.

## Try it

Use a `specforge` built from this repository (`cargo build --release -p specforge-cli`, then
`target/release/specforge`).

```bash
cd examples/spec-driven-method
specforge check                 # 0 errors, 0 warnings, 0 infos
specforge stats                 # 19 entities, 32 edges
specforge trace phase_redirect  # the requirements, milestone, journey and later phase tied to it
specforge analyze coverage      # 7 verify obligations, none proven yet
```

`analyze coverage` counts the obligations a test still has to prove. There is no code here, so none
are proven. In a real project, the tests name the obligation they prove and `specforge collect`
records the results (see [`../todo-app/`](../todo-app/)).

## See the referee at work

Point a requirement at a phase that does not exist:

```bash
specforge set resolve_code features phase_search --dry-run
# error[E080]: the edit would leave the project with 1 error it does not have; nothing was written
#   error[E003]: unresolved reference 'phase_search' in entity 'resolve_code'
```
