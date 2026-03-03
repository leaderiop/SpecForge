# Home Page

> page | `PG-home`

## Wireframe

```
┌──────────────────────────────────────────────────────────────┐
│ /                                        single-column       │
│                                                              │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ CMP-header                                               │ │
│ │  ┌────────────┐                                          │ │
│ │  │  ELM-logo  │                                          │ │
│ │  └────────────┘                                          │ │
│ └──────────────────────────────────────────────────────────┘ │
│                                                              │
│                       ~ spacer ~                             │
│                                                              │
│              ┌────────────────────────────┐                  │
│              │ CMP-search-bar             │                  │
│              │                            │                  │
│              │ ┌──────────────┐ ┌──────┐  │                  │
│              │ │ELM-search-   │ │ELM-  │  │                  │
│              │ │input         │ │search│  │                  │
│              │ │              │ │button│  │                  │
│              │ └──────────────┘ └──────┘  │                  │
│              │                            │                  │
│              └────────────────────────────┘                  │
│                                                              │
│                       ~ spacer ~                             │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Route Configuration

| Property  | Value                          | Notes                                       |
|-----------|--------------------------------|---------------------------------------------|
| `path`    | `/`                            | Root route, default entry point              |
| `params`  | `[]`                           | No path parameters                           |
| `query`   | `q: string` (default `""`)     | Pre-populates search input from URL          |
| `hash`    | `false`                        | No hash fragment routing                     |
| `guard`   | `null`                         | No authentication or authorization required  |

The `q` query parameter allows deep-linking to a search. When the page loads
with `/?q=hello`, the search-store initializes its `query` field to `"hello"`
and immediately triggers `ACT-submit-search`. This enables bookmarkable and
shareable search URLs.

## Layout

The page uses a `single-column` layout. This means all components stack
vertically in source order within a single centered column. No sidebar, no
grid. This keeps the search-focused experience distraction-free.

## Component Placement

```
 ┌───────────────────────────────┐
 │  1. CMP-header                │  top: 0, sticky
 │     - full width              │
 │     - height: 64px desktop    │
 ├───────────────────────────────┤
 │  2. CMP-search-bar            │  vertically centered
 │     - max-width: 584px        │  in remaining viewport
 │     - horizontally centered   │
 └───────────────────────────────┘
```

The header is position-sticky at the top. The search bar is placed in the
vertical center of the remaining viewport space using flexbox
(`justify-content: center` on the main content area).

## Store Bindings

| Store              | Purpose                                        |
|--------------------|------------------------------------------------|
| `STR-router-store` | Tracks current path; header reads `currentPath` |
| `STR-search-store` | Holds query + results; search bar reads/writes  |

Both stores are scoped to this page. They initialize when the page mounts and
remain in memory until the page unmounts.

## SEO Meta

| Tag            | Value                                    |
|----------------|------------------------------------------|
| `<title>`      | `Search App - Home`                      |
| `<meta desc>`  | `Search anything with the Search App`    |
