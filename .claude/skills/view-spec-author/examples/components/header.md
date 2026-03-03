# Header

> component | `CMP-header`

## Wireframe

```
┌──────────────────────────────────────────────────────────────┐
│ CMP-header                                    height: 64px   │
│ padding: 0 24px                                              │
│ display: flex                                                │
│ align-items: center                                          │
│                                                              │
│  ┌────────────────┐                                          │
│  │   ELM-logo     │                                          │
│  │   120 x 40     │                                          │
│  │   cursor: ptr  │                                          │
│  └────────────────┘                                          │
│                                                              │
│  (remaining space intentionally empty)                       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Props

| Name    | Type     | Required | Default        | Description                     |
|---------|----------|----------|----------------|---------------------------------|
| `title` | `string` | no       | `"Search App"` | App title, used for a11y labels |

The `title` prop is not rendered visually in the current design. It is passed
to the logo element as an accessible label and may appear in a future mobile
hamburger menu or breadcrumb.

## Children

```
CMP-header
 └─ ELM-logo         click --> ACT-navigate
```

The header contains a single child element: the logo. The logo is left-aligned
and acts as a navigation link back to the home route.

## Store Binding

```
  STR-router-store
       │
       │  selector: currentPath
       │
       ▼
  CMP-header.activePath   (local prop)
```

| Store              | Selector      | Maps To      | Purpose                            |
|--------------------|---------------|--------------|------------------------------------|
| `STR-router-store` | `currentPath` | `activePath` | Enables active-route styling       |

The header subscribes to the router store's `currentPath` selector. The value
is mapped to a local `activePath` prop, which could be used to highlight the
active navigation item if the header grows to include a nav bar.

## Responsive Visibility

| Viewport | Visible | Notes                             |
|----------|---------|-----------------------------------|
| Desktop  | yes     | Full 64px height, 24px padding    |
| Tablet   | yes     | Reduced to 56px height            |
| Mobile   | yes     | Reduced to 48px height, 16px pad  |

The header is always visible across all breakpoints. Only its dimensions adapt.

## Design Notes

The header uses a minimal left-aligned logo pattern. There is no navigation
menu, search within header, or user avatar. This is intentional: the app is
a single-page search tool, so the header serves only as a brand anchor and
home-navigation affordance.

A bottom border (`1px solid #DADCE0`) visually separates the header from the
content area without adding visual weight.
