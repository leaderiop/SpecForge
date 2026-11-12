---
quick_id: 261105-tcl
slug: totals-column
type: execute
files_modified: [src/lib.rs, src/main.rs, tests/totals.rs]
autonomous: true
must_haves:
  truths:
    - "A total prints right-aligned in a column of fourteen"
  artifacts:
    - path: src/lib.rs
      provides: "column"
      contains: "pub fn column"
---

<objective>
Show totals in a fixed-width column.

Purpose: totals line up when a user prints several stores one under another.
Output: `column(cents)` in `src/lib.rs`, used by `ledger total`.
</objective>

<tasks>

<task type="auto">
  <name>Task 1: The total's column</name>
  <files>src/lib.rs, src/main.rs, tests/totals.rs</files>
  <action>Add `column(cents)`: euros and cents, right-aligned in 14 characters; `ledger total` prints through it. Fold the total in an `i32` so the column's width is the value's width.</action>
  <verify>cargo test --test totals a_total_prints_in_a_column_of_fourteen</verify>
  <done>A total prints in a column of fourteen</done>
</task>

</tasks>

<success_criteria>
- A total prints right-aligned in a column of fourteen
</success_criteria>
