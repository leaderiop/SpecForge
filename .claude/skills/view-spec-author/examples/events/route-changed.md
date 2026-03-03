# Route Changed

> event | `EVT-route-changed`

## Payload Schema

```
EVT-route-changed
┌─────────────────────────────────────────┐
│ payload                                  │
│                                          │
│  ┌─────────┬────────┬──────────────────┐ │
│  │ Field   │ Type   │ Required         │ │
│  ├─────────┼────────┼──────────────────┤ │
│  │ path    │ string │ yes              │ │
│  │ params  │ object │ no (default: {}) │ │
│  └─────────┴────────┴──────────────────┘ │
│                                          │
│  Example:                                │
│  {                                       │
│    path: "/",                            │
│    params: {}                            │
│  }                                       │
│                                          │
└─────────────────────────────────────────┘
```

- `path` (string, required): The target route path. Always starts with `/`.
- `params` (object, optional): Key-value map of route parameters. Defaults to
  an empty object when omitted. In this app, the home route has no params, so
  this is always `{}`.

## Target Store Mapping

```
  EVT-route-changed
         │
         │  payload: { path, params }
         │
         ▼
  ┌─────────────────────────────────────┐
  │ STR-router-store                     │
  │                                      │
  │  Reducer:                            │
  │  ┌──────────────┬───────────────────┐│
  │  │ Field        │ Operation         ││
  │  ├──────────────┼───────────────────┤│
  │  │ previousPath │ SET state.current ││
  │  │              │     Path          ││
  │  │ currentPath  │ SET payload.path  ││
  │  │ params       │ SET payload.params││
  │  │              │     || {}         ││
  │  └──────────────┴───────────────────┘│
  │                                      │
  │  State after:                        │
  │  {                                   │
  │    previousPath: "/old",             │
  │    currentPath: "/",                 │
  │    params: {},                       │
  │    query: {}                         │
  │  }                                   │
  └─────────────────────────────────────┘
```

The event targets exactly one store. The reducer saves the old `currentPath`
into `previousPath` before overwriting it, creating a simple back-navigation
history.

## Side Effects

### 1. Navigation

```
  EVT-route-changed
         │
         ▼
  ┌──────────────────────┐
  │ Side Effect:         │
  │ navigation           │
  │                      │
  │ window.history       │
  │   .pushState(        │
  │     {},              │
  │     "",              │
  │     payload.path     │
  │   )                  │
  └──────────────────────┘
```

The navigation side effect calls `pushState` on the browser history API,
updating the URL bar without a full page reload. This keeps the application
in a single-page-app mode.

### 2. Analytics

```
  EVT-route-changed
         │
         ▼
  ┌──────────────────────┐
  │ Side Effect:         │
  │ analytics            │
  │                      │
  │ track("page_view", { │
  │   path: payload.path │
  │ })                   │
  └──────────────────────┘
```

Every route change fires a `page_view` analytics event. This is essential
for SPA analytics because the browser does not fire native page-load events
on client-side navigations.

## Event Flow

```
  Source            Event              Targets              Side Effects
  ──────            ─────              ───────              ────────────
  ACT-navigate  ->  EVT-route    ->   STR-router-store  +  pushState("/")
                    -changed                             +  analytics("page_view")
```

## Ordering

Side effects execute after the store reducer completes. The store is updated
first so that any component re-renders triggered by the state change see the
new route before the browser URL updates. The analytics call fires last.

```
  1. Store reducer   (synchronous)
  2. pushState       (synchronous)
  3. analytics       (fire-and-forget, async)
```
