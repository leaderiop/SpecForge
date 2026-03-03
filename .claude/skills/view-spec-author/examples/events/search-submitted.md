# Search Submitted

> event | `EVT-search-submitted`

## Payload Schema

```
EVT-search-submitted
┌──────────────────────────────────────────────┐
│ payload                                       │
│                                               │
│  ┌───────────┬────────┬─────────────────────┐ │
│  │ Field     │ Type   │ Required            │ │
│  ├───────────┼────────┼─────────────────────┤ │
│  │ query     │ string │ yes                 │ │
│  │ timestamp │ number │ no (default: now()) │ │
│  └───────────┴────────┴─────────────────────┘ │
│                                               │
│  Example:                                     │
│  {                                            │
│    query: "weather forecast",                 │
│    timestamp: 1709012345678                   │
│  }                                            │
│                                               │
└──────────────────────────────────────────────┘
```

- `query` (string, required): The trimmed search query. Guaranteed non-empty
  by the action precondition.
- `timestamp` (number, optional): Unix timestamp in milliseconds. Used for
  request deduplication and ordering when multiple searches overlap.

## Target Store

```
  EVT-search-submitted
         │
         │  payload: { query, timestamp }
         │
         ▼
  ┌─────────────────────────────────────────┐
  │ STR-search-store                         │
  │                                          │
  │  Reducer:                                │
  │  ┌───────────┬────────────┬────────────┐ │
  │  │ Field     │ Operation  │ Value      │ │
  │  ├───────────┼────────────┼────────────┤ │
  │  │ query     │ SET        │ payload.   │ │
  │  │           │            │ query      │ │
  │  │ isLoading │ SET        │ true       │ │
  │  │ error     │ CLEAR      │ null       │ │
  │  │ results   │ CLEAR      │ []         │ │
  │  └───────────┴────────────┴────────────┘ │
  │                                          │
  │  State after:                            │
  │  {                                       │
  │    query: "weather forecast",            │
  │    results: [],                          │
  │    isLoading: true,                      │
  │    error: null,                          │
  │    totalResults: 0                       │
  │  }                                       │
  └─────────────────────────────────────────┘
```

The reducer optimistically clears previous results and errors, then sets
`isLoading` to `true`. This gives immediate visual feedback (loading state
on input and button) before the API responds.

## Side Effects

### 1. API Call

```
  EVT-search-submitted
         │
         ▼
  ┌────────────────────────────────────────┐
  │ Side Effect: api-call                   │
  │                                         │
  │  GET /api/search?q={query}              │
  │                                         │
  │  Headers:                               │
  │    Content-Type: application/json       │
  │                                         │
  │  ┌──────────┐        ┌──────────┐      │
  │  │ SUCCESS  │        │ FAILURE  │      │
  │  │          │        │          │      │
  │  │ dispatch │        │ dispatch │      │
  │  │ EVT-     │        │ EVT-     │      │
  │  │ search-  │        │ search-  │      │
  │  │ results- │        │ failed   │      │
  │  │ received │        │          │      │
  │  └──────────┘        └──────────┘      │
  │                                         │
  └────────────────────────────────────────┘
```

The query is URL-encoded and sent as a GET parameter. The API call is the
only async side effect in this event's chain.

### 2. Analytics

```
  EVT-search-submitted
         │
         ▼
  ┌──────────────────────────────┐
  │ Side Effect: analytics        │
  │                               │
  │  track("search", {            │
  │    query: payload.query       │
  │  })                           │
  │                               │
  └──────────────────────────────┘
```

The analytics event fires immediately (fire-and-forget). It does not wait
for the API response. This means every search attempt is tracked, even if
the API fails.

## Success / Failure Chain

```
  EVT-search-submitted
         │
         │  (API call in flight)
         │
    ┌────┴────┐
    │         │
 SUCCESS   FAILURE
    │         │
    ▼         ▼
  ┌─────────────────────┐    ┌─────────────────────┐
  │ EVT-search-results- │    │ EVT-search-failed    │
  │ received            │    │                      │
  │                     │    │ Reduces:             │
  │ Reduces:            │    │  isLoading = false   │
  │  results = [...]    │    │  error = message     │
  │  totalResults = N   │    │                      │
  │  isLoading = false  │    │ UI shows error state │
  │                     │    │ on search input      │
  │ UI renders results  │    └─────────────────────┘
  └─────────────────────┘
```

Both follow-up events target `STR-search-store`. The success path populates
results and clears loading. The failure path sets an error message and clears
loading. In both cases, the UI returns to an interactive state.

## Full Event Flow

```
  Source              Event                Targets             Side Effects
  ──────              ─────                ───────             ────────────
  ACT-submit-  ->    EVT-search-    ->    STR-search-    +   GET /api/search
  search              submitted            store          +   analytics("search")
                                                              │
                                                     ┌────────┴────────┐
                                                     │                 │
                                                  200 OK            Error
                                                     │                 │
                                                     ▼                 ▼
                                              EVT-search-       EVT-search-
                                              results-          failed
                                              received
```

## Ordering

```
  1. Store reducer       (synchronous -- sets isLoading: true)
  2. API call            (async -- HTTP request in flight)
  3. Analytics           (async -- fire-and-forget)
  4. API resolves        (async -- dispatches success/failure event)
  5. Store reducer       (synchronous -- handles success/failure)
```
