# Search Store

> store | `STR-search-store`

## State Shape

```
STR-search-store
┌──────────────────────────────────────────────┐
│                                               │
│  ┌──────────────┬──────────┬───────────────┐  │
│  │ Field        │ Type     │ Initial       │  │
│  ├──────────────┼──────────┼───────────────┤  │
│  │ query        │ string   │ ""            │  │
│  │ results      │ array    │ []            │  │
│  │ isLoading    │ boolean  │ false         │  │
│  │ error        │ string?  │ null          │  │
│  │ totalResults │ number   │ 0             │  │
│  └──────────────┴──────────┴───────────────┘  │
│                                               │
│  Example (mid-search):                        │
│  {                                            │
│    query: "weather forecast",                 │
│    results: [],                               │
│    isLoading: true,                           │
│    error: null,                               │
│    totalResults: 0                            │
│  }                                            │
│                                               │
│  Example (after success):                     │
│  {                                            │
│    query: "weather forecast",                 │
│    results: [                                 │
│      { title: "...", url: "...", ... },        │
│      { title: "...", url: "...", ... }         │
│    ],                                         │
│    isLoading: false,                          │
│    error: null,                               │
│    totalResults: 42                           │
│  }                                            │
│                                               │
└──────────────────────────────────────────────┘
```

The store tracks the full lifecycle of a search operation: the query that
initiated it, the loading state, the results, and any errors.

## Reducer Table

```
  ┌──────────────────────┬─────────────┬───────────┬────────────────┐
  │ Event                │ Field       │ Operation │ Value          │
  ├──────────────────────┼─────────────┼───────────┼────────────────┤
  │ EVT-search-submitted │ query       │ SET       │ payload.query  │
  │                      │ isLoading   │ SET       │ true           │
  │                      │ error       │ CLEAR     │ null           │
  │                      │ results     │ CLEAR     │ []             │
  └──────────────────────┴─────────────┴───────────┴────────────────┘
```

The reducer handles one event. When a search is submitted, the store
transitions to a clean loading state: query is set, results and error are
cleared, and `isLoading` becomes `true`.

### Reducer Flow

```
  EVT-search-submitted { query: "weather forecast" }
         │
         ▼
  ┌────────────────────────────────────────────┐
  │ Step 1: query = payload.query              │
  │         query: "weather forecast"          │
  ├────────────────────────────────────────────┤
  │ Step 2: isLoading = true                   │
  │         isLoading: true                    │
  ├────────────────────────────────────────────┤
  │ Step 3: error = null (CLEAR)               │
  │         error: null                        │
  ├────────────────────────────────────────────┤
  │ Step 4: results = [] (CLEAR)               │
  │         results: []                        │
  └────────────────────────────────────────────┘
```

Note: `EVT-search-results-received` and `EVT-search-failed` events (dispatched
by the API side effect) would add additional reducers for setting `results`,
`totalResults`, `error`, and `isLoading = false`. These events are referenced
in the search-submitted event spec but their store reducers are not defined
in this example to keep the scope focused.

## Selectors

```
  ┌─────────────┬──────────────────────────┬──────────────────┐
  │ Selector    │ Compute                  │ Return Type      │
  ├─────────────┼──────────────────────────┼──────────────────┤
  │ query       │ state.query              │ string           │
  │ results     │ state.results            │ array            │
  │ isLoading   │ state.isLoading          │ boolean          │
  │ hasResults  │ state.results.length > 0 │ boolean          │
  │ hasError    │ state.error !== null     │ boolean          │
  │ resultCount │ state.totalResults       │ number           │
  └─────────────┴──────────────────────────┴──────────────────┘
```

### Selector Dependency Graph

```
  state.query ────────> query       (identity)

  state.results ──────> results     (identity)
                  ────> hasResults  (derived: length > 0)

  state.isLoading ────> isLoading   (identity)

  state.error ────────> hasError    (derived: !== null)

  state.totalResults ─> resultCount (identity)
```

The `hasResults` and `hasError` selectors are boolean derivations that
simplify conditional rendering in consuming components. Instead of checking
`results.length > 0` in every consumer, the selector centralizes the logic.

## Consumer List

```
  STR-search-store
         │
         ├── selector: query      ──> CMP-search-bar.searchValue (two-way)
         │                             └── ELM-search-input.value
         │
         ├── selector: isLoading  ──> CMP-search-bar.loading (read-only)
         │                             ├── ELM-search-input.state
         │                             └── ELM-search-button.state
         │
         └── (selectors: results, hasResults, hasError, resultCount)
              └── (future: results list component)
```

| Consumer            | Selector    | Mapped To      | Direction |
|---------------------|-------------|----------------|-----------|
| `CMP-search-bar`    | `query`     | `searchValue`  | two-way   |
| `CMP-search-bar`    | `isLoading` | `loading`      | read-only |
| `ELM-search-input`  | `query`     | field binding  | two-way   |
| `ELM-search-button` | `isLoading` | field binding  | read-only |

Three consumers subscribe to this store. The search bar component gets both
`query` and `isLoading` and passes them down to its children. The input and
button also bind directly for field-level reactivity.

## Persistence

| Property  | Value             |
|-----------|-------------------|
| `enabled` | `true`            |
| `storage` | `session-storage` |
| `key`     | `"search-state"`  |

Persistence is enabled for this store. The entire state is serialized to
`sessionStorage` under the key `"search-state"`. This means:

- Refreshing the page restores the last search query and results.
- Opening a new tab starts with a clean state (sessionStorage is per-tab).
- Closing the tab discards the state.

### What Gets Persisted

```
  sessionStorage["search-state"] = JSON.stringify({
    query: "weather forecast",
    results: [...],
    isLoading: false,          // always persisted as false
    error: null,
    totalResults: 42
  })
```

`isLoading` is always persisted as `false` to avoid restoring into a loading
state with no in-flight request. On restore, if a `query` exists, the UI
shows the previous results without re-fetching.

### Hydration Flow

```
  Page Load
       │
       ▼
  ┌──────────────────────────┐
  │ Check sessionStorage     │
  │ key: "search-state"      │
  └────────────┬─────────────┘
               │
      ┌────────┴────────┐
      │                 │
   Found            Not Found
      │                 │
      ▼                 ▼
  ┌──────────┐    ┌──────────┐
  │ Parse &  │    │ Use      │
  │ restore  │    │ initial  │
  │ state    │    │ state    │
  └──────────┘    └──────────┘
```
