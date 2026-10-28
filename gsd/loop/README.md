# `@gsd/loop`

**Status: being designed; nothing is built yet.** A profile over [`@gsd/core`](../core/) for gsd-loop's GitHub lanes. A loop issue is a plan,
its PR's verdict is a review, its labels (`gsd:ready`, `gsd:rework`, `gsd:approved`, `gsd:escalated` …) are computed
states. It adds only what is new: the decision map of `discover`, the `O-N` outcome and `X-N` exclusion contracts of an
issue, the build and review lanes, and the three-strikes escalation rule. GitHub is read and written by collectors and
commands, never by the kernel. Tracks gsd-loop 0.4.1.
