---
name: view-spec-author
description: Author View Specifications in YAML following the Flux Pattern (Action → Event → Store → View). Produces collocated triads (.yaml + .md + .feature) for each entity.
user_invocable: true
---

# View Spec Author

Author **View Specifications** in YAML following the **Flux Pattern** (Action → Event → Store → View). Each entity gets a **collocated triad**: `.yaml` (spec) + `.md` (wireframe/rationale) + `.feature` (Gherkin tests).

## Entity Types

| Entity | ID Prefix | Purpose | Contains |
|--------|-----------|---------|----------|
| **Wireframe** | `WF-` | Root container, viewport definitions | Pages, theme, responsive breakpoints |
| **Page** | `PG-` | Route-bound screen | Components, route config, guards, meta |
| **Component** | `CMP-` | Reusable UI block | Elements, props, store bindings, visibility |
| **Element** | `ELM-` | Atomic interactive unit | States, actions, validation, a11y |
| **Action** | `ACT-` | User intent trigger | Preconditions, event dispatch, debounce |
| **Event** | `EVT-` | State mutation signal | Payload, target stores, side effects |
| **Store** | `STR-` | State container | Initial state, reducers, selectors, persistence |

## Flux Cycle

```
┌─────────┐    triggers    ┌─────────┐    dispatches    ┌─────────┐
│ Element │───────────────▶│ Action  │────────────────▶│  Event  │
│  (ELM)  │                │  (ACT)  │                  │  (EVT)  │
└─────────┘                └─────────┘                  └────┬────┘
     ▲                                                       │
     │ re-renders                                   updates  │
     │                                                       ▼
┌─────────┐                                          ┌─────────┐
│Component│◀─────────────────────────────────────────│  Store  │
│  (CMP)  │              subscribes                   │  (STR)  │
└─────────┘                                          └─────────┘
```

Every interaction must complete the cycle: **ELM → ACT → EVT → STR → CMP**.

## Collocated Triad Pattern

Every entity produces 3 files side-by-side in the same directory:

```
components/
  search-bar.yaml      # Spec definition (refs, props, structure)
  search-bar.md        # ASCII wireframe, design rationale, flow diagrams
  search-bar.feature   # Gherkin scenarios covering all testable behaviors
```

The YAML references both companions:
```yaml
docs: ./search-bar.md
tests: ./search-bar.feature
```

## Workflow

1. **Identify the screen** — Name the wireframe, define viewports (desktop/tablet/mobile).
2. **Decompose into pages** — One page per route. Define route patterns, guards, meta.
3. **Extract components** — Identify reusable UI blocks. Define props, store bindings, visibility.
4. **Atomize elements** — Break components into elements. Define states, actions, validation, a11y.
5. **Map actions** — For each interactive element, create an action. Define preconditions, event dispatch.
6. **Define events** — For each action, create dispatched events. Define payload, target stores, side effects.
7. **Design stores** — For each event target, create or update a store. Define reducers, selectors, persistence.
8. **Write companion docs** — For each entity, create the `.md` with ASCII wireframe and rationale.
9. **Write Gherkin tests** — For each entity, create the `.feature` covering all behaviors.
10. **Verify** — Run `verify.sh` to validate all cross-references. All 7 checks must PASS.

## YAML Templates

### Wireframe

```yaml
entity: wireframe
id: WF-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

viewports:
  desktop: { min-width: 1024 }
  tablet: { min-width: 768, max-width: 1023 }
  mobile: { max-width: 767 }

theme:
  colors:
    primary: "<hex>"
    secondary: "<hex>"
    background: "<hex>"
    surface: "<hex>"
    text: "<hex>"
    error: "<hex>"
  typography:
    font-family: "<font>"
    base-size: "<size>"

pages:
  - $ref: PG-<page-name>
```

### Page

```yaml
entity: page
id: PG-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

route:
  path: "/<path>"
  params: []
  query: []
  hash: false
  guard: null

layout: single-column | sidebar | split | full-bleed

meta:
  title: "<title>"
  description: "<desc>"

components:
  - $ref: CMP-<component-name>

stores:
  - $ref: STR-<store-name>
```

### Component

```yaml
entity: component
id: CMP-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

props:
  - name: "<prop>"
    type: string | number | boolean | array | object
    required: true | false
    default: <value>

children:
  - $ref: ELM-<element-name>
  - $ref: CMP-<child-component>

stores:
  - $ref: STR-<store-name>
    binds:
      - selector: "<selector-name>"
        to: "<prop-name>"

visibility:
  condition: "<expression>"
  responsive:
    desktop: true
    tablet: true
    mobile: true
```

### Element

```yaml
entity: element
id: ELM-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

type: button | input | text | image | icon | link | container | list | form

states:
  default: { <style-props> }
  hover: { <style-props> }
  active: { <style-props> }
  disabled: { <style-props> }
  error: { <style-props> }
  loading: { <style-props> }
  selected: { <style-props> }
  focused: { <style-props> }

actions:
  - trigger: click | submit | change | focus | blur | keydown | hover | scroll
    action: { $ref: ACT-<action-name> }

store-binding:
  store: { $ref: STR-<store-name> }
  field: "<field>"

validation:
  - rule: required | min-length | max-length | pattern | custom
    value: <value>
    message: "<error message>"

accessibility:
  role: "<aria-role>"
  label: "<aria-label>"
  keyboard:
    - key: Enter | Space | Escape | Tab
      action: { $ref: ACT-<action-name> }
```

### Action

```yaml
entity: action
id: ACT-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

type: <action-type>   # See references/actions-catalog.md

trigger:
  element: { $ref: ELM-<element-name> }
  interaction: click | submit | change | focus | blur | keydown | hover | scroll

preconditions:
  - condition: "<expression>"
    fail-action: block | warn | redirect

events-dispatched:
  - $ref: EVT-<event-name>

debounce: null | { wait: <ms>, leading: false, trailing: true }
throttle: null | { wait: <ms> }
optimistic: null | { rollback-event: EVT-<name> }
```

### Event

```yaml
entity: event
id: EVT-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

payload:
  - name: "<field>"
    type: string | number | boolean | array | object
    required: true | false

target-stores:
  - $ref: STR-<store-name>

side-effects:
  - type: api-call | navigation | notification | analytics | local-storage
    config: { <effect-specific-config> }

on-success: null | { dispatch: EVT-<name> }
on-failure: null | { dispatch: EVT-<name> }
```

### Store

```yaml
entity: store
id: STR-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

initial-state:
  <field>: <default-value>

reducers:
  <event-id>:
    - field: "<field>"
      operation: set | append | remove | increment | decrement | toggle | clear
      value: "<expression>"

selectors:
  - name: "<selector>"
    compute: "<expression>"

consumers:
  - $ref: CMP-<component-name>
  - $ref: ELM-<element-name>

persistence:
  enabled: true | false
  storage: local-storage | session-storage | indexeddb
  key: "<storage-key>"
```

## Gherkin Template

```gherkin
@<ENTITY-ID>
Feature: <Entity Name>
  As a <role>
  I want <goal>
  So that <benefit>

  Background:
    Given the <entity-type> "<entity-name>" is initialized

  Scenario: <Behavior description>
    Given <precondition>
    When <action>
    Then <expected outcome>
```

## Verify Script

Run from the spec root directory:

```bash
./scripts/verify.sh
```

The script performs 7 checks:

| # | Check | Description |
|---|-------|-------------|
| 1 | ID Uniqueness | All `id:` values across YAML files are globally unique |
| 2 | Ref Resolution | Every `$ref: XX-*` points to an existing entity ID |
| 3 | No Orphans | Every entity (except wireframe root) is referenced by at least one other |
| 4 | Flux Cycle Complete | Actions→events-dispatched, events→target-stores, elements→actions |
| 5 | Docs Existence | All `docs:` paths resolve to existing `.md` files |
| 6 | Tests Existence | All `tests:` paths resolve to existing `.feature` files |
| 7 | Index Completeness | `index.yaml` references all entity YAML files |

Output: Markdown table, exit code 0 (all pass) or 1 (any fail).

## Quick Reference

- [Full YAML Schema](./references/schema.md) — All 7 entity schemas with field descriptions
- [Actions Catalog](./references/actions-catalog.md) — 57 action types by category
- [Flux Pattern](./references/flux-pattern.md) — Cycle diagram, rules, enforcement
- [Verify Template](./assets/verify-template.sh) — Bash verification script
- [Complete Example](./examples/) — Search App wireframe with 14 entities
