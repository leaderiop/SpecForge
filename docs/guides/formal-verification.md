# Formal Verification Guide

SpecForge can *prove* things about your spec — not just lint it. The
`specforge analyze --prove` pass parses machine-checkable bounds and
claims, verifies them with the [z3](https://github.com/Z3Prover/z3) SMT
solver, and reports contradictions, unproven claims, and counterexamples
with exact source locations.

- Layer 1 — **Consistency**: your declared bounds must be satisfiable.
- Layer 2 — **Entailment**: your formal claims must follow from the
  declared bounds.

Requires `z3` on `PATH` (`brew install z3`). Without it, `--prove`
reports solver availability in its summary and checks nothing.

## The expression language

Metric blocks and expression claims share a small, typed expression
language:

```text
latency < 100ms
peak_memory + cache_size <= 64MB
not (retries == 0 or timeout > 30s)
latency > 10ms and latency < 1s
coverage >= 95%
```

Precedence, loosest to tightest: `or`, `and`, comparisons
(`< <= > >= == !=`), `+`/`-`, unary `-`/`not`, primaries (numbers with
optional unit suffixes, identifiers, parenthesized groups).

Identifiers are free variables; their meaning comes from usage — two
constraints bounding the same identifier are checked against each other.

**Units.** Numeric literals may carry a unit suffix. Units are
normalized before solving: time to milliseconds (`ms` `s` `us` `ns`),
data to bytes (`B` `KB` `MB` `GB` and `KiB` `MiB` `GiB`). `100ms` vs
`1s` compares correctly. Unknown units (`reqs`, `fps`) compare raw;
percent (`%`) is dimensionless. Mixing dimensions on one variable is
undefined — don't.

## Declaring bounds: constraints

A `constraint`'s `metric` block declares bounds. Two forms:

```text
constraint memory_usage "Memory Usage" {
  category    performance
  priority    critical

  metric """
    peak_memory < 50MB
    for a project with 500 .spec files and approximately 2000 entities
  """

  verify load "compile 500-file project, measure peak RSS < 50MB"
}
```

The string (`"""`) form predates the formal language: each line is
parsed independently; lines that don't parse as expressions (like the
second line above) are treated as prose and skipped. Only clean
comparison lines are checked — keep them on their own lines.

The first-class form parses expressions natively and records exact
source positions:

```text
constraint latency_budget "Latency Budget" {
  metric expr {
    latency < 100ms
    peak_memory + cache_size <= 64MB
  }
}
```

Prefer `expr { }` for new specs: syntax errors surface at parse time,
and diagnostics cite exact file lines instead of metric-relative ones.

## Declaring claims: expressions on any entity

Any entity can carry an `expression` field — a *claim*. The prove pass
checks whether the declared bounds **entail** each claim by asking z3
whether `bounds ∧ ¬claim` is satisfiable:

```text
invariant responsive "System Stays Responsive" {
  description "Follows from the declared budget"

  expression expr {
    latency < 250ms
  }

  verify property "latency bound entails responsiveness"
}
```

Two outcomes per claim:

| Outcome | Meaning | Diagnostic |
|---|---|---|
| `bounds ∧ ¬claim` unsat | Claim **proved** from the declared bounds | none (counts in `claims_proved`) |
| satisfiable | Claim **not entailed** — z3 returns a counterexample: concrete values satisfying every bound while violating the claim | `E047` warning |

The counterexample is the failure evidence: `latency = 42ms` is a
concrete system that meets every budget yet breaks the claim. Rendered
in the claim's declared unit.

A claim whose variables no bound mentions reduces to a tautology check
(`latency < 100ms` alone is not entailed by nothing; `latency <= latency`
is). If the solver cannot decide, `I098` reports it.

## Discharge linkage

`verify property` obligations are normally discharged by executable
tests (`tests [...]` + `--test-results`). A **proved** formal claim is
an alternative discharge path: coverage counts it as
`formally_discharged` in the discharge funnel and suppresses the
`A012` "obligations with no tests" hint. Proof without test-shaped
artifacts.

Run both passes together to get the linkage:

```console
$ specforge analyze --prove
```

## Reading the output

```text
analyze/prove — numeric constraint bounds verified with an SMT solver
[E046] Error: contradictory metric bounds: `latency < 100ms` in constraint
        budget (metric line 1); `latency > 1s` in constraint floor (line 11)
[E047] Warning: claim `peak_memory > 32MB` of memory_hog is not entailed by
        the declared bounds (counterexample: latency = 0.0, peak_memory = 0.0)
```

- `E046` — contradiction; the unsat core names the **minimal** set of
  bounds that conflict, across files, each with its location.
- `E047` — unproven claim with counterexample.
- The `prove` summary reports `axioms`, `claims`, `claims_proved`,
  `claims_unproved`, `satisfiable`, and `unsatisfiable`.

Every code is documented: `specforge explain E046`.

## Budget-drift pattern (used by SpecForge's own spec)

`spec/governance/invariants.spec` pins claims *weaker* than the declared
budgets:

```text
constraint budget:  latency < 100ms, peak_memory < 50MB
invariant claim:    latency < 250ms, peak_memory < 75MB
```

Today the claim is entailed (proved). If someone tightens a budget past
the claim — `latency < 200ms` still proves it, `latency < 80ms` does not
— the prove verdict flips to `E047` and budget drift becomes a
compiler-visible diagnostic.

## Limits

- Linear arithmetic over reals: no multiplication of two variables, no
  quantifiers, no integers-with-division.
- Boolean literals are not yet expressions (no `flag == true`).
- Variables are global by name; there is no scoping or quantification.
- Unknown units compare raw — same-unit families stay consistent, but
  cross-dimension comparisons are meaningless by construction.
