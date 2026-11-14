<!-- Ported from gsd-core v1.19.0 gsd-core/templates/milestone-archive.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Milestone Archive Template

SpecForge never archives a milestone: it stays a `milestone` entity (`specforge gsd close-milestone` closes it). Its phases, plans, requirements and decisions stay entities of it, so the archive this template described is a read, never a file: `specforge query <milestone> --depth 3 --format context` prints the milestone with its phases and their details, and `specforge schema --kind milestone` lists the fields a closed milestone holds (its `completion`, and the computed `state`, `shipped` and `stats`).
