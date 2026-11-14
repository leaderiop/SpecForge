# GSD A/B review record (gate A)

The record of the review that freezes version 2 of the protocol before any of its runs ([README.md](README.md) section 10). Two
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
| 1. Question and hypotheses: H1 and H2 say what is measured, and H2 gates nothing | | |
| 2. Arms: A, B, B0 and Bv1 frozen in `tasks.json#arms`; the prompts differ only in their commands, and Bv1's is B's | | |
| 3. Fixtures: three, each with its reasons, each arm from the same commit | | |
| 4. Tasks: 12, each with its prompt, seeds, acceptance (`run` or `judge`) and ground truth | | |
| 5. Seeded defects: A and B state the same false fact; B's signal and when it shows | | |
| 6. Measures: M1 to M6 computed from the run file alone, fixed for v1 (autonomy measures are v2's) | | |
| 7. Run procedure and [OPERATOR.md](OPERATOR.md): every run file field has one way to read it | | |
| 8. Analysis: pairing, the bootstrap, a fixed RNG seed | | |
| 9. Gate B: the pass rule is decided before any run | | |
| 10. Pre-registration: what the tag freezes (this directory, the fixtures by their pinned trees, the components, the tag Bv1 reads) | | |
| 11. Threats: each named with what holds against it | | |
| `seed.py` and `score.py`: their self-tests, and what they refuse | | |
| `prompts/A.md`, `B.md`, `B0.md`, `Bv1.md` | | |

## The fixtures

| Fixture | Owner: verdict, findings | Agent: verdict, findings |
|---|---|---|
| taskboard: `.planning/` and its twin say the same; its four tasks' seeds and ground truth | | |
| vault: `.planning/` and its twin say the same; the open threat; its four tasks | | |
| ledger: the red test, `.planning/` still saying passed; its four tasks, each holding the red truth not verified | | |

## Close

Ready to tag `gsd-ab-v2` at `<sha>`.
