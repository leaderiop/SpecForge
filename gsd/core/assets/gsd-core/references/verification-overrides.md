<!-- Ported from gsd-core v1.19.0 gsd-core/references/verification-overrides.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Verification Overrides

Mechanism for intentionally accepting must-have failures when the deviation is known and acceptable. Prevents verification loops on items that will never pass as originally specified.

<override_format>

## Override Format

An override is an `override` entity under the phase's verification, and only a person records one: the host refuses
an agent's write to it (`specforge schema --kind override` lists its fields). The person runs, in their terminal:

```
specforge new override OV<n> "<the truth's title>" --in <the verification's file> --verification <the verification> --truth <the truth> --accepts "<the truth's text>" --reason "<why>" --by "<who>" --at <YYYY-MM-DD>
```

The verifier reads them with `specforge query "$VERIFICATION" --depth 1 --kinds override --format context`.

### What an override records

| Field | Meaning |
|---|---|
| `truth` | The truth the person accepts. |
| `accepts` | The truth's text as the person accepted it: the override counts only while the truth still says this. |
| `reason` | What makes the deviation acceptable, said precisely: "not needed" is no reason. |
| `by` | Who accepted it (gsd/E503 when absent). |
| `at` | When (gsd/E503 when absent). |

</override_format>

## When to Use

Overrides apply when a phase intentionally deviated from the original plan during execution — for example, a requirement was descoped, an alternative approach was chosen, or a dependency changed.

Without overrides, the verifier reports these as FAIL even though the deviation was intentional. Overrides let the developer mark specific items as `PASSED (override)` with a documented reason.

Overrides are appropriate when:
- The roadmap has not caught up yet with a requirement that moved once the plan was made
- An alternative implementation satisfies the intent but not the literal wording
- A must-have is deferred to a later phase with explicit tracking
- External constraints make the original must-have impossible or unnecessary

## When NOT to Use

Overrides are NOT appropriate when:
- The implementation is simply incomplete — fix it instead
- The must-have is unclear — clarify it instead
- The developer wants to skip verification — that undermines the process
- Multiple must-haves are failing for the same phase — if more than 2-3 items need overrides, revisit the plan instead of overriding in bulk

<matching_rules>

## Matching Rules

Override matching uses **fuzzy matching**, not exact string comparison. This accommodates minor wording differences between how must-haves are phrased in ROADMAP.md, PLAN.md frontmatter, and the override entry.

### Matching Algorithm

1. **Normalize both strings:** case-insensitive comparison — lowercase both strings, strip punctuation, collapse whitespace
2. **Token overlap:** split into words, compute intersection
3. **Match threshold:** 80% token overlap in EITHER direction (override tokens found in must-have, OR must-have tokens found in override)
4. **Key noun priority:** nouns and technical terms (file paths, component names, API endpoints) are weighted higher than common words

### Examples

| Must-Have | Override `must_have` | Match? | Reason |
|-----------|---------------------|--------|--------|
| "User can authenticate via OAuth2 PKCE" | "OAuth2 PKCE flow implemented" | Yes | Key terms `OAuth2` and `PKCE` overlap, 80% threshold met |
| "Rate limiting on /api/auth/login" | "Rate limiting on login endpoint" | Yes | `rate limiting` + `login` overlap |
| "Chat component renders messages" | "OAuth2 PKCE flow implemented" | No | No meaningful token overlap |
| "src/components/Chat.tsx provides message list" | "Chat.tsx message list rendering" | Yes | `Chat.tsx` + `message` + `list` overlap |

### Ambiguity Resolution

If an override matches multiple must-haves, apply it to the **most specific match** (highest token overlap percentage). If still ambiguous, apply to the first match and log a warning.

</matching_rules>

<verifier_behavior>

## Verifier Behavior with Overrides

### Check Order

The override check happens **before marking a must-have as FAIL**. The flow is:

1. Evaluate must-have against codebase (Steps 3-5 of verification process)
2. If evaluation result is FAIL or UNCERTAIN:
   a. Look for a matching `override` among the verification's (`specforge query "$VERIFICATION" --depth 1 --kinds override`)
   b. If override found: mark as `PASSED (override)` instead of FAIL
   c. If no override found: mark as FAIL as normal
3. If evaluation result is PASS: mark as VERIFIED (overrides are irrelevant)

### Output Format

Overridden items appear with distinct status in all verification tables:

```markdown
| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can authenticate | VERIFIED | OAuth session flow working |
| 2 | OAuth2 PKCE flow | PASSED (override) | Override: Using session-based auth — accepted by dave on 2026-04-04 |
| 3 | Chat renders messages | FAILED | Component returns placeholder |
```

The `PASSED (override)` status must be visually distinct from both `VERIFIED` and `FAILED`. In the evidence column, include the override reason and who accepted it.

### Impact on Overall Status

- `PASSED (override)` items count toward the passing score, not the failing score
- A phase with all items either VERIFIED or PASSED (override) can have status `passed`
- Overrides do NOT suppress `human_needed` items — those still require human testing

### How an override counts

The verification's computed `score` counts each truth an override accepts as verified.

</verifier_behavior>

<creating_overrides>

## Creating Overrides

### Interactive Override Suggestion

When the verifier marks a must-have as FAIL and the failure looks intentional (e.g., alternative implementation exists, or the code explicitly handles the case differently), the verifier should suggest creating an override:

````markdown
### F-002: OAuth2 PKCE flow

**Status:** FAILED
**Evidence:** No PKCE implementation found. Session-based auth used instead.

**This looks intentional.** Your code gets there another way, through sessions rather than the flow this truth names. If you accept
the deviation, record an override in your terminal, then re-run verification:

```
specforge new override OV<n> "<the truth's title>" --in <the verification's file> --verification <the verification> --truth v1.0/03/03-01/TR2 --accepts "<the truth's text>" --reason "Session-based auth instead; PKCE is not needed for a server-rendered app" --by "<who>" --at <YYYY-MM-DD>
```
````

### Override via the verification workflow

Overrides can also be managed through the verification workflow:

1. Run `/gsd:verify-work` — verification finds gaps
2. Review gaps — determine which are intentional deviations
3. Record an override for each one you accept, in your terminal (the line above)
4. Re-run `/gsd:verify-work` — overrides are applied, remaining gaps shown

</creating_overrides>

<override_lifecycle>

## Override Lifecycle

### During Re-verification

When a phase is re-verified (e.g., after gap closure):
- Existing overrides carry forward automatically
- If the underlying code now satisfies the must-have, the override becomes unnecessary — mark as VERIFIED instead
- Overrides are never removed automatically; they persist as documentation

### At Milestone Completion

During `/gsd:audit-milestone`, overrides are surfaced in the audit report:

```
### Verification Overrides ({count} across {phase_count} phases)

| Phase | Must-Have | Reason | Accepted By |
|-------|----------|--------|-------------|
| 03 | OAuth2 PKCE | Session-based auth used instead | dave |
```

This gives the team visibility into all accepted deviations before closing the milestone.

### Cleanup

Stale overrides (where the must-have was later implemented or removed from ROADMAP.md) can be cleaned up during milestone completion. They are informational — leaving them causes no harm.

</override_lifecycle>

## Example

An override the person recorded on phase 3's verification, and the report the verifier returns:

```
specforge new override OV1 "<the truth's title>" --in <the verification's file> --verification <the verification> --truth v1.0/03/03-02/TR1 --accepts "<the truth's text>" --reason "Descoped: under 100 items, pagination adds complexity without value" --by "<who>" --at <YYYY-MM-DD>
```

```markdown
## Phase 3: API Layer — Verification

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | REST endpoints return JSON | VERIFIED | curl tests confirm |
| 2 | Paginated API responses | PASSED (override) | Descoped — see override: dataset under 100 items |
| 3 | Authentication middleware | VERIFIED | JWT validation working |
```
