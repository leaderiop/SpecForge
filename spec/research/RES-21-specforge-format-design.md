# RES-21: specforge format — CST-Based Code Formatter for .spec Files

> [!NOTE]
> **ID collision:** The RES-21 namespace is shared with the plugin runtime decision files (RES-21a through RES-21e in `spec/research/lua/`). This document covers the formatter; cite as RES-21-format to disambiguate.

> **Status:** active
> **Date:** 2026-03-03
> **Priority:** HIGH
> **Depends on:** RES-11a (core compiler architecture), RES-18 (AI agent token economics), RES-20 (LSP implementation review)
> **Tags:** formatting, developer-experience, cst, lsp, ci, ai-agents

---

## Executive Summary

SpecForge currently has 11 CLI commands but no code formatter. Inconsistent `.spec` file formatting creates noisy diffs, wastes AI agent tokens (RES-18 demonstrates 70-90% token reduction potential — inconsistent formatting undermines this by forcing agents to parse style variations), and slows code review. A `specforge format` command — analogous to `rustfmt`, `gofmt`, or `black` — would enforce consistent style with a CST-based formatter leveraging the existing tree-sitter grammar.

**Key findings:**

- **CST-based formatting** (not AST) is required to preserve comments, whitespace intent, and round-trip fidelity — tree-sitter already provides the CST via `tree_sitter_specforge`.
- **Opinionated by default** (like `gofmt` and `black`) with minimal configurability — reduces bikeshedding and maximizes consistency across projects.
- **Three integration points**: CLI (`specforge format`), CI (`specforge format --check`), and LSP (`textDocument/formatting`) — all sharing the same formatting engine.
- **Idempotency guarantee**: `format(format(x)) == format(x)` — a non-negotiable invariant that must be verified by property tests.
- **Performance budget**: <50ms for a single file, <500ms for a typical project (50-100 files) — fast enough for format-on-save in editors.

---

## 1. Problem Statement

### 1.1 Inconsistent Formatting in Practice

Without an official formatter, `.spec` files accumulate style inconsistencies over time:

| Inconsistency | Example | Impact |
|---------------|---------|--------|
| Indentation | Tabs vs. spaces, 2 vs. 4 spaces | Noisy diffs, merge conflicts |
| Trailing whitespace | Lines ending with spaces | CI lint noise |
| Blank lines | 0, 1, 2, or 3 blank lines between blocks | Visual inconsistency |
| Reference list wrapping | Inline vs. multi-line | Readability varies |
| Alignment | Field values aligned vs. ragged | Style debates |
| String formatting | Triple-quoted string indentation | Inconsistent dedent |
| Import ordering | Alphabetical vs. declaration order | Merge conflicts |

### 1.2 AI Agent Token Waste

RES-18 established that agents waste 60-80% of tokens on discovery and disambiguation. Inconsistent formatting compounds this waste:

- **Parsing overhead**: Inconsistent formatting forces the infrastructure to deliver suboptimal context — multiple style variations for the same semantic content increase noise for all consumers (Principle 6: infrastructure serves agents, not the reverse).
- **Example variations**: When producing `.spec` files, agents copy inconsistent examples, propagating more inconsistency.
- **Diff noise**: Style-only changes mixed with semantic changes force all consumers to distinguish meaningful from cosmetic diffs.
- **Context pollution**: Inconsistently formatted specs consume more tokens per unit of information (extra whitespace, varying indentation depths).

A canonical format eliminates these sources of waste entirely.

### 1.3 CI Pipeline Gap

Teams currently have no way to enforce formatting consistency in CI. Unlike `rustfmt --check` or `prettier --check`, there is no `specforge format --check` to fail a build on formatting violations. This is a missing quality gate.

### 1.4 Editor Experience Gap

RES-20 (LSP Implementation Review) identified format-on-save as a key missing LSP feature. Without `textDocument/formatting` support, developers must manually format files — which they inevitably do inconsistently.

---

## 2. Design Decisions

### 2.1 CST vs. AST Formatting

**Decision: CST-based formatting.**

| Approach | Comment preservation | Whitespace control | Complexity | Round-trip fidelity |
|----------|---------------------|--------------------|------------|---------------------|
| AST-based | Must re-attach comments (lossy) | Full rewrite (aggressive) | Lower | Poor — information loss |
| CST-based | Comments are tree nodes (lossless) | Modify existing tokens (precise) | Higher | Excellent — no information loss |

AST-based formatters (like early `rustfmt` prototypes) lose comments and whitespace intent because they reconstruct output from the abstract tree. CST-based formatters walk the concrete syntax tree, modifying only whitespace and indentation tokens while preserving everything else.

**Tree-sitter already provides a CST** via `tree_sitter_specforge`. The formatter walks the tree-sitter CST, applying formatting rules to whitespace tokens between significant nodes. This reuses the existing grammar investment and ensures the formatter always agrees with the parser on what constitutes valid syntax.

### 2.2 Configuration Philosophy

**Decision: Opinionated defaults with minimal configuration (like `gofmt` + `black`).**

The formatter should have fewer than 10 configuration options. Most formatting choices are **not configurable** — they are enforced rules. This follows the `gofmt` philosophy: "Gofmt's style is no one's favorite, yet gofmt is everyone's favorite."

Configurable options are limited to project-specific needs that genuinely vary across teams:

| Option | Default | Configurable? | Rationale |
|--------|---------|---------------|-----------|
| Indent width | 2 spaces | Yes | Some teams prefer 4 |
| Indent style | Spaces | Yes | Tab users exist |
| Max line width | 100 | Yes | Varies by team |
| Trailing newline | Yes | No | Universal best practice |
| Trailing comma | Yes | No | Cleaner diffs |
| Import sorting | Alphabetical | No | Deterministic |
| Blank lines between blocks | 1 | No | Visual consistency |
| Field alignment | Aligned | No | Readability |
| Reference list wrapping | Auto (>80 chars) | No | Deterministic |

**Non-configurable enforced rules** eliminate bikeshedding. If the formatter makes a choice, that choice is final and identical across all SpecForge projects.

### 2.3 Tree-sitter Reuse

**Decision: Build on `tree_sitter_specforge`, not a separate parser.**

The formatter reuses the same tree-sitter grammar used by the compiler and LSP. Benefits:

1. **Single source of truth**: No risk of parser/formatter disagreement on valid syntax.
2. **Error recovery**: Tree-sitter's error recovery means the formatter can handle partially broken files gracefully, formatting the valid portions.
3. **Incremental parsing**: Tree-sitter supports incremental re-parsing, which the LSP integration needs for format-on-type performance.
4. **Already maintained**: The grammar is actively maintained for the compiler — the formatter gets improvements for free.

### 2.4 Formatter Independence from Semantic Analysis

**Decision: The formatter operates purely on syntax — no name resolution, type checking, or graph construction.**

The formatter does NOT need:
- Import resolution (it sorts imports alphabetically by path string)
- Reference validation (it formats reference lists syntactically)
- Entity type information (it uses tree-sitter node types)

This means formatting is **fast** (no compilation required) and **independent** (can run on files with broken references).

---

## 3. Technical Architecture

### 3.1 Pipeline Overview

```
Input .spec file
    │
    ▼
┌─────────────────────┐
│  tree-sitter parse  │  → CST (concrete syntax tree)
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│   comment attacher   │  → CST with comment ownership resolved
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│    rule engine       │  → CST with formatting decisions applied
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│     CST printer      │  → Formatted output string
└─────────┬───────────┘
          │
          ▼
Output .spec file
```

### 3.2 Comment Attachment Algorithm

Comments are the hardest part of any formatter. The comment attacher assigns ownership of each comment to the nearest significant node using these rules:

**Rule 1: Leading comments attach to the following node.**
```
// This comment belongs to the behavior below
behavior validate_input "Validate Input" {
```

**Rule 2: Trailing comments attach to the preceding node on the same line.**
```
  risk high  // This comment belongs to the risk field
```

**Rule 3: Section header comments (lines starting with `// ──`) attach to the next block group.**
```
// ── Developer + CLI ──────────────────────────────────────────
```

**Rule 4: Blank-line-separated comment blocks are standalone.**
```
behavior foo "Foo" {
  ...
}

// This is a standalone comment between blocks
// It may span multiple lines

behavior bar "Bar" {
```

**Rule 5: Comments inside blocks attach to the next field or sub-block.**
```
behavior foo "Foo" {
  // This describes the invariants
  invariants [bar, baz]
}
```

### 3.3 Rule Engine

The rule engine is a visitor over the CST that emits formatting decisions:

```
enum FormatAction {
    /// Keep the existing whitespace
    Keep,
    /// Replace whitespace with exact content
    Replace(String),
    /// Insert whitespace where none exists
    Insert(String),
    /// Remove existing whitespace
    Remove,
}
```

Each node type in the tree-sitter grammar has a formatting rule. Rules are organized into modules:

| Module | Responsibility |
|--------|---------------|
| `indent` | Indentation levels and style |
| `spacing` | Spaces between tokens on a line |
| `newlines` | Blank lines between blocks and fields |
| `alignment` | Column alignment of field values |
| `wrapping` | Line wrapping for reference lists and long lines |
| `imports` | Import statement sorting and grouping |
| `strings` | Triple-quoted string indentation |
| `comments` | Comment formatting and placement |

### 3.4 CST Printer

The CST printer walks the tree with formatting decisions applied and emits the output string. It handles:

- **Indentation**: Emitting correct indent strings at line starts
- **Alignment**: Computing column widths for aligned fields
- **Wrapping**: Breaking long lines at appropriate points
- **Trailing content**: Ensuring trailing newline, removing trailing whitespace

---

## 4. Configuration Design

### 4.1 Configuration File

Configuration is stored in `.specforgefmt.toml` in the project root, following the convention of `rustfmt.toml`, `prettier.config.js`, and `pyproject.toml`.

```toml
# .specforgefmt.toml — SpecForge formatter configuration
# All options shown with their defaults. Only override what you need.

# Number of spaces per indentation level (2 or 4)
indent_width = 2

# Use tabs instead of spaces (true or false)
use_tabs = false

# Maximum line width before wrapping (60-120)
max_width = 100
```

### 4.2 Configuration Discovery

The formatter discovers configuration by walking up from the formatted file to the filesystem root:

1. Check for `.specforgefmt.toml` in the same directory as the file
2. Walk parent directories until a `.specforgefmt.toml` is found
3. Stop at the directory containing `specforge.spec` (project root)
4. If no config file found, use defaults

This matches the behavior of `rustfmt` (walks up to `rustfmt.toml`) and `prettier` (walks up to config file).

### 4.3 Configuration Validation

The formatter validates configuration on load:

| Option | Valid range | Default | Invalid behavior |
|--------|------------|---------|-----------------|
| `indent_width` | 2, 4 | 2 | Error: "indent_width must be 2 or 4" |
| `use_tabs` | true, false | false | Error: "use_tabs must be a boolean" |
| `max_width` | 60-120 | 100 | Error: "max_width must be between 60 and 120" |

Invalid configuration produces a diagnostic and the formatter falls back to defaults.

### 4.4 Inline Overrides

The formatter supports `// specforgefmt: off` and `// specforgefmt: on` comments to disable formatting for specific regions:

```
// specforgefmt: off
behavior legacy_format "Legacy Format" {
  types [
    VeryLongTypeName, AnotherLongTypeName,
    YetAnotherType, FinalType
  ]
}
// specforgefmt: on
```

This is a standard feature in formatters (`// rustfmt::skip`, `// prettier-ignore`, `// fmt: off`).

---

## 5. Command Line Interface

### 5.1 Command Syntax

```
specforge format [FILES...] [OPTIONS]

Arguments:
  [FILES...]    Files or directories to format (default: all .spec files in project)

Options:
  --check       Check formatting without modifying files (exit 1 if unformatted)
  --diff        Show a diff of formatting changes (implies --check)
  --stdin       Read from stdin, write to stdout
  --config      Path to .specforgefmt.toml (overrides discovery)
  --quiet       Suppress output except errors
```

### 5.2 Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All files formatted (or already formatted in --check mode) |
| 1 | Formatting changes needed (--check mode) or formatting errors |
| 2 | Invalid configuration or CLI arguments |

### 5.3 Output Behavior

**Default mode (no flags)**: Format files in-place, print names of changed files.
```
$ specforge format
  Formatted spec/behaviors/formatting.spec
  Formatted spec/features/formatting.spec
  2 files formatted, 52 files unchanged
```

**Check mode (`--check`)**: Report unformatted files, exit 1 if any found.
```
$ specforge format --check
  Would format spec/behaviors/formatting.spec
  Would format spec/features/formatting.spec
  2 files would be formatted
```

**Diff mode (`--diff`)**: Show unified diff of changes.
```
$ specforge format --diff
--- spec/behaviors/formatting.spec
+++ spec/behaviors/formatting.spec (formatted)
@@ -1,5 +1,5 @@
-behavior  validate_input "Validate Input" {
+behavior validate_input "Validate Input" {
   invariants [
-    string_interning_consistency,entity_id_uniqueness
+    string_interning_consistency, entity_id_uniqueness
   ]
```

**Stdin mode (`--stdin`)**: Read from stdin, write formatted output to stdout. Used by editor integrations that pipe buffer contents.

### 5.4 File Discovery

When no files are specified, the formatter discovers files:

1. Find project root (directory containing `specforge.spec`)
2. Recursively find all `*.spec` files under the project root
3. Respect `.gitignore` patterns (skip ignored files)
4. Skip files in `node_modules/`, `target/`, `.git/`

### 5.5 Parallel Formatting

Multiple files are formatted in parallel using Rayon. Each file is independently parseable and formattable, making this embarrassingly parallel. The formatter reports results as files complete.

---

## 6. LSP Integration

### 6.1 Supported LSP Methods

| Method | Purpose | Priority |
|--------|---------|----------|
| `textDocument/formatting` | Format entire document | P0 |
| `textDocument/rangeFormatting` | Format selected range | P1 |
| `textDocument/onTypeFormatting` | Format as user types | P2 (deferred) |

### 6.2 textDocument/formatting

When the user triggers format-document (or format-on-save):

1. LSP receives the full document text
2. LSP calls the formatting engine with the document text
3. Formatting engine returns a list of `TextEdit` operations
4. LSP sends `TextEdit[]` back to the editor
5. Editor applies the edits atomically

**Performance budget**: <50ms for a single file. This is fast enough for format-on-save to feel instantaneous.

### 6.3 textDocument/rangeFormatting

Range formatting requires additional logic:

1. Map the selected range to CST node boundaries
2. Expand the range to include complete blocks (formatting a partial block is undefined)
3. Format only the expanded range
4. Return `TextEdit[]` for the range

**Constraint**: Range formatting must produce the same result as full-document formatting for the affected blocks. This is a weaker form of idempotency scoped to blocks.

### 6.4 Editor Configuration Respect

The LSP formatting implementation respects editor-level settings:

- `editor.formatOnSave`: Triggers `textDocument/formatting` on save
- `editor.tabSize`: Overrides `indent_width` if no `.specforgefmt.toml` exists
- `editor.insertSpaces`: Overrides `use_tabs` if no `.specforgefmt.toml` exists

When `.specforgefmt.toml` exists, it takes precedence over editor settings.

### 6.5 Incremental Formatting

For `textDocument/onTypeFormatting` (P2, deferred), the formatter would use tree-sitter's incremental parsing to re-parse only the changed region and apply formatting rules to the affected nodes. This requires careful handling of comment re-attachment in the changed region.

---

## 7. Formatting Rules Catalog

### 7.1 Indentation

| Rule | Description | Example |
|------|-------------|---------|
| IND-01 | Top-level blocks at column 0 | `behavior foo "Foo" {` |
| IND-02 | Block body indented one level | `  invariants [bar, baz]` |
| IND-03 | Nested blocks indented one additional level | `  scenario "title" {\n    given "..."` |
| IND-04 | Multi-line reference lists indented one level from `[` | `  [\n    foo, bar,\n    baz,\n  ]` |
| IND-05 | Triple-quoted strings preserve relative indentation | Content indentation relative to opening `"""` is preserved |
| IND-06 | Continuation lines indented two levels from statement start | Long `method` lines wrap with double indent |

### 7.2 Spacing

| Rule | Description | Before | After |
|------|-------------|--------|-------|
| SPC-01 | Single space after entity keyword | `behavior  foo` | `behavior foo` |
| SPC-02 | Single space before opening brace | `"Title"{` | `"Title" {` |
| SPC-03 | No space before commas | `[foo , bar]` | `[foo, bar]` |
| SPC-04 | Single space after commas | `[foo,bar]` | `[foo, bar]` |
| SPC-05 | No trailing whitespace | `  risk high  ` | `  risk high` |
| SPC-06 | Single space around `->` in methods | `method foo()->` | `method foo() -> ` |
| SPC-07 | No space inside parentheses | `method foo( x: Y )` | `method foo(x: Y)` |
| SPC-08 | No space inside brackets in inline lists | `[ foo, bar ]` | `[foo, bar]` |

### 7.3 Alignment

| Rule | Description | Example |
|------|-------------|---------|
| ALN-01 | Field names aligned within a block | `family    core`<br>`features  [...]`<br>`depends_on [...]` |
| ALN-02 | Type annotations aligned in type blocks | `path    string`<br>`content string`<br>`checksum string @readonly` |
| ALN-03 | Port method arrows aligned | `method check(...)   -> Result<...>`<br>`method parse(...)   -> Result<...>` |
| ALN-04 | Alignment resets at block boundaries | Each block computes alignment independently |

### 7.4 Blank Lines

| Rule | Description |
|------|-------------|
| BLK-01 | Exactly one blank line between top-level blocks |
| BLK-02 | No blank lines between fields within a block (unless comments separate them) |
| BLK-03 | Exactly one blank line before section header comments (`// ──`) |
| BLK-04 | No trailing blank lines at end of file |
| BLK-05 | Exactly one trailing newline at end of file |
| BLK-06 | No more than one consecutive blank line anywhere |

### 7.5 Wrapping

| Rule | Description | Threshold |
|------|-------------|-----------|
| WRP-01 | Short reference lists stay inline | `[foo, bar, baz]` when ≤ max_width |
| WRP-02 | Long reference lists wrap to multi-line | One item per line when > max_width |
| WRP-03 | Multi-line reference lists have trailing comma | `[\n  foo,\n  bar,\n]` |
| WRP-04 | Method signatures wrap after `->` if too long | `method name(params)\n    -> Result<Type, Error>` |
| WRP-05 | Criteria lists always multi-line | Each criterion on its own line |
| WRP-06 | Payload blocks always multi-line | Each field on its own line |

### 7.6 Comments

| Rule | Description |
|------|-------------|
| CMT-01 | File-level comments at column 0 with `//` prefix |
| CMT-02 | Section headers use `// ──` pattern with consistent dash length |
| CMT-03 | Inline comments separated by two spaces from code |
| CMT-04 | Comment text has one space after `//` |
| CMT-05 | Multi-line comments maintain relative indentation |
| CMT-06 | Comments inside `// specforgefmt: off` regions are not modified |

### 7.7 Imports

| Rule | Description |
|------|-------------|
| IMP-01 | Imports sorted alphabetically by path |
| IMP-02 | Imports grouped by directory (blank line between groups) |
| IMP-03 | Selective imports sorted alphabetically: `use path { A, B, C }` |
| IMP-04 | Imports appear before any entity declarations |
| IMP-05 | Single blank line after import block |

### 7.8 Strings

| Rule | Description |
|------|-------------|
| STR-01 | Triple-quoted strings: opening `"""` on same line as field name |
| STR-02 | Triple-quoted strings: closing `"""` on its own line, aligned with field indent |
| STR-03 | Content indentation relative to the closing `"""` line is preserved |
| STR-04 | Single-line strings use double quotes, not triple quotes |
| STR-05 | Verify descriptions stay on one line (no wrapping) |

---

## 8. Implementation Strategy

### 8.1 Phase 1: Core Formatting Engine

**Goal**: Format individual files from the CLI.

**Deliverables**:
- CST walker with all formatting rules from Section 7
- Comment attachment algorithm
- `.specforgefmt.toml` loading and validation
- `specforge format [FILES...]` command
- Idempotency property tests

**Criteria**:
- `specforge format` runs on SpecForge's own `spec/` directory
- `format(format(x)) == format(x)` verified by property tests
- All comments preserved after formatting
- <50ms per file on typical hardware

### 8.2 Phase 2: CI Integration

**Goal**: Enable formatting checks in CI pipelines.

**Deliverables**:
- `--check` flag (exit 1 if unformatted)
- `--diff` flag (unified diff output)
- `--quiet` flag (suppress non-error output)
- File discovery with `.gitignore` respect
- Parallel formatting with Rayon

**Criteria**:
- `specforge format --check` exits 1 on unformatted files
- `specforge format --diff` produces valid unified diff output
- Parallel formatting scales linearly with file count
- Formatting a 100-file project completes in <500ms

### 8.3 Phase 3: LSP Formatting

**Goal**: Format-on-save and range formatting in editors.

**Deliverables**:
- `textDocument/formatting` handler in `specforge_lsp`
- `textDocument/rangeFormatting` handler
- Editor config respect (tab size, insert spaces)
- `--stdin` flag for external editor integration

**Criteria**:
- Format-on-save works in VS Code, Neovim, and Helix
- Range formatting produces same results as full formatting for affected blocks
- <50ms response time for `textDocument/formatting`

### 8.4 Phase 4: Advanced Features

**Goal**: Polish and edge case handling.

**Deliverables**:
- `// specforgefmt: off/on` inline overrides
- `textDocument/onTypeFormatting` (format-on-type)
- Incremental formatting (only re-format changed regions)
- Pre-commit hook integration guide

**Criteria**:
- Inline overrides correctly disable formatting for regions
- Format-on-type handles common patterns (closing brace, newline after field)
- Incremental formatting produces same results as full formatting

---

## 9. Prior Art Analysis

### 9.1 Comparison Matrix

| Feature | gofmt | rustfmt | black | prettier | specforge format |
|---------|-------|---------|-------|----------|-----------------|
| **Approach** | CST | AST → CST | CST | AST (Wadler-Lindig) | CST |
| **Configurability** | None | ~100 options | 5 options | ~20 options | 3 options |
| **Philosophy** | "One true format" | Flexible | "Uncompromising" | "Opinionated" | "Opinionated" |
| **Comment handling** | Excellent | Good (some edge cases) | Good | Good | Excellent (goal) |
| **CI integration** | `gofmt -l` | `rustfmt --check` | `black --check` | `prettier --check` | `--check` |
| **LSP support** | gopls | rust-analyzer | python-lsp | LSP extensions | specforge-lsp |
| **Range formatting** | No | Yes | No | Yes | Yes |
| **Inline disable** | No | `#[rustfmt::skip]` | `# fmt: off` | `// prettier-ignore` | `// specforgefmt: off` |
| **Speed** | ~1ms/file | ~10ms/file | ~30ms/file | ~50ms/file | <50ms/file (target) |

### 9.2 Lessons from Prior Art

**From `gofmt`**: Zero configuration is the ideal. The most impactful design choice `gofmt` made was having no options. "Gofmt's style is no one's favorite, yet gofmt is everyone's favorite." We adopt this philosophy but allow 3 options for genuine project-level needs (indent width, tabs, line width).

**From `rustfmt`**: Too many options lead to fragmentation. With ~100 options, the Rust ecosystem has `rustfmt` configurations that differ between projects, partially defeating the purpose. Our 3-option limit avoids this.

**From `black`**: Marketing matters. Black's tagline "The uncompromising Python code formatter" set clear expectations. Users accept the style because they know it is intentional and non-negotiable. We adopt "opinionated" language.

**From `prettier`**: The Wadler-Lindig algorithm (IR-based pretty printing) is powerful but complex. For `.spec` files — which have simpler structure than JavaScript/TypeScript — a direct CST walker is sufficient and easier to maintain.

**From all**: Idempotency is non-negotiable. Every major formatter guarantees `format(format(x)) == format(x)`. Violations are treated as P0 bugs.

### 9.3 Key Differentiators

SpecForge's formatter has unique advantages:

1. **Simple language**: `.spec` files have ~16 block types with regular structure — much simpler than general-purpose languages. This means fewer edge cases and higher confidence in formatting correctness.
2. **Single grammar**: The formatter shares the exact same tree-sitter grammar as the compiler and LSP. No separate parsing infrastructure.
3. **No expression formatting**: `.spec` files have no arithmetic expressions, control flow, or complex nesting. The hardest formatting problems in other languages (chained method calls, nested ternaries, complex generics) do not exist.
4. **AI-first**: The formatter's primary value proposition is enabling consistent, token-efficient specs for AI agents — a use case that did not exist when `gofmt` was designed.

---

## 10. Success Metrics

### 10.1 Correctness Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Idempotency | 100% | `format(format(x)) == format(x)` for all valid inputs |
| Comment preservation | 100% | No comments lost or relocated incorrectly |
| Semantic preservation | 100% | Formatted files produce identical compiler graph |
| Round-trip fidelity | 100% | `parse(format(parse(x)))` produces same AST as `parse(x)` |

### 10.2 Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Single file | <50ms | Wall clock time for formatting one file |
| Project (50 files) | <500ms | Wall clock time for formatting a typical project |
| Project (100 files) | <1s | Wall clock time for formatting a large project |
| LSP response | <50ms | Time to respond to `textDocument/formatting` |
| Memory | <50MB | Peak memory for formatting a 100-file project |

### 10.3 Adoption Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| SpecForge dogfooding | 100% | All files in `spec/` pass `specforge format --check` |
| CI adoption | >80% | Projects with `specforge format --check` in CI |
| Format-on-save | >50% | LSP users with format-on-save enabled |
| Zero config | >90% | Projects using default configuration |

### 10.4 Quality Gate: Idempotency Verification

Idempotency is verified at multiple levels:

1. **Unit tests**: Each formatting rule is tested for idempotency on sample inputs.
2. **Property tests**: Random valid `.spec` files are generated and verified: `format(format(x)) == format(x)`.
3. **Dogfooding**: SpecForge's own `spec/` directory is formatted and verified in CI.
4. **Regression tests**: Any reported idempotency violation becomes a permanent test case.

---

## 11. Risk Analysis

### 11.1 Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Comment attachment edge cases | High | Medium | Comprehensive test suite, fuzzing, community bug reports |
| Tree-sitter version incompatibility | Low | High | Pin tree-sitter version, test against grammar changes |
| Performance regression on large files | Low | Medium | Benchmark suite, profiling, streaming printer |
| Alignment instability (oscillating format) | Medium | High | Idempotency property tests, careful alignment algorithm |
| Conflict with editor auto-formatters | Low | Low | Document recommended editor settings |

### 11.2 Adoption Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Users resist style choices | Medium | Medium | "Opinionated" messaging, minimal config options |
| Formatter changes require mass reformatting | Medium | Medium | Versioned formatter output, migration guide |
| LSP formatting conflicts with plugins | Low | Low | Respect editor settings hierarchy |

---

## 12. Relationship to Other Research

- **RES-11a** (Core Compiler): The formatter builds on the same tree-sitter grammar and CST infrastructure designed for the compiler pipeline.
- **RES-18** (Token Economics): Consistent formatting directly reduces AI agent token waste by eliminating style variation from the context budget.
- **RES-20** (LSP Review): LSP formatting was identified as a missing feature; this research provides the design for it.

---

## References

1. Go Team. "go fmt command." Go Documentation. https://pkg.go.dev/cmd/gofmt
2. Rust Team. "rustfmt — A tool for formatting Rust code." https://rust-lang.github.io/rustfmt/
3. Ambv, Łukasz Langa. "Black: The uncompromising Python code formatter." https://black.readthedocs.io/
4. Prettier Team. "Prettier — Opinionated Code Formatter." https://prettier.io/
5. Wadler, Philip. "A prettier printer." Journal of Functional Programming, 2003.
6. Lindig, Christian. "Strictly Pretty." Technical Report, 2000.
7. Maxbrunsfeld et al. "Tree-sitter — An incremental parsing system." https://tree-sitter.github.io/
8. Vijayvargiya et al. "Ambig-SWE: Resolving Ambiguities in LLM-Based Code Generation." ICLR 2026.
9. GitClear. "Coding on Copilot: 2023 Data Suggests Downward Pressure on Code Quality." 2024.
