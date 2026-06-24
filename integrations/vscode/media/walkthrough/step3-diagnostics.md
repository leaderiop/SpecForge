# Understand Diagnostics

SpecForge validates your specs in real time and reports issues as you type. Every diagnostic has a unique code that tells you exactly what went wrong.

## Diagnostic severity levels

| Prefix | Severity | Meaning |
|--------|----------|---------|
| **E** | Error | Must fix -- the graph cannot be built correctly |
| **W** | Warning | Should fix -- the spec is valid but likely incorrect |
| **I** | Info | Consider fixing -- best practice suggestion (pedantic profile only) |

## Common diagnostics

### Errors you will see first

- **E001** -- Syntax error in the spec file
- **E004** -- Unknown entity kind (check your extensions)
- **E006** -- Duplicate entity ID in the same scope
- **E010** -- Unresolved reference (the target entity does not exist)

### Warnings to watch for

- **W001** -- Entity has no references (orphan in the graph)
- **W003** -- Missing recommended field
- **W010** -- Circular dependency detected

## Using diagnostics

### In the editor

Errors appear as **red squiggles**, warnings as **yellow squiggles**. Hover over them to see the full message and diagnostic code.

### Quick fixes

Click the **$(lightbulb) lightbulb** icon that appears next to diagnostics. Common quick fixes include:

- Create a missing entity that is referenced
- Add a missing required field
- Fix a typo in an entity reference (fuzzy match suggestions)

### Problems panel

Open **View > Problems** (or press `Ctrl+Shift+M` / `Cmd+Shift+M`) to see all diagnostics across your project.

### Explain a diagnostic

Need more detail? Use the Command Palette:

1. Press `Ctrl+Shift+P` / `Cmd+Shift+P`
2. Run **SpecForge: Explain Error**
3. Enter the code (e.g., `E001`)

Or from the terminal:
```bash
specforge explain E001
```

This shows the full explanation with examples and suggested fixes.

## Diagnostic profiles

By default, SpecForge shows only errors and warnings. To also see informational diagnostics, change the lint profile in your settings:

- Open Settings and search for `specforge.lint.profile`
- Change from `default` to `pedantic`

Pedantic mode enables info-level diagnostics (I-codes) for best practice checks like missing descriptions, orphaned modules, and effort estimation gaps.
