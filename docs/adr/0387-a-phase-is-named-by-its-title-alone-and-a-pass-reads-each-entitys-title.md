# A phase is named by its title alone, and a pass reads each entity's title

**Status:** accepted (2026-11-22). **Amends** [ADR 0019](0019-one-entity-snapshot-per-compile.md) (an entity's
record, and the pass entity read from it, carry its title) and
[ADR 0372](0372-the-converter-gives-every-old-kind-one-fate-and-writes-gsds-ids.md) D4 (a release's title is its
milestone's title, and a listed milestone's title, less a leading `Phase N:`, is its phase's title; the converter
writes no `name`). Protocol 2.3.0 is unreleased: `title` goes into its row, no bump. The wire changes, so every
builtin is re-vendored.

## Context

- `@gsd/core`'s `phase` and `milestone` declared a required `name` beside the title every block already has
  (ADR 0148). GSD's `### Phase N: Name` heading has one name, so the two copies said the same thing, and nothing held
  them equal: a phase could be titled one way and named another.
- They drifted. The converter that moved this repository's own spec to `@gsd/core` wrote `name` without the old
  milestone's `Phase N:` but kept the title as written, so thirteen phases were titled `Phase 12: Graph Protocol Schema
  Versioning` under the id `v1.0.0/03`. The live plan-phase run's agents called the phase "Phase 12" while every
  command said `03`. The titles were fixed and the converter changed to title a phase by its name, but `phase --edit
  --name` could still move one copy without the other, and no rule saw a numbered title.
- The workflows print a phase's label as `Phase {number}: {name}`. The number is the last segment of the phase's id;
  a title that carries a number of its own prints it twice, and a renumber (`phase --remove` renumbers the later ids,
  never their titles) makes the two disagree.
- A rule on the title could not be written: a pass is sent each entity's id, kind and fields, never its title. A
  command reads `GraphNode::title`; a pass had nothing to read.

## D1. A pass entity carries its title

The entity snapshot's record holds the title its header writes, and the pass entity read from it carries it as `title`
(`PassEntity::title`), as a command's `GraphNode::title` does: absent for a fact written without one. It is sent to
every pass; the field text, the values and the edge counts are unchanged. Protocol 2.3.0's row gains the item. On the
wire the key is absent, never `null`, when the entity has no title, and a guest reads an absent key as `None`.

## D2. `@gsd/core`: the title is a phase's and a milestone's one name

- `phase` and `milestone` declare no `name`. A phase's label is `Phase {its id's last segment}: {its title}`, and a
  milestone's name is its title: `progress-report`'s `milestone_name` and the statusline read it.
- A phase keeps `slug`, the directory and branch word GSD derives from the name: `phase` writes `slug` from the title
  it gives, on `--add`, `--insert` and `--backlog` as on `--edit --name`, and the computed `branch` falls back to the
  title's slug, a milestone's to its title's.
- `phase --edit --name X` retitles the phase `X` and rewrites its slug. The argument keeps GSD's name: what the user
  calls the phase's name is its title. `phase` refuses, on every form that names a phase, a name of more than one line
  (a carriage return as a line feed) and a name with no ASCII letter or digit, whose slug would be empty; the line it
  prints names the phase as it is stored, trimmed.
- The workflows read a label's name from the phase node's `title` (`specforge query "$PHASE_ID" --depth 0 --pick
  '.nodes[0].title'`); the roadmap template writes no `name`. A workflow that reads `progress-report` and prints a
  label with a name says, in a definition, that the name is the phase's `title`, and never reads `phases[].name`.
- `progress-report`'s phase `name` stays the slug read as words, as GSD reads its directory name: the parity
  golden of `progress json` holds it. It is the one place the slug stands for the name: every line a person reads
  that the extension prints itself names the phase by its title (the statusline, the session's progress line,
  `progress-report`'s text form).
- No rule ties the slug to the title. The slug is an identifier, not a second name: it names a directory and a branch,
  which outlive a retitle (a branch pushed, a worktree on disk), and a project may choose a short one (the roadmap
  template writes it apart from the title). `phase --edit --name` moves both because it is the one writer that knows
  the person renamed the phase; a title edited by hand keeps its slug, and only `progress json`'s parity `name`, GSD's
  directory name, then reads otherwise than the title. A rule would warn on every deliberate short slug, or push a
  retitle to move a branch; a computed slug would move it silently.

## D3. `gsd/W109`: a phase title embeds a number

A phase whose title begins with a phase number is `gsd/W109`, a warning under gsd-compat and an error under
gsd-strict and gsd-plus. The rule is deliberately narrow: it reads the forms a phase label takes and nothing wider, so
it never fires on a title that only holds a number.

- Flagged: `Phase` (any case) and a number in GSD's grammar (`\d+[A-Z]?(\.\d+)*`: `3`, `03.1`, `12A`) followed by no
  letter or digit (`Phase 12: Core`, `phase 03.1 - Core`, `Phase 3:Core`, `Phase 4`); a bare number of digits and dots
  followed by `:` or `.` and a space (or the end), `)`, or a dash between spaces (`03: Core`, `3. Core`, `2) Core`,
  `04 - Core`).
- Not flagged: a number inside the title (`HTTP 2 support`, `2FA login`, `v2.0 migration`, `3:00 standup`); a number
  joined to a word with no space (`03-Core`, `3-way merge`, `03:Core`, `3.Core`); a bare number before a word (`3
  Core`, `#3 Core`, `(3) Core`); another word before the number (`Step 3: Core`, `Phase-3 cleanup`); a lower-case
  letter after the number (`Phase 3a rollout`). `03-Core` reads like GSD's directory name, but `3-way merge` and
  `24-hour retention` share its form, so it is not clearly the same drift and stays out.

Every phase is read, a backlog phase too: it prints the same label. The suggestion names the title without its number.
The converter drops exactly the flagged number (its copy of the grammar is held to this one by a test), so a converted
project is never `gsd/W109`.

## Rejected

- **Keep both and add a rule that they are equal.** Two sources and a check that they agree is still two sources;
  every writer would have to set both, and a person editing one would be told to edit the other.
- **Derive the label's name from the slug.** The slug is lossy (case, punctuation, `&`), and it is a directory word,
  not a name a person reads.
- **Rename `--name` to `--title`.** GSD's argument is the phase's name; the flag sets the title, which is that name.
- **A repository-only check of this spec's titles.** The drift is a property of every `@gsd/core` project, and a
  converter is not the only writer of a numbered title.

## Consequences

- A project written for the old declaration carries `name` on its phases and milestones: W020 (a field its kind does
  not declare) on each until the lines are deleted. SpecForge is pre-release; no shim reads `name` as the title.
- Deleting the `name` lines changes the content digest of every phase and milestone that had one (an entity's digest
  is its authored values and its title, ADR 0290), so a verdict recorded over one goes stale once and is judged again.
  A retitle still stales a verdict over the phase: the title is in the digest, as it was when `--edit --name` wrote
  both copies.
- Every pass may read a title; the coverage rule and the field text do not change.
- A phase's renumber leaves its title and label consistent: the number is read from the id alone.

## Limits

- A guest reads `title` as absent when a host sends none, and `gsd/W109` then reports nothing: it never guesses a title
  from another field. No released host can do that to it: every 2.3.0 host sends the title, and an older host refuses
  a 2.3.0 guest (E088).

## Tests that hold it

| Test | Holds |
|---|---|
| `snapshot::tests::title::a_pass_entity_carries_the_title_its_header_writes` (`specforge-registry`) | D1 |
| `calls_roundtrip::a_pass_entity_carries_its_title_and_a_fact_without_one_none` (`specforge-protocol-types`) | D1 |
| `declaration_project::phase_and_milestone_are_named_by_their_title` (`gsd-core`) | D2: no `name` field |
| `cmd_phase::edit_name_retitles_the_phase`, `cmd_phase::a_new_name_rewrites_the_title_and_the_slug` | D2: the edit |
| `cmd_phase::a_name_of_two_lines_or_without_a_slug_is_refused_and_one_is_printed_as_stored` | D2: the edit |
| `ported_labels::the_phase_label_is_read_from_the_id_and_the_title` | D2: the workflows' label |
| `ported_labels::a_label_reads_its_name_from_the_phase_title_not_the_report` | D2: the workflows' label |
| `reports::tests::a_phase_reads_its_computed_values`, `statusline::tests::graph`, `community::tests` | D2: the title |
| `roadmap::a_numbered_phase_title_is_w109`, `roadmap::a_title_with_a_number_inside_is_not_w109` | D3 |
| `roadmap::tests::a_form_outside_the_label_grammar_is_not_w109` (the pass) | D3: the boundary |
| `roadmap::w109_is_an_error_under_gsd_strict_and_gsd_plus`, `roadmap::roadmap_errors_report_each_code_once` | D3 |
| `roadmap::a_name_line_on_a_phase_or_a_milestone_is_w020` | Consequences |
| `tests::roadmap` (`xtask` absorb), `the_copied_grammar_is_the_gsd_original` | ADR 0372 D4 as amended, D3 |
