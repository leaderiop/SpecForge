# Understand Diagnostics

SpecForge validates your specs in real time and reports issues as you type. The same check runs on every edit, whether you typed it or an AI agent made it, so a change that breaks a rule never slips through. Every diagnostic has a unique code that tells you exactly what went wrong.

Some checks are structural (a name that points nowhere). The rest come from your extensions: each one brings the rules for the kinds it declares.

## Diagnostic severity levels

| Prefix | Severity | Meaning |
|--------|----------|---------|
| **E** | Error | Must fix -- the graph cannot be built correctly |
| **W** | Warning | Should fix -- the spec is valid but likely incorrect |
| **I** | Info | Consider fixing -- best practice suggestion |

## Common diagnostics

### Errors you will see first

- **E001** -- Syntax error in the spec file
- **E002** -- Duplicate entity ID
- **E003** -- Unresolved reference (the target entity does not exist)
- **E006** -- Missing required field

### Warnings to watch for

- **W001** -- Behavior implements no feature (with `@specforge/product`)
- **W003** -- Invariant that no behavior enforces
- **W061** -- Reference cycle detected

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

## Info-level advice

SpecForge always reports info-level diagnostics (I-codes) next to errors and warnings: best practice checks like missing descriptions, orphaned modules, and effort estimation gaps. They are suggestions, not problems.

On the command line, `specforge check --severity info` shows only them (or `--severity error`, `--severity warning`); the exit code still counts everything reported.
