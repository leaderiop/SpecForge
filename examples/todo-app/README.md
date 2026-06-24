# todo-app — worked SpecForge example

A complete, compiler-validated SpecForge project specifying a small **task / todo
management** application. It is the running example used by the
[zero-to-hero authoring guide](../../docs/guides/authoring-spec-files.md).

It uses four extensions: `@specforge/software`, `@specforge/product`,
`@specforge/governance`, and `@specforge/formal`.

## Try it

```bash
cd examples/todo-app
specforge check                         # 0 errors, 0 warnings
specforge trace task_management         # traceability chain for the feature
specforge query create_task --depth 1   # neighborhood of a behavior
specforge export --format=context       # what an agent consumes
specforge stats                         # entity/edge counts
```

## Layout

Files are organized **by entity kind** (the recommended convention), wired together
with `use` imports.

```
spec/
  main.spec                  # the singleton `spec { }` block
  types/task.spec            # Task, TaskStatus, commands, errors, event payloads
  invariants/task.spec       # guarantees (id uniqueness, completed→timestamp)
  ports/task_repository.spec # the persistence boundary (outbound port)
  events/task.spec           # task_created, task_completed
  behaviors/task.spec        # create_task, complete_task, list_tasks
  features/task_management.spec
  product/
    personas.spec            # individual
    channels.spec            # cli
    journeys.spec            # capture_and_complete
    planning.spec            # module, milestone, deliverable
    glossary.spec            # term: task, done
  governance/decisions.spec  # decision, constraint, failure_mode
  formal/properties.spec     # a safety property
```

## The graph it produces

30 entities and 35 edges. The feature `task_management` is the hub: behaviors point
*up* to it (`features [...]`), and journeys, modules, milestones, and personas all
reference it — so `specforge trace task_management` shows the full chain from a
user-facing capability down to the invariants that protect it.
