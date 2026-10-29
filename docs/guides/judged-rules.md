# Judged rules

A judged rule holds while a **fresh verdict** says so (ADR 0290). Some checks need judgement, not a computation: does this
plan cover its requirements, does this code satisfy this truth. An extension declares such a rule with the inputs the
judge reads; an agent (the rule's judge) reads those inputs, decides, and records a `verdict` on the entity. When an
input changes, the verdict goes **stale** and the rule fires again.

## The verdict statement

A verdict is a member of the entity's block, beside its fields and `verify` statements. It is written by `set_verdict`,
not by hand:

```
plan 03-02 "Session tokens" {
  phase 03
  verdict gsd/W311 passed {
    judge "gsd-plan-checker@0.1.0"
    model "claude-sonnet-5-5"
    at "2026-10-28"
    cited [AUTH-07, "docs/design.md"]
    inputs {
      plan "sha256:..."
      decisions "sha256:..."
      cited "sha256:..."
    }
    findings [{ severity warning, message "no test for token expiry" }]
  }
}
```

`verdict <rule> <outcome> { ... }` names the rule (qualified code, or the local code when unique) and one of its
outcomes. `inputs` holds the digest of each input the judge read, `cited` always last; `cited` and `cited_tests` list
what the judge looked at besides. Every verdict also has a line in `specforge-harness`
(key `verdict:<rule>`): a verdict with no line was written by hand, which is **E083**, and the rule counts it as not
recorded. A field that a rule binds to the verdict (`writer: "verdict"`) is set only by a fresh verdict; writing it by
hand is **E082**.

## Where a rule stands

| State | Means |
|---|---|
| `fresh` | a verdict is recorded and every input digests as it did; the rule holds when its outcome is passing |
| `stale` | a verdict is recorded and an input changed (`changed` names it) |
| `missing` | the entity is judged and has no verdict |
| `unrecorded` | a verdict is written and the ledger has no line for it |
| `unreadable` | a verdict cannot be read (**W168**) |

A stale or missing verdict never counts as a pass. A rule gated off by its setting is not evaluated, not listed and
not asked for. **I205** says once per run that test inputs were not judged because no test report exists; **E045** is an
unusable report.

## Writing around a judged rule

A judged rule's finding is never an error a write adds (ADR 0297): E080 does not count it. Creating an entity a rule
targets (a new truth, a phase spec), editing an input of a passing verdict (it becomes `stale`) and recording a failing
verdict are all written, and the finding is reported by the write and by `check`, which still fails on it. Recording
`failed` is the honest record when the judge disagrees. A rule that guards a contradiction between a pass and other
evidence (gsd/E502: a `verified` verdict over tests that failed) is a pass, not a judged rule: it refuses the write.
A writing collector's plan is judged with the report its collect wrote, so a failing test does not refuse its commits. The same applies to its `set_verdict`: a `basis` that the report changed
(the entity's tests, say) is refused as stale, not accepted and stale on the next compile.

## Inputs

An input is a name and a selector over the project:

| Selector | Picks |
|---|---|
| `self` | the entity |
| `kind` or `kind\|other` | every entity of the kind(s) |
| `self.phase` | the entities the field references (a hop; more hops follow: `self.plan.phase`) |
| `self.phase<decision.phase` | the entities of a kind that reference the entity through a field (`<kind.field`, one or more kinds) |
| `file(self.files_modified)` / `file("path", ...)` | the files the field (or the literal paths) name, from the project root |
| `test(self)` | the (name, status) of the tests that verify the entity |

Entities digest by their fields, obligations and verdicts, never by position; a rename of an entity or of a scope above
it stales the verdicts that read it, a respelling does not. Files digest by bytes (a `.spec` source of the project by
the entities it declares). Test inputs compare name and status only, and are skipped when there is no report.

## Reading the inputs, recording the verdict

```
specforge query v1.0/03/03-02 --inputs gsd/W311 [--cite AUTH-07] [--cite-file docs/design.md] [--cite-test name]
```

prints the answer as JSON (`--format graph`, the default), or with `--format brief` each input (members, digest), where the verdict stands and the `basis`; MCP's `specforge.verdict_inputs` answers
the same. A command then answers a `set_verdict` op with `basis` set to what it read. If any input changed since,
the plan is refused `stale_revision` with the inputs that moved, and nothing is written. `inspect`, `query --format
json`, the graph export and a command's graph list each entity's `verdicts` and `judgements`. The editor's hover lists
the verdicts with their state and a code lens says whether each is fresh, failed or stale.

## History

`re_verification` compares statuses at the baseline: the judged rules are evaluated at the baseline commit over that
commit's own files (and ledger), so a verdict stale at the commit is not "passing there".

## Declaring one (SDK)

```rust
c.rule("<code>", |r| {
    r.severity(ValidationSeverity::Warning)
        .message_template("the plan check of {id} does not hold");
    r.judged("gsd-plan-checker", |j| {
        j.outcomes(&["passed", "issues"]).passing(&["passed"]);
        j.target("plan", |t| {
            t.input("plan", "self").input("decisions", "self.phase<decision.phase");
        });
    });
});
```

## Who judges

The judge is a word naming an agent of the rule's extension or of a declared peer, or `prompt:<id>`, a prompt of the
rule's own extension (ADR 0295): for a judgement an agent does inline, in the session that follows a workflow, with
nothing to spawn. A prompt of a peer is not a judge, and a missing one is W112. The verdict reads `judge
"prompt:spec_phase@1.0.0"`, `set_verdict` takes it from a command of the same extension as for an agent, and a rendered
prompt lists the rules it judges under `judged` in its payload, with their outcomes and passing outcomes.

```rust
c.rule("<code>", |r| {
    r.judged("prompt:spec_phase", |j| { j.outcomes(&["clear", "vague"]).passing(&["clear"]); /* targets as above */ });
});
```
