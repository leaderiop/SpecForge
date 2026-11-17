# GSD A/B review record (gate A)

The record of the review that freezes the protocol before any of its runs ([README.md](README.md) section 10). Two
reviewers read `evaluation/gsd-ab/` at one commit and fill one row each per item: the owner, and one independent
agent given only this directory (not the plans or notes that produced it). A finding that is text is fixed in the
commit that fills this record; one that changes a seed, a task or a script is fixed before it, with its test, and
`ab_seeds` and `ab_protocol` pass on the result.

The protocol depends on these paths outside this directory. The independent agent does not read them; the owner
checks that the commit's tests over them pass (`cargo gate full --only gsd`):

- the fixtures, `gsd/parity/fixtures/taskboard`, `vault` and `ledger`, pinned by tree in `tasks.json`;
- the tests, `gsd/core/tests/cli/ab_seeds.rs`, `ab_protocol.rs` and `harness_ab.rs`, and
  `gsd/security/tests/cli/ab_seeds.rs`;
- the components and the binary the arms load: `gsd/core`, `gsd/security` and the `specforge` CLI (`crates/`);
- the tag `gsd-ab-v1`, whose fixture trees, components and binary arm Bv1 runs.

Reviewed commit: `<sha>`

| Reviewer | Who | Date |
|---|---|---|
| owner | | |
| independent agent | `<model id>`, given only `evaluation/gsd-ab/` | |

Verdict: `sound` (nothing to change), `fixed` (findings fixed, named in the findings column), or `blocks` (the tag
waits for it).

## The protocol

| Item | Owner: verdict, findings | Agent: verdict, findings |
|---|---|---|
| 1. Question and hypotheses: H1, H2 and H3 say what is measured, and H2 and H3 gate nothing | | |
| 2. Arms: A, B, B0 and Bv1 frozen in `tasks.json#arms`; the prompts differ only in their commands, and Bv1's is B's | | |
| 3. Fixtures: three, each with its reasons, each arm from the same commit | | |
| 4. Tasks: 12, each with its prompt, seeds, acceptance (`run` or `judge`) and ground truth | | |
| 5. Seeded defects: A and B state the same false fact; B's signal and when it shows | | |
| 6. Measures: M1 to M7 computed from the run file alone; M7 is the context tokens a gsd-executor spawn holds at its first state change: a file tool, a mutating engine tool or a writing command, by MCP or by CLI (`tasks.json#state_writes`); its last request's when it changes none | | |
| 7. Run procedure and [OPERATOR.md](OPERATOR.md): every run file field has one way to read it | | |
| 8. Analysis: pairing, the bootstrap, a fixed RNG seed | | |
| 9. Gate B: the pass rule is decided before any run | | |
| 10. Pre-registration: what the tag freezes (this directory, the fixtures by their pinned trees, the components, the tag Bv1 reads) | | |
| 11. Threats: each named with what holds against it | | |
| `seed.py` and `score.py`: their self-tests, and what they refuse (`spawns`: an unknown transcript layout, a subagent without its type or usage, a command the shell reader cannot split) | | |
| `prompts/A.md`, `B.md`, `B0.md`, `Bv1.md` | | |

## The fixtures

| Fixture | Owner: verdict, findings | Agent: verdict, findings |
|---|---|---|
| taskboard: `.planning/` and its twin say the same; its four tasks' seeds and ground truth | | |
| vault: `.planning/` and its twin say the same; the open threat; its four tasks | | |
| ledger: the red test, `.planning/` still saying passed; its four tasks, each holding the red truth not verified | | |

## Close

Ready to tag `gsd-ab-v3` at `<sha>`.

## Version 3

v3 is reviewed at its own gate A before its tag `gsd-ab-v3` is set (it is not tagged yet). v2 was never tagged, so this
record covers v2's additions too (arm Bv1, M7 and H3). What changed from v2:

- S1 is retired (`tasks.json#retired`), with its reason.
- S5 and S6 are signalled by `gsd/W408` on the deliverable whose check names the failed or deleted test, not by
  `code/W401` and `code/E101` on a truth; where that is not seed-only a task does not seed the defect (`not_seeded`).
  `taskboard-verify-01` seeds both, `ledger-verify-01` S5, `vault-verify-01` neither.
- H2 is re-registered over S5 and S6, the defects marked `"thread": true`: a change of the registered hypothesis's
  defect set (README section 1), so the reviewers read item 1 again.
- The requirement kind's B read finds its truths through the plans that list the requirement.
- The fixtures' preflight baselines are re-recorded.
