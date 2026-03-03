# Navigate

> action | `ACT-navigate`

## Flow Diagram

```
  ┌───────────────┐
  │  ELM-logo     │
  │               │
  │  (click or    │
  │   Enter key)  │
  └───────┬───────┘
          │
          ▼
  ┌───────────────┐
  │ ACT-navigate  │
  │               │
  │ type:navigate │
  └───────┬───────┘
          │
          │  preconditions: []
          │  (none -- always allowed)
          │
          ▼
  ┌───────────────────┐
  │ EVT-route-changed │
  │                   │
  │ payload:          │
  │   path: "/"       │
  │   params: {}      │
  └───────┬───────────┘
          │
          ├──────────────────────┐
          │                      │
          ▼                      ▼
  ┌───────────────┐    ┌────────────────┐
  │ STR-router-   │    │ Side Effects   │
  │ store         │    │                │
  │               │    │ 1. navigation  │
  │ previousPath  │    │    pushState   │
  │  = old path   │    │    to "/"      │
  │ currentPath   │    │                │
  │  = "/"        │    │ 2. analytics   │
  │               │    │    page_view   │
  └───────────────┘    │    path: "/"   │
                       └────────────────┘
```

## Trigger

| Source      | Interaction | Element    |
|-------------|-------------|------------|
| `ELM-logo`  | `click`     | Logo image |

The navigate action fires when the user clicks the logo or activates it via
keyboard (Enter key). There is only one trigger source.

## Preconditions

None. Navigation to home is always permitted. There is no guard, no
authentication check, and no confirmation dialog. This makes the logo a
reliable escape hatch that always works regardless of application state.

## Event Dispatched

| Event               | Payload                              |
|---------------------|--------------------------------------|
| `EVT-route-changed` | `{ path: "/", params: {} }`          |

The action always dispatches the route-changed event with a fixed payload
pointing to the home route. The `params` object is empty because the home
route has no path parameters.

## Sequence

```
  Time ──────────────────────────────────────────────────>

  User        Action           Event            Store
  ────        ──────           ─────            ─────
  click   ->  ACT-navigate ->  EVT-route   ->  STR-router
  logo                         -changed         -store
                                   │              │
                                   │         state updated
                                   │              │
                                   ├──> pushState("/")
                                   │
                                   └──> analytics("page_view")
```

The entire flow is synchronous. The event is dispatched, the store reducer
runs, and side effects execute in order. There is no async gap between the
click and the navigation.

## Edge Cases

| Scenario                          | Behavior                          |
|-----------------------------------|-----------------------------------|
| Already on home page              | No-op; store sets same values     |
| During active search (loading)    | Navigates anyway; search aborted  |
| Rapid repeated clicks             | Each click dispatches; idempotent |
