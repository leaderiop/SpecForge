<!-- Ported from gsd-core v1.19.0 gsd-core/references/verifier-wiring-patterns.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Verifier wiring and data-flow patterns

Full pattern bodies for `agents/gsd-verifier.md`, extracted per
`DEFECT.AGENT-FILE-SIZE-CAP-BREACH` (issue #2995, epic #1671 Phase 6.4). The agent
keeps the step and its checklist; the per-pattern detail and shell recipes live here.

Artifacts that pass Levels 1-3 (exist, substantive, wired) can still be hollow if their data source produces empty or hardcoded values. Level 4 traces upstream from the artifact to verify real data flows through the wiring.

**When to run:** For each artifact that passes Level 3 (WIRED) and renders dynamic data (components, pages, dashboards — not utilities or configs).

**How:**

1. **Identify the data variable** — what state/prop does the artifact render?

```bash
# Find state variables that are rendered in JSX/TSX
specforge files search 'useState|useQuery|useSWR|useStore|props\.' "$artifact" --format human
```

2. **Trace the data source** — where does that variable get populated?

```bash
# Find the fetch/query that populates the state
specforge files search -U "(set${STATE_VAR}|${STATE_VAR}[ \t]*=)(.*\n){0,5}?.*(fetch|axios|query|store|dispatch|props\.)" "$artifact" --format human
```

3. **Verify the source produces real data** — does the API/store return actual data or static/empty values?

```bash
# Check the API route or data source for real DB queries vs static returns
specforge files search 'prisma\.|db\.|query\(|findMany|findOne|select|FROM' "$source_file" --format human
# Flag: static returns with no query
specforge files search 'return.*json\(\s*\[\]|return.*json\(\s*\{\}' "$source_file" --format human
```

4. **Check for disconnected props** — props passed to child components that are hardcoded empty at the call site

```bash
# Find where the component is used and check prop values
specforge files search -U "<${COMPONENT_NAME}(.*\n){0,3}?.*=\{(\[\]|\{\}|null|''|\"\")\}" "${search_path:-src/}" --glob '*.tsx' --format human
```

> These two status tables are intentionally mirrored in `agents/gsd-verifier.md`.
> The status vocabulary is load-bearing verifier output and must remain in the
> agent body (#2995); this copy is here so the procedure below reads standalone.

**Data-flow status:**

| Data Source | Produces Real Data | Status |
| ---------- | ------------------ | ------ |
| DB query found | Yes | ✓ FLOWING |
| Fetch exists, static fallback only | No | ⚠️ STATIC |
| No data source found | N/A | ✗ DISCONNECTED |
| Props hardcoded empty at call site | No | ✗ HOLLOW_PROP |

**Final Artifact Status (updated with Level 4):**

| Exists | Substantive | Wired | Data Flows | Status |
| ------ | ----------- | ----- | ---------- | ------ |
| ✓ | ✓ | ✓ | ✓ | ✓ VERIFIED |
| ✓ | ✓ | ✓ | ✗ | ⚠️ HOLLOW — wired but data disconnected |
| ✓ | ✓ | ✗ | - | ⚠️ ORPHANED |
| ✓ | ✗ | - | - | ✗ STUB |
| ✗ | - | - | - | ✗ MISSING |

### Pattern: Component → API

```bash
specforge files search "fetch\(['\"].*$api_path|axios\.(get|post).*$api_path" "$component" --format human
specforge files search -U '(fetch|axios)(.*\n){0,5}?.*(await|\.then|setData|setState)' "$component" --format human
```

Status: WIRED (call + response handling) | PARTIAL (call, no response use) | NOT_WIRED (no call)

### Pattern: API → Database

```bash
specforge files search "prisma\.$model|db\.$model|$model\.(find|create|update|delete)" "$route" --format human
specforge files search 'return.*json.*\w+|res\.json\(\w+' "$route" --format human
```

Status: WIRED (query + result returned) | PARTIAL (query, static return) | NOT_WIRED (no query)

### Pattern: Form → Handler

```bash
specforge files search 'onSubmit=\{|handleSubmit' "$component" --format human
specforge files search -U 'onSubmit.*=(.*\n){0,10}?.*(fetch|axios|mutate|dispatch)' "$component" --format human
```

Status: WIRED (handler + API call) | STUB (only logs/preventDefault) | NOT_WIRED (no handler)

### Pattern: State → Render

```bash
specforge files search "useState.*$state_var|\[$state_var," "$component" --format human
specforge files search "\{.*$state_var.*\}|\{$state_var\." "$component" --format human
```

Status: WIRED (state displayed) | NOT_WIRED (state exists, not rendered)
