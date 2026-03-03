# Logo

> element | `ELM-logo`

## Wireframe

```
Default state:
┌────────────────────┐
│                    │
│   SEARCH APP       │   120 x 40 px
│   [logo image]     │   cursor: pointer
│                    │
└────────────────────┘

Hover state:
┌────────────────────┐
│ ░░░░░░░░░░░░░░░░░░ │
│ ░░ SEARCH APP ░░░░ │   opacity: 0.8
│ ░░ [logo image]░░░ │   cursor: pointer
│ ░░░░░░░░░░░░░░░░░░ │
└────────────────────┘
```

## State Diagram

```
                ┌─────────────┐
                │   DEFAULT   │
                │             │
                │ width: 120  │
                │ height: 40  │
                │ cursor: ptr │
                └──────┬──────┘
                       │
              mouse-enter / focus
                       │
                       ▼
                ┌─────────────┐
                │    HOVER    │
                │             │
                │ opacity:0.8 │
                │ cursor: ptr │
                └──────┬──────┘
                       │
              mouse-leave / blur
                       │
                       ▼
                ┌─────────────┐
                │   DEFAULT   │
                └─────────────┘
```

The logo has only two visual states. There is no disabled, loading, or error
state because the logo is always available as a navigation element.

## Action Binding

| Trigger    | Action          | Event Dispatched     |
|------------|-----------------|----------------------|
| `click`    | `ACT-navigate`  | `EVT-route-changed`  |

Clicking the logo dispatches a navigation action that routes the user back to
the home page (`/`). This is a no-op if the user is already on the home page
(the router store's reducer will set `previousPath` to the same value).

## Accessibility

| Property   | Value                                  |
|------------|----------------------------------------|
| `role`     | `img`                                  |
| `label`    | `"Search App logo - navigate to home"` |

### Keyboard Navigation

| Key     | Action         | Notes                                    |
|---------|----------------|------------------------------------------|
| `Tab`   | Focus logo     | Logo is in the natural tab order         |
| `Enter` | `ACT-navigate` | Same as click; triggers route change     |

The logo must be wrapped in a focusable container (e.g., `<a>` or element with
`tabindex="0"`) so keyboard users can reach it. The `role="img"` with an
explicit label ensures screen readers announce the element's purpose rather than
just "image".

### Contrast

The logo image should maintain a minimum 3:1 contrast ratio against the header
background (`#FFFFFF`) per WCAG 2.1 SC 1.4.11 (Non-text Contrast).
