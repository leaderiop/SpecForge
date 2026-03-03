# Flux Pattern for View Specs

The View Spec system follows the **Flux unidirectional data flow** pattern. Every user interaction must complete the full cycle.

## The Cycle

```
                         USER INTERACTION
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│                        Element (ELM)                         │
│  - Renders current state from store binding                  │
│  - Captures user interaction (click, submit, change, etc.)   │
│  - Triggers an Action                                        │
└──────────────────────┬───────────────────────────────────────┘
                       │ triggers
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                        Action (ACT)                          │
│  - Receives trigger from element                             │
│  - Checks preconditions (block/warn/redirect on failure)     │
│  - May apply debounce/throttle                               │
│  - Dispatches one or more Events                             │
└──────────────────────┬───────────────────────────────────────┘
                       │ dispatches
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                        Event (EVT)                           │
│  - Carries payload data                                      │
│  - Targets one or more Stores                                │
│  - May trigger side effects (API, navigation, notification)  │
│  - May chain to success/failure events                       │
└──────────────────────┬───────────────────────────────────────┘
                       │ updates
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                        Store (STR)                           │
│  - Reduces event into state mutations                        │
│  - Exposes selectors for derived state                       │
│  - Notifies consumer Components                              │
│  - May persist state to storage                              │
└──────────────────────┬───────────────────────────────────────┘
                       │ notifies
                       ▼
┌──────────────────────────────────────────────────────────────┐
│                      Component (CMP)                         │
│  - Subscribes to store selectors                             │
│  - Maps selector values to props                             │
│  - Re-renders child elements with updated state              │
│  - Cycle begins again when user interacts                    │
└──────────────────────────────────────────────────────────────┘
```

## Entity Mapping

| Flux Concept | Entity | Responsibility |
|--------------|--------|----------------|
| View | Element (`ELM`) | Renders UI, captures interactions |
| Action | Action (`ACT`) | Validates and dispatches |
| Dispatcher | Event (`EVT`) | Carries payload, targets stores |
| Store | Store (`STR`) | Manages state, notifies subscribers |
| Container | Component (`CMP`) | Binds stores to elements |
| Layout | Page (`PG`) | Routes, composes components |
| App Shell | Wireframe (`WF`) | Viewports, theme, page list |

## Cycle Enforcement Rules

### Rule 1: Elements Must Trigger Actions

Every interactive element must have at least one action binding:

```yaml
# Element
actions:
  - trigger: click
    action: { $ref: ACT-submit-search }
```

An element without actions is either:
- A **display-only** element (text, image) — no action needed
- **Missing its action** — add it

### Rule 2: Actions Must Dispatch Events

Every action must dispatch at least one event:

```yaml
# Action
events-dispatched:
  - $ref: EVT-search-submitted
```

An action with empty `events-dispatched` is incomplete.

### Rule 3: Events Must Target Stores

Every event must target at least one store:

```yaml
# Event
target-stores:
  - $ref: STR-search-store
```

An event with empty `target-stores` is incomplete.

### Rule 4: Stores Must Have Consumers

Every store should have at least one consumer (component or element):

```yaml
# Store
consumers:
  - $ref: CMP-search-bar
```

A store with no consumers holds dead state.

### Rule 5: Components Must Subscribe to Stores

Components that display dynamic data must have store bindings:

```yaml
# Component
stores:
  - $ref: STR-search-store
    binds:
      - selector: query
        to: searchValue
```

### Rule 6: No Reverse Flow

Data flows **one way** through the cycle:

```
ELM → ACT → EVT → STR → CMP → ELM (render)
```

- Elements do NOT directly update stores
- Actions do NOT directly modify state
- Events do NOT directly re-render components
- Stores do NOT directly trigger actions

## Tracing a Complete Cycle

Example: User types in a search box and presses Enter.

```
1. ELM-search-input    → user types "hello"
2. ACT-submit-search   → triggered by submit, checks non-empty
3. EVT-search-submitted → payload: { query: "hello" }
4. STR-search-store    → reduces: set query to "hello", set loading to true
5. CMP-search-bar      → re-renders with query="hello", loading=true
6. ELM-search-input    → displays "hello" in input
```

## Side Effect Chains

Events can trigger side effects that produce new events:

```
EVT-search-submitted
  ├── target-stores: STR-search-store (set loading: true)
  ├── side-effect: api-call GET /search?q=hello
  │   ├── on-success: EVT-search-results-received
  │   └── on-failure: EVT-search-failed
  │
EVT-search-results-received
  └── target-stores: STR-search-store (set results, set loading: false)

EVT-search-failed
  └── target-stores: STR-search-store (set error, set loading: false)
```

Each event in the chain must also complete the cycle (target stores, stores notify consumers).

## Verification Checklist

| Check | Rule | How to Verify |
|-------|------|---------------|
| Every interactive ELM has actions | Rule 1 | `actions` array is non-empty |
| Every ACT dispatches events | Rule 2 | `events-dispatched` array is non-empty |
| Every EVT targets stores | Rule 3 | `target-stores` array is non-empty |
| Every STR has consumers | Rule 4 | `consumers` array is non-empty |
| Every CMP subscribes to stores | Rule 5 | `stores` array has bindings (for dynamic data) |
| No reverse flow | Rule 6 | No direct store writes from elements |
