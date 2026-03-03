# Submit Search

> action | `ACT-submit-search`

## Flow Diagram

```
  ┌─────────────────┐     ┌──────────────────┐
  │ ELM-search-     │     │ ELM-search-      │
  │ input           │     │ button           │
  │                 │     │                  │
  │ (submit/change) │     │ (click)          │
  └────────┬────────┘     └────────┬─────────┘
           │                       │
           └───────────┬───────────┘
                       │
                       ▼
               ┌───────────────┐
               │ Debounce      │
               │               │
               │ wait: 300ms   │
               │ leading: no   │
               │ trailing: yes │
               └───────┬───────┘
                       │
                       │  (300ms elapsed, no new triggers)
                       │
                       ▼
               ┌───────────────────┐
               │ Precondition      │
               │ Check             │
               │                   │
               │ query.trim()      │
               │   .length > 0     │
               └───────┬───────────┘
                       │
              ┌────────┴────────┐
              │                 │
           PASS              FAIL
              │                 │
              ▼                 ▼
  ┌───────────────────┐   ┌──────────┐
  │ EVT-search-       │   │  BLOCK   │
  │ submitted         │   │          │
  │                   │   │ No event │
  │ payload:          │   │ dispatch │
  │   query: "..."    │   └──────────┘
  │   timestamp: now  │
  └───────┬───────────┘
          │
          ├──────────────────────┐
          │                      │
          ▼                      ▼
  ┌───────────────┐    ┌────────────────┐
  │ STR-search-   │    │ Side Effects   │
  │ store         │    │                │
  │               │    │ 1. API call    │
  │ query = "..." │    │    GET /api/   │
  │ isLoading     │    │    search?q=   │
  │  = true       │    │                │
  │ error = null  │    │ 2. analytics   │
  │ results = []  │    │    "search"    │
  └───────────────┘    └───────┬────────┘
                               │
                      ┌────────┴────────┐
                      │                 │
                   SUCCESS           FAILURE
                      │                 │
                      ▼                 ▼
          ┌──────────────────┐  ┌──────────────────┐
          │ EVT-search-      │  │ EVT-search-      │
          │ results-received │  │ failed           │
          └──────────────────┘  └──────────────────┘
```

## Trigger

| Source             | Interaction | Notes                        |
|--------------------|-------------|------------------------------|
| `ELM-search-input` | `submit`    | Enter key press              |
| `ELM-search-input` | `change`    | Every keystroke (debounced)  |
| `ELM-search-button` | `click`    | Button click                 |

Multiple elements can trigger this action. All triggers pass through the
same debounce gate.

## Debounce

| Property   | Value   | Rationale                                      |
|------------|---------|------------------------------------------------|
| `wait`     | `300ms` | Fast enough to feel responsive, slow enough to  |
|            |         | batch rapid keystrokes into one API call        |
| `leading`  | `false` | Do not fire on the first trigger                |
| `trailing` | `true`  | Fire after the wait period with the latest value |

The debounce prevents excessive API calls during typing. For example, typing
"weather" fires 7 change events but only one API call 300ms after the last
keystroke.

The button click also passes through the debounce. This means if the user
types and immediately clicks the button, only one search executes (the
trailing edge of the debounce window).

## Preconditions

| Condition                    | Fail Action | Rationale                    |
|------------------------------|-------------|------------------------------|
| `query.trim().length > 0`   | `block`     | Prevents empty/whitespace    |
|                              |             | searches from hitting the API |

When the precondition fails, the action is silently blocked. No event is
dispatched and no error is shown to the user. This is intentional: an empty
search bar is not an error state, it is simply a not-yet-ready state.

## Event Dispatched

| Event                 | Payload                                   |
|-----------------------|-------------------------------------------|
| `EVT-search-submitted` | `{ query: string, timestamp: number }`   |

The `timestamp` field is optional in the payload schema but is always
populated by the action. It uses `Date.now()` and can be used for request
deduplication or ordering.

## Sequence (Happy Path)

```
  Time ────────────────────────────────────────────────────────>

  User     Debounce      Precondition     Event         Store
  ────     ────────      ────────────     ─────         ─────
  type  ->  (wait)
  type  ->  (reset)
  type  ->  (reset)
            (300ms) ->   check: PASS  ->  EVT-search ->  query="..."
                                          -submitted     isLoading=true
                                              │
                                              ├──> GET /api/search
                                              │
                                              └──> analytics
                                                       │
                                               ┌───────┴───────┐
                                               │               │
                                            success          failure
                                               │               │
                                               ▼               ▼
                                          EVT-search-     EVT-search-
                                          results-        failed
                                          received
```

## Edge Cases

| Scenario                          | Behavior                              |
|-----------------------------------|---------------------------------------|
| Empty query after trim            | Blocked by precondition               |
| Whitespace-only query             | Blocked (trim reduces to empty)       |
| Query exceeds 256 chars           | Input validation blocks before action |
| Rapid Enter + click               | Debounce collapses to one dispatch    |
| Submit while already loading      | New search replaces in-flight search  |
