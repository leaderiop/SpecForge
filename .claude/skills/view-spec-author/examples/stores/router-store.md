# Router Store

> store | `STR-router-store`

## State Shape

```
STR-router-store
┌──────────────────────────────────────────────┐
│                                               │
│  ┌──────────────┬─────────┬────────────────┐  │
│  │ Field        │ Type    │ Initial        │  │
│  ├──────────────┼─────────┼────────────────┤  │
│  │ currentPath  │ string  │ "/"            │  │
│  │ previousPath │ string? │ null           │  │
│  │ params       │ object  │ {}             │  │
│  │ query        │ object  │ {}             │  │
│  └──────────────┴─────────┴────────────────┘  │
│                                               │
│  Example (after one navigation):              │
│  {                                            │
│    currentPath: "/",                          │
│    previousPath: "/results",                  │
│    params: {},                                │
│    query: {}                                  │
│  }                                            │
│                                               │
└──────────────────────────────────────────────┘
```

The state is intentionally flat. No nested objects, no arrays. This makes
equality checks cheap (shallow compare) and reducer logic trivial.

## Reducer Table

```
  ┌─────────────────────┬──────────────┬───────────┬────────────────────┐
  │ Event               │ Field        │ Operation │ Value              │
  ├─────────────────────┼──────────────┼───────────┼────────────────────┤
  │ EVT-route-changed   │ previousPath │ SET       │ state.currentPath  │
  │                     │ currentPath  │ SET       │ payload.path       │
  │                     │ params       │ SET       │ payload.params||{} │
  └─────────────────────┴──────────────┴───────────┴────────────────────┘
```

The reducer responds to exactly one event. The operation order matters:
`previousPath` must be set before `currentPath` is overwritten, because
`previousPath` reads from the current (pre-update) value of `currentPath`.

### Reducer Flow

```
  EVT-route-changed { path: "/new", params: { id: "42" } }
         │
         ▼
  ┌─────────────────────────────────────────┐
  │ Step 1: previousPath = state.currentPath│
  │         previousPath: "/" (was null)     │
  │         currentPath:  "/" (unchanged)    │
  ├─────────────────────────────────────────┤
  │ Step 2: currentPath = payload.path      │
  │         previousPath: "/"               │
  │         currentPath:  "/new"            │
  ├─────────────────────────────────────────┤
  │ Step 3: params = payload.params || {}   │
  │         params: { id: "42" }            │
  └─────────────────────────────────────────┘
```

## Selectors

```
  ┌───────────────┬─────────────────────────┬────────────────────────┐
  │ Selector      │ Compute                 │ Return Type            │
  ├───────────────┼─────────────────────────┼────────────────────────┤
  │ currentPath   │ state.currentPath       │ string                 │
  │ previousPath  │ state.previousPath      │ string | null          │
  │ isHome        │ state.currentPath==="/" │ boolean                │
  └───────────────┴─────────────────────────┴────────────────────────┘
```

Selectors are pure derivations. They take the current state and return a
value. No side effects, no mutations. The `isHome` selector is a derived
boolean that avoids string comparison in consuming components.

### Selector Dependency Graph

```
  state.currentPath ──> currentPath (identity)
                    ──> isHome      (derived)

  state.previousPath -> previousPath (identity)
```

## Consumer List

```
  STR-router-store
         │
         │  selector: currentPath
         │
         ▼
  ┌───────────────┐
  │ CMP-header    │
  │               │
  │ binds to:     │
  │ activePath    │
  └───────────────┘
```

| Consumer     | Selector      | Mapped To    | Usage                           |
|--------------|---------------|--------------|---------------------------------|
| `CMP-header` | `currentPath` | `activePath` | Active route indicator styling  |

Only one component consumes this store. The binding is read-only; the header
never writes back to the router store.

## Persistence

| Property  | Value             |
|-----------|-------------------|
| `enabled` | `false`           |
| `storage` | `session-storage` |
| `key`     | `"router-state"`  |

Persistence is disabled. Route state is ephemeral and reconstructed from the
URL on each page load. The configuration is included but inactive, showing
that session-storage was considered and explicitly rejected. URL is the source
of truth for navigation state.
