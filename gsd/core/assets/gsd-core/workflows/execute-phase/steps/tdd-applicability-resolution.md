<!-- Ported from gsd-core v1.19.0 gsd-core/workflows/execute-phase/steps/tdd-applicability-resolution.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# TDD-applicability resolution (#4266/#4272)

Run for each plan, immediately after executor routing and before composing
that plan's `Agent()` prompt in step 3. Resolves whether this dispatch is TDD
— fail closed, do not guess.

## Resolution

```bash
# the commands below are specforge commands: there is no launcher to define
TDD_APPLICABLE_RAW=$(specforge query "$PLAN_ID" --depth 0 --format json | jq -r '.nodes[0].fields.type == "tdd"' 2>/dev/null)
TDD_APPLICABLE_RC=$?
if [ $TDD_APPLICABLE_RC -ne 0 ]; then
  echo "FATAL: could not resolve TDD-applicability for plan {plan_number} — 'phase.tdd-applicable' failed. Refusing to guess whether this dispatch needs the TDD procedure. Halting." >&2
  exit 1
fi
TDD_APPLICABLE="$TDD_APPLICABLE_RAW"
```

## Pre-dispatch check (MANDATORY)

Before calling Agent(), confirm every `${...}` conditional in the prompt below
(`TDD_APPLICABLE`, `CONTEXT_WINDOW`, `AGENT_SKILLS`) was resolved to concrete
text for THIS plan. If any marker's value was not computed, HALT — do not
dispatch a prompt containing literal `${...}` template syntax (#4266).
