# regulated-process: releasing a batch of medicine

A pharmaceutical plant may only send a batch of medicine to market after a Qualified Person (QP) has
certified it. This project writes that procedure down as a checked graph: the steps, the rules no step
may break, the limits the lab works to, the batch lifecycle as a state machine, the decisions behind
the procedure and what can go wrong.

**This is an illustration.** It is not a validated procedure. It does not make anyone compliant with
GMP, EU Annex 16, 21 CFR Part 11 or any other rule. It shows how SpecForge can hold this kind of knowledge.

Extensions: `@specforge/software`, `@specforge/governance` and `@specforge/formal`. Software is loaded
for its generic kinds: `invariant` (a rule that must always hold), `behavior` (a step with a contract)
and `event` (a record a step leaves behind). Nothing here is code.

## Try it

Use a `specforge` built from this repository (`cargo build --release -p specforge-cli`, then
`target/release/specforge`).

```bash
cd examples/regulated-process
specforge check                 # 0 errors, 0 warnings, 1 info
specforge analyze --prove       # needs z3 on PATH: 3 claims, 3 proved
specforge trace certify_batch
```

## See the referee at work

Nobody can delete a rule that the procedure still relies on:

```bash
specforge delete no_release_without_certification --dry-run
# error[E080]: the edit would leave the project with 4 errors it does not have; nothing was written
```

The solver also watches the limits. The lab releases at an assay of at least 98% (`internal_release_limit`);
the registered limit is 95% (`within_registered_specification`). `specforge analyze --prove` proves the
first implies the second. Lower the internal limit in `spec/rules.spec` to `94.0` and the proof fails
with W139 and a counterexample (`assay_purity = 94.0`).

## What is in it

| File | Kinds | What it says |
|---|---|---|
| `spec/rules.spec` | `invariant`, `constraint`, `property` | no release without certification or with an open deviation; the internal and registered limits; certification always comes before shipment |
| `spec/procedure.spec` | `behavior`, `event` | review the batch record, close deviations, certify or reject, ship; each step's `requires` and `ensures` and the record it produces, with its deadline |
| `spec/lifecycle.spec` | `process`, `protocol` | the batch lifecycle from quarantine to released or rejected; the QA-to-QP handover |
| `spec/governance.spec` | `decision`, `failure_mode` | why the internal limit is tighter; why reviewer and QP are two people; shipping from quarantine and missed deviations as failure modes |

## What it does not do

The builtin `verify` obligations are about tests (`unit`, `property`, `integration`), and `collect`
runs test runners. A notion of evidence for a regulated process (a signed record, a training file, an
audit trail entry) is not built yet, so this example declares no `verify` lines. SpecForge does not
read from a quality system, an ERP or a LIMS either: the graph holds what is written in the `.spec` files.
