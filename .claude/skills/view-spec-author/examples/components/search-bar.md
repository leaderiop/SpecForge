# Search Bar

> component | `CMP-search-bar`

## Wireframe

```
Desktop / Tablet:
┌──────────────────────────────────────────────────────────────┐
│ CMP-search-bar                          max-width: 584px     │
│ display: flex                           gap: 8px             │
│ align-items: center                                          │
│                                                              │
│  ┌────────────────────────────────────────┐  ┌────────────┐  │
│  │ ELM-search-input                       │  │ ELM-search │  │
│  │                                        │  │  -button   │  │
│  │ placeholder: "Type to search..."       │  │  "Search"  │  │
│  │ flex: 1                                │  │  auto-w    │  │
│  └────────────────────────────────────────┘  └────────────┘  │
│                                                              │
└──────────────────────────────────────────────────────────────┘

Mobile (stacked):
┌──────────────────────────────┐
│ CMP-search-bar               │
│ flex-direction: column       │
│ gap: 12px                    │
│                              │
│  ┌──────────────────────────┐│
│  │ ELM-search-input         ││
│  │ width: 100%              ││
│  └──────────────────────────┘│
│  ┌──────────────────────────┐│
│  │ ELM-search-button        ││
│  │ width: 100%              ││
│  └──────────────────────────┘│
│                              │
└──────────────────────────────┘
```

## Props

| Name          | Type     | Required | Default              | Description                        |
|---------------|----------|----------|----------------------|------------------------------------|
| `placeholder` | `string` | no       | `"Type to search..."` | Placeholder text for search input  |
| `maxLength`   | `number` | no       | `256`                | Maximum character length for input |

Props are forwarded to the `ELM-search-input` child. The `maxLength` prop
both sets the HTML `maxlength` attribute and drives the validation rule.

## Children

```
CMP-search-bar
 ├─ ELM-search-input     submit/change --> ACT-submit-search
 └─ ELM-search-button    click         --> ACT-submit-search
```

Both children trigger the same action (`ACT-submit-search`) through different
interactions. The input fires on `submit` (Enter key) and `change` (debounced),
while the button fires on `click`.

## Store Bindings

```
  STR-search-store
       │
       ├── selector: query       -->  searchValue  (two-way)
       │
       └── selector: isLoading   -->  loading      (read-only)
       │
       ▼
  CMP-search-bar
       │
       ├── searchValue  -->  ELM-search-input.value
       └── loading      -->  ELM-search-button.disabled
                              ELM-search-input.state = "loading"
```

| Store              | Selector    | Maps To       | Direction | Purpose                          |
|--------------------|-------------|---------------|-----------|----------------------------------|
| `STR-search-store` | `query`     | `searchValue` | two-way   | Syncs input value with store     |
| `STR-search-store` | `isLoading` | `loading`     | read-only | Disables controls during search  |

When `isLoading` is `true`, the input shows a loading state and the button
enters its `loading` state (reduced opacity + wait cursor). Both become
non-interactive until the API call resolves.

## Responsive Visibility

| Viewport | Visible | Layout     |
|----------|---------|------------|
| Desktop  | yes     | horizontal |
| Tablet   | yes     | horizontal |
| Mobile   | yes     | vertical   |

On mobile (< 768px), the flex direction switches to `column` so the input and
button stack vertically, each taking full width for comfortable touch targets
(minimum 48px height).
