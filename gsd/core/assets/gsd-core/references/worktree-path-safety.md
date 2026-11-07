<!-- Ported from gsd-core v1.19.0 gsd-core/references/worktree-path-safety.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Worktree Path Safety

Guards for executor agents running inside Claude Code worktrees. The
supplied-root pin (step 0p) runs in EVERY mode; the remaining checks run before
any staging, Edit, or Write operation in worktree mode.

---

## Supplied-root pin — step 0p (#4254, EVERY mode)

Sequential-mode dispatch (no `isolation="worktree"`) gives the executor no
spawn-time cwd guarantee, and the worktree-only guards below do not apply — so
a sequential executor whose process cwd resolved to a different checkout of
the same repo would self-derive that checkout as its root and commit there,
silently. Step 0p closes that hole by comparing the executor's actual root
against a root the ORCHESTRATOR already validated — never against anything the
executor derives itself.

**Runtime contract (executor):** if your prompt contains a `<project_root_pin>`
block, run its guard script verbatim before your first Edit/Write and again
before every commit, in the same cwd as that write or commit. On FATAL, halt
and report — recovery (moving commits between checkouts) is an
orchestrator/human decision, never agent self-repair. If your prompt contains
NO `<project_root_pin>` block (worktree/isolated dispatch, or a legacy
orchestrator), emit one warning line and continue with steps 0a/0b below — do
not fail closed on dispatches that never carried a pin. **Never bind
`{PINNED_ROOT}` yourself**: if this template reaches you unbound it is
reference prose, not your pin — only the orchestrator's build-time
substitution produces a valid guard.

**Composition contract (orchestrator — build time, NOT a sub-agent runtime
step):** copy the guard below into the dispatched prompt inside a
`<project_root_pin>` block, substituting `{PINNED_ROOT}` with the literal value
of `$ORCHESTRATOR_WT` captured at execute_waves entry, shell-single-quoted:
wrap the path in `'…'` and escape any embedded `'` as `'\''`. A path that
cannot be quoted this way must halt the phase (surface a blocker) rather than
ship a pin that could mis-parse. The comparison is one reader on BOTH sides —
`specforge git repo`, run from the pinned path, resolves it to its repo's top with
every link resolved, as it does the cwd's, so symlink aliases, trailing slashes,
`/var` vs `/private/var` spellings, and Windows drive-letter forms — forward- or
backslash-separated, `RUNNER~1`-style short names included — compare equal by
construction (shell `pwd -P` normalization does NOT match that spelling on
Windows — do not re-introduce it).

Two portability rules baked into the guard below, learned from the #4254 CI
Windows legs: (1) a backslash comparator must be GENERATED at runtime
(`printf '\134'`), because a backslash written twice in the script text does
not survive the Windows command-line round-trip into bash — the doubled form
arrives halved, which silently rewrites any escape pattern that relies on it;
(2) every FATAL names its `Guard stage` and, where a git capture failed,
git's own stderr in a `Diagnostic` line, so a platform failure self-describes
instead of surfacing as a bare `Actual root: <none>`.

```bash
# gsd:guard=supplied-root-pin (#4254) — run before the first Edit/Write and before every commit.
PINNED_ROOT='{PINNED_ROOT}'  # orchestrator build-time substitution — the only valid source of this value
PIN_STAGE=''
PIN_DIAG=''
gsd_pin_fail() {
  echo "FATAL: executor root does not match the orchestrator-supplied PROJECT_ROOT pin (#4254)." >&2
  echo "  Pinned root: ${PINNED_ROOT:-<empty or unexpanded>}" >&2
  echo "  Actual root: ${ACTUAL_ROOT:-<none>}" >&2
  echo "  Guard stage: ${PIN_STAGE:-<unset>}" >&2
  if [ -n "$PIN_DIAG" ]; then echo "  Diagnostic: $PIN_DIAG" >&2; fi
  echo "  No writes or commits are permitted from this checkout. HALT and report; recovery is an" >&2
  echo "  orchestrator/human decision. Only the IMMEDIATE submodule of the pinned checkout is a" >&2
  echo "  legitimate other cwd — nested submodules must surface as a blocker, not self-route." >&2
  exit 1
}
# Backslash comparator, generated at runtime: a backslash written twice in this
# script does not survive the Windows spawn path into bash (the command-line
# round-trip halves the doubled form), which rejected every C:\ pin at the form
# gate on the #4254 CI Windows legs. printf's octal escape is a lone backslash,
# which does survive; the quoted expansion below is literal in a case pattern.
BS=$(printf '\134')
# Fail closed if the comparator could not be generated: an empty BS would widen
# the drive-form arm below to drive-RELATIVE pins (C:foo) — the one fail-open
# seam in this construction, closed loudly rather than trusted to the shell.
if [ -z "$BS" ]; then
  PIN_STAGE=form-gate
  PIN_DIAG='backslash comparator generation failed (printf octal escape returned empty)'
  gsd_pin_fail
fi
case "$PINNED_ROOT" in
  ''|'{PINNED_ROOT}') PIN_STAGE=pin-unbound; gsd_pin_fail ;;  # empty or unexpanded pin — fail closed, never warn-and-proceed
  /*) ;;                                                     # absolute POSIX form
  [A-Za-z]:/*|[A-Za-z]:"$BS"*) ;;                            # Windows drive form, forward- or backslash-separated
  *) PIN_STAGE=form-gate; gsd_pin_fail ;;                    # relative pin — never trustworthy across cwds
esac
ACTUAL_ROOT=$(specforge git repo --pick .top 2>/dev/null)
if [ -z "$ACTUAL_ROOT" ]; then
  PIN_STAGE=actual-capture
  PIN_DIAG="specforge git repo from the cwd failed: $(specforge git repo --pick .top 2>&1 1>/dev/null)"
  gsd_pin_fail
fi
PINNED_TL=$(cd "$PINNED_ROOT" 2>/dev/null && specforge git repo --pick .top 2>/dev/null)   # cd, not --in: the pin may lie outside the cwd's project
if [ -z "$PINNED_TL" ]; then
  PIN_STAGE=pinned-capture
  PIN_DIAG="specforge git repo from the pinned root failed: $(cd "$PINNED_ROOT" 2>&1 && specforge git repo --pick .top 2>&1 1>/dev/null)"
  gsd_pin_fail
fi
if [ "$ACTUAL_ROOT" != "$PINNED_TL" ]; then
  # Registered-submodule allowance: sub_repos plans legitimately commit inside an
  # immediate submodule of the pinned checkout. The superproject working tree is
  # answered by the same reader as PINNED_TL, so the equality is
  # representation-safe on every platform.
  SUPER_TL=$(specforge git repo --pick .superproject 2>/dev/null)
  if [ "$SUPER_TL" != "$PINNED_TL" ]; then
    PIN_STAGE=root-mismatch
    PIN_DIAG="actual=${ACTUAL_ROOT} pinned=${PINNED_TL} superproject=${SUPER_TL:-<none>}"
    gsd_pin_fail
  fi
fi
```

---

## Worktree branch check (run once at spawn-time)

The spawn-time HEAD/base guard now lives in the canonical fragment
`gsd-core/references/worktree-branch-check.md`, which the orchestrator embeds directly
into your prompt at dispatch. Run that block FIRST, before any reset/checkout or staging.
If your prompt contains a `<worktree_branch_check>` embed instruction rather than the block itself, complete that read-and-embed step before any reset/checkout or staging.

---

## cwd-drift sentinel — step 0a (#3097)

A prior Bash call may have `cd`'d out of the worktree into the main repo. When
that happens `[ -f .git ]` is false (main repo's `.git` is a directory), silently
skipping all worktree guards. The sentinel captures the spawn-time toplevel and
detects drift before every commit.

```bash
if [ -f .git ]; then  # we are in a worktree
  WT_GIT_DIR=$(specforge git repo --pick .git_dir 2>/dev/null)
  case "$WT_GIT_DIR" in
    *.git/worktrees/*)
      SENTINEL="$WT_GIT_DIR/gsd-spawn-toplevel"
      [ ! -f "$SENTINEL" ] && specforge git repo --pick .top > "$SENTINEL" 2>/dev/null
      EXPECTED_TL=$(cat "$SENTINEL" 2>/dev/null)
      ACTUAL_TL=$(specforge git repo --pick .top 2>/dev/null)
      if [ -n "$EXPECTED_TL" ] && [ "$ACTUAL_TL" != "$EXPECTED_TL" ]; then
        echo "FATAL: cwd drifted from spawn-time worktree root (#3097)" >&2
        echo "  Spawn-time: $EXPECTED_TL" >&2
        echo "  Current:    $ACTUAL_TL" >&2
        echo "RECOVERY: cd \"$EXPECTED_TL\" before staging, then re-run this commit." >&2
        exit 1
      fi
      ;;
  esac
fi
```

---

## Absolute-path guard — step 0b (#3099)

Edit/Write calls using absolute paths constructed from the **orchestrator's** `pwd`
(main repo root) will resolve to the main repo, not the worktree. Writes land in
the wrong directory; `git commit` from the worktree sees a clean tree and the work
is silently lost.

Before any Edit or Write using an absolute path:

```bash
WT_ROOT=$(specforge git repo --pick .top 2>/dev/null)
# Fail fast if ABS_PATH resolves outside the worktree
if [[ "$ABS_PATH" != "$WT_ROOT"* ]]; then
  echo "WARNING: $ABS_PATH is outside the worktree ($WT_ROOT)" >&2
  echo "Use a relative path or recompute the absolute path from WT_ROOT." >&2
fi
```

**Prefer relative paths** for all Edit/Write operations. When an absolute path is
unavoidable, always derive it from `git rev-parse --show-toplevel` run inside the
worktree — never from `pwd` captured in the orchestrator context.

---

## `<automated>` command guard — step 0c (#4767)

The plan's `<automated>` text is where an orchestrator-cwd absolute path most often
arrives: the planner saw absolute paths in its own context and wrote one into the
command. Run as written, `cd /abs/main-checkout/… && <test>` leaves the worktree, runs
against the main tree, and **passes on code this worktree changed and the main tree did
not** — a green verify that verified nothing. Before executing any `<automated>` command,
scan its text for absolute paths and halt if one is outside the worktree. Fail loud; never
rewrite the prefix silently (#3050) — a rewritten command hides the defective plan, and the
next executor meets it again.

```bash
# Reads the task's <automated> commands as the shell's words and never runs them. A directory one relocates
# to outside this worktree, an absolute path inside the main checkout, or a command it cannot read is a FATAL
# line and exit 1; what the shell would build at run time passes, named under `dynamic` in the answer.
TASK_ID='{TASK_ID}'  # bound when this is run: the graph's id of the task being verified, its plan's `<id>/T<n>`
case "$TASK_ID" in ''|'{TASK_ID}') echo "FATAL: step 0c has no task to read" >&2; exit 1 ;; esac
COMMON_DIR=$(specforge git repo --pick .common_dir) || exit 1
specforge gsd automated-guard "$TASK_ID" --task --common-dir "$COMMON_DIR" || exit 1









































































































































































































```

A halt here is a plan defect, not an executor deviation: report it via the checkpoint return
format naming the task and the offending command verbatim, and stop. The plan-checker's path
probe (`check verify-command-paths`) warns on the *outside-orchestrator-root* case before
execution; this guard is the one that sees the executor's actual root.

### What this guard does NOT see — step 0c's stated boundary

The answer recognizes **fixed sets** — relocating verbs, directory flags, shells, interpreters, launchers —
and the domain they act on, shell a planner may write, can acquire a member without this file changing.
So the boundary is written out rather than left to be rediscovered, and each entry says what follows
from it.

**The largest one first, because it is the one a reader will otherwise assume away:**

- **A RELATIVE argument of a command that does not relocate.** A test runner handed ../main/tests, a
  script path that climbs out, a makefile named with -f, a redirect from a relative file: all pass. Only
  words that begin with a slash are read as paths, because resolving every relative word halts on
  ordinary in-worktree arguments unless it can tell a path from a flag value from a bare string, and a
  guard that fails toward false positives is the kind that gets deleted.

The rest, in descending order of how likely a planner is to reach them:

- **What the shell builds at run time.** A variable, a substitution's result, a glob or a tilde: only the
  part written out is read, and the expansion counts as one directory, so a literal prefix in the main
  checkout or a climb out of the worktree halts. The rest passes, named under `dynamic`. The commands
  inside a substitution and a function's body are read, and an absolute target is always read.
- **A launcher, interpreter or directory flag outside the tables.** A command behind an unknown launcher
  is one more word, and a directory flag on an unknown tool is an ordinary argument. Its absolute form is
  still caught; what is uncovered is the relative form. The scoping is per tool because the same spelling
  is not the same flag everywhere: a prefix flag on an archive names its members and moves nothing.
- **A path computed in program text.** An interpreter's program is not shell, so only literal absolute
  paths are read from it, never one the program assembles.
- **Nesting past four.** A payload inside a payload is read to a bounded depth, which keeps a pathological
  nest finite; past it nothing is read.
- **A link outside the project, or another spelling of the main checkout.** Targets resolve lexically, and
  a link is seen only inside the project. An absolute path that reaches the main checkout through a link
  elsewhere is not recognized as inside it.

The tables are the parts that rot: they mirror a toolchain this file does not own. The guard is the
executor's last containment check, not its only one — a halt it misses still has to get past the plan
checker's path probe at plan time.























































