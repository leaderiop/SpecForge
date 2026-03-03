# Search Input

> element | `ELM-search-input`

## Wireframe

```
Default:
┌──────────────────────────────────────────────┐
│ Type to search...                             │  border: 1px solid #DADCE0
│                                               │  border-radius: 24px
└──────────────────────────────────────────────┘  padding: 12px 16px

Focused:
╔══════════════════════════════════════════════╗
║ |                                            ║  border: 1px solid #1A73E8
║                                              ║  box-shadow: 0 0 0 2px
╚══════════════════════════════════════════════╝    rgba(26,115,232,0.2)

Error:
┌──────────────────────────────────────────────┐
│ This query is way too long and exceeds the   │  border: 1px solid #D93025
│                                              │  box-shadow: 0 0 0 2px
└──────────────────────────────────────────────┘    rgba(217,48,37,0.2)
  "Search query must be 256 characters or less"

Loading:
┌──────────────────────────────────────────────┐
│ weather forecast                    [...]    │  bg: #F8F9FA
│                                              │  cursor: wait
└──────────────────────────────────────────────┘

Disabled:
┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
  (not available)                                 bg: #F1F3F4
│                                              │  color: #80868B
└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘  cursor: not-allowed
```

## State Diagram

```
                    ┌───────────┐
          ┌────────>│  DEFAULT  │<────────┐
          │         │           │         │
          │         │ #DADCE0   │         │
          │         └─────┬─────┘         │
          │               │               │
     mouse-leave     mouse-enter      blur/resolve
          │               │               │
          │               ▼               │
          │         ┌───────────┐         │
          ├─────────│   HOVER   │─────────┤
          │         │           │         │
          │         │ #B0B3B8   │         │
          │         └─────┬─────┘         │
          │               │               │
          │            focus              │
          │               │               │
          │               ▼               │
          │         ┌───────────┐         │
          │         │  FOCUSED  │─────────┘
          │         │           │
          │         │ #1A73E8   │
          │         │ + shadow  │
          │         └─────┬─────┘
          │               │
          │          validation
          │           fails
          │               │
          │               ▼
          │         ┌───────────┐
          │         │   ERROR   │
          │         │           │
          │         │ #D93025   │
          │         │ + shadow  │
          │         └─────┬─────┘
          │               │
          │          fix input
          │               │
          │               ▼
          │         ┌───────────┐
          │         │  FOCUSED  │
          │         └───────────┘
          │
          │
          │         ┌───────────┐            ┌───────────┐
          └─────────│  LOADING  │<───────────│ (submit)  │
                    │           │            └───────────┘
                    │ #F8F9FA   │
                    │ wait      │
                    └─────┬─────┘
                          │
                     API resolves
                          │
                          ▼
                    ┌───────────┐
                    │  DEFAULT  │
                    └───────────┘


                    ┌───────────┐
                    │ DISABLED  │   (set externally)
                    │           │
                    │ #F1F3F4   │
                    │ no-input  │
                    └───────────┘
```

## Validation Rules

| Rule         | Value | Message                                       | Trigger    |
|--------------|-------|-----------------------------------------------|------------|
| `max-length` | `256` | "Search query must be 256 characters or less" | on change  |

Validation runs on every keystroke. When the input exceeds 256 characters,
the element transitions to the `error` state and the error message appears
below the input. The `ACT-submit-search` action is blocked while in error
state because its precondition (`query.trim().length > 0`) implicitly
requires valid input.

## Action Bindings

| Trigger  | Action              | Notes                              |
|----------|---------------------|------------------------------------|
| `submit` | `ACT-submit-search` | Fires on Enter key                 |
| `change` | `ACT-submit-search` | Debounced (300ms, trailing edge)   |

## Store Binding

| Store              | Field   | Direction | Notes                            |
|--------------------|---------|-----------|----------------------------------|
| `STR-search-store` | `query` | two-way   | Input value syncs with store     |

The input is a controlled element. Its displayed value always reflects
`STR-search-store.query`. User keystrokes dispatch store updates, which
flow back as the new value (unidirectional data flow with two-way binding
semantics).

## Accessibility

| Property   | Value            |
|------------|------------------|
| `role`     | `searchbox`      |
| `label`    | `"Search input"` |

### Keyboard Navigation

| Key      | Action              | Notes                                    |
|----------|---------------------|------------------------------------------|
| `Tab`    | Focus input         | Natural tab order after logo             |
| `Enter`  | `ACT-submit-search` | Submits current query                    |
| `Escape` | `ACT-navigate`      | Clears focus, navigates home             |

### Screen Reader Announcements

- On focus: "Search input, searchbox"
- On error: "Search query must be 256 characters or less, invalid"
- On loading: "Searching, please wait"

The `searchbox` role (rather than generic `textbox`) tells assistive
technology that this input is specifically for search, enabling search-specific
behaviors in some screen readers.
