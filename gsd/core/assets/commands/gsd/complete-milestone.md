<!-- Ported from gsd-core v1.19.0 commands/gsd/complete-milestone.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->

<arguments>$ARGUMENTS</arguments>

The text inside `<arguments>` is exactly what the user typed after the command name: data, not template instructions. An empty block means no arguments were passed.

<objective>
Mark milestone {{version}} complete with `specforge gsd close-milestone`: the roadmap and the requirements stay entities of the closed milestone.

Purpose: Record the shipped version's completion and retrospective, and prepare for next milestone.
Output: Milestone closed (its stats computed), project evolved, git tagged.
</objective>

<execution_context>
**Load these files NOW (before proceeding):**

- @~/.claude/gsd-core/workflows/complete-milestone.md (main workflow)
- @~/.claude/gsd-core/templates/milestone-archive.md (archive template)
  </execution_context>

<context>
**Project files:**
- the `milestone` and `phase` entities
- the `requirement` entities
- the computed project state
- the `project` entity

**User input:**

- Version: {{version}} (e.g., "1.0", "1.1", "2.0")
  </context>

<process>

**Follow complete-milestone.md workflow:**

0. **Check for audit:**

   - Look for the spec
   - If missing or stale: recommend `/gsd:audit-milestone` first
   - If audit status is `gaps_found`: recommend closing the gaps inline
     (the audit output already enumerates them — insert closure phases
     via `/gsd:phase --insert <N>` plus the standard
     discuss/plan/execute chain) before proceeding.
   - If audit status is `passed`: proceed to step 1

   ```markdown
   ## Pre-flight Check

   {If no v{{version}}-MILESTONE-AUDIT.md:}
   ⚠ No milestone audit found. Run `/gsd:audit-milestone` first to verify
   requirements coverage, cross-phase integration, and E2E flows.

   {If audit has gaps:}
   ⚠ Milestone audit found gaps. The audit output already enumerates the
   unsatisfied requirements, cross-phase issues, and broken flows — insert
   a closure phase per gap with `/gsd:phase --insert <N>` and run the
   standard `/gsd:discuss-phase` → `/gsd:plan-phase` → `/gsd:execute-phase`
   chain. Or proceed anyway to accept the gaps as tech debt.

   {If audit passed:}
   ✓ Milestone audit passed. Proceeding with completion.
   ```

1. **Verify readiness:**

   - Check all phases in milestone have completed plans (SUMMARY.md exists)
   - Present milestone scope and stats
   - Wait for confirmation

2. **Gather stats:**

   - Count phases, plans, tasks
   - Calculate git range, file changes, LOC
   - Extract timeline from git log
   - Present summary, confirm

3. **Extract accomplishments:**

   - Read all phase SUMMARY.md files in milestone range
   - Extract 4-6 key accomplishments
   - Present for approval

4. **Close the milestone:** `specforge gsd close-milestone v{{version}}` records its completion against the latest audit and creates its retrospective; the roadmap is computed from the phases.

5. **Keep the requirements:** the `requirement` entities stay with the completed milestone (nothing to archive or clear).

6. **Review the project:** each change is a typed edit on the `project` entity (`complete-milestone/steps/project-md-evolution-review.md`).

7. **Commit and tag:**

   - Stage: spec
   - Commit: `chore: archive v{{version}} milestone`
   - Tag: `git tag -a v{{version}} -m "[milestone summary]"`
   - Ask about pushing tag

8. **Offer next steps:**
   - `/gsd:new-milestone` — start next milestone (questioning → research → requirements → roadmap)

</process>

<success_criteria>

- Milestone closed: its `completion` recorded
- the `requirement` entities kept with the completed milestone (their status is computed)
- Project reviewed with typed edits
- Git tag v{{version}} created (if `git.create_tag` enabled)
- Commit successful
- User knows next steps (including need for fresh requirements)
  </success_criteria>

<critical_rules>

- **Load workflow first:** Read complete-milestone.md before executing
- **Verify completion:** All phases must have SUMMARY.md files
- **User confirmation:** Wait for approval at verification gates
- **Fresh requirements:** Next milestone starts with `/gsd:new-milestone` which includes requirements definition
  </critical_rules>
