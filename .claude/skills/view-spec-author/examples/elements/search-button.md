# Search Button

> element | `ELM-search-button`

## Wireframe

```
Default:
┌─────────────────┐
│     Search       │  bg: #1A73E8, color: #FFF
│                  │  border-radius: 24px
└─────────────────┘  padding: 12px 24px, font-weight: 600

Hover:
┌─────────────────┐
│     Search       │  bg: #1557B0
│                  │
└─────────────────┘

Active (pressed):
┌─────────────────┐
│     Search       │  bg: #174EA6
│                  │
└─────────────────┘

Loading:
┌─────────────────┐
│   [...]  Search  │  bg: #1A73E8, opacity: 0.7
│                  │  cursor: wait
└─────────────────┘

Disabled:
┌ ─ ─ ─ ─ ─ ─ ─ ┐
  Search              bg: #DADCE0, color: #80868B
│                 │   cursor: not-allowed
└ ─ ─ ─ ─ ─ ─ ─ ┘
```

## State Diagram

```
                    ┌───────────┐
         ┌─────────│  DEFAULT   │<─────────┐
         │         │            │          │
         │         │ bg:#1A73E8 │          │
         │         │ #FFF text  │          │
         │         └─────┬──────┘          │
         │               │                 │
    mouse-leave     mouse-enter        mouse-up / blur
         │               │                 │
         │               ▼                 │
         │         ┌───────────┐           │
         ├─────────│   HOVER   │───────────┤
         │         │            │          │
         │         │ bg:#1557B0 │          │
         │         └─────┬──────┘          │
         │               │                 │
         │          mouse-down             │
         │               │                 │
         │               ▼                 │
         │         ┌───────────┐           │
         │         │  ACTIVE   │───────────┘
         │         │            │
         │         │ bg:#174EA6 │
         │         └─────┬──────┘
         │               │
         │           mouse-up
         │           (click fires ACT-submit-search)
         │               │
         │               ▼
         │         ┌───────────┐
         │         │  LOADING  │
         │         │            │
         │         │ opacity:0.7│
         │         │ cursor:wait│
         │         └─────┬──────┘
         │               │
         │          API resolves
         │               │
         │               ▼
         │         ┌───────────┐
         └─────────│  DEFAULT  │
                   └───────────┘


                   ┌───────────┐
                   │ DISABLED  │   (query is empty or loading)
                   │            │
                   │ bg:#DADCE0 │
                   │ #80868B   │
                   │ no-click  │
                   └───────────┘
```

State transitions:
- DEFAULT -> HOVER -> ACTIVE -> LOADING -> DEFAULT is the happy path.
- DISABLED is entered when `STR-search-store.isLoading` is `true` or the
  input query is empty. Click events are suppressed in this state.
- LOADING is entered after a successful click and exits when the API call
  resolves (success or failure).

## Action Binding

| Trigger | Action              | Event Dispatched        |
|---------|---------------------|-------------------------|
| `click` | `ACT-submit-search` | `EVT-search-submitted`  |

The button is a secondary trigger for the same action the input fires on
submit. It exists as an explicit affordance for users who prefer clicking
over pressing Enter.

## Store Binding

| Store              | Field       | Direction | Purpose                          |
|--------------------|-------------|-----------|----------------------------------|
| `STR-search-store` | `isLoading` | read-only | Controls loading/disabled states  |

When `isLoading` is `true`, the button renders in its `loading` state. The
button does not write to the store directly; it delegates through the action.

## Accessibility

| Property   | Value             |
|------------|-------------------|
| `role`     | `button`          |
| `label`    | `"Submit search"` |

### Keyboard Navigation

| Key     | Action              | Notes                                      |
|---------|---------------------|--------------------------------------------|
| `Tab`   | Focus button        | Follows search input in tab order          |
| `Enter` | `ACT-submit-search` | Same as click                              |
| `Space` | `ACT-submit-search` | Same as click (native button behavior)     |

### Focus Ring

On keyboard focus, the button displays a 2px outline offset by 2px in the
`primary` color (`#1A73E8`). This is distinct from the hover state to ensure
keyboard users can distinguish focus from pointer hover.

### Disabled State Announcement

When the button is disabled, screen readers announce "Submit search, button,
dimmed" (or equivalent). The `aria-disabled="true"` attribute is preferred
over the HTML `disabled` attribute so the button remains focusable and
discoverable.
