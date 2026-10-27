# `@gsd/path`

**Status: being designed; nothing is built yet.** A profile over [`@gsd/core`](../core/) for gsd-path's way of working. It reuses core's kinds
wherever gsd-path shares a concept (its stages are the phase lifecycle) and adds, by `enhance` and its own rules, only
what is new: the program charter and program roadmap above milestones, lookahead (planning the next milestone under
`next/` while one builds), and the abandon-and-reslice ruling (`milestone.abandoned` lives in core so that core can
show it). Tracks gsd-path 1.6.2.
