# View Spec YAML Schema

Complete schema for all 7 entity types. Every entity includes `docs` and `tests` fields for the collocated triad pattern.

## Common Fields (All Entities)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `entity` | string | yes | Entity type: `wireframe`, `page`, `component`, `element`, `action`, `event`, `store` |
| `id` | string | yes | Unique identifier with type prefix (`WF-`, `PG-`, `CMP-`, `ELM-`, `ACT-`, `EVT-`, `STR-`) |
| `name` | string | yes | Human-readable display name |
| `docs` | string | yes | Relative path to companion `.md` file |
| `tests` | string | yes | Relative path to companion `.feature` file |

## 1. Wireframe (`WF-`)

Root container for the entire specification. Defines viewports, theme, and page list.

```yaml
entity: wireframe
id: WF-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

viewports:
  desktop:
    min-width: <number>
  tablet:
    min-width: <number>
    max-width: <number>
  mobile:
    max-width: <number>

theme:
  colors:
    primary: "<hex>"
    secondary: "<hex>"
    background: "<hex>"
    surface: "<hex>"
    text: "<hex>"
    error: "<hex>"
  typography:
    font-family: "<font-stack>"
    base-size: "<size with unit>"

pages:
  - $ref: PG-<page-name>
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `viewports` | object | yes | Responsive breakpoint definitions |
| `viewports.desktop` | object | yes | Desktop breakpoint (`min-width`) |
| `viewports.tablet` | object | yes | Tablet breakpoint (`min-width`, `max-width`) |
| `viewports.mobile` | object | yes | Mobile breakpoint (`max-width`) |
| `theme` | object | yes | Global design tokens |
| `theme.colors` | object | yes | Color palette (primary, secondary, background, surface, text, error) |
| `theme.typography` | object | yes | Font configuration (font-family, base-size) |
| `pages` | array of $ref | yes | References to page entities |

## 2. Page (`PG-`)

Route-bound screen containing components. Defines routing, layout, and metadata.

```yaml
entity: page
id: PG-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

route:
  path: "/<url-path>"
  params:
    - name: "<param>"
      type: string | number
      required: true | false
  query:
    - name: "<param>"
      type: string | number
      default: <value>
  hash: true | false
  guard:
    type: auth | role | feature-flag
    config: { <guard-specific> }

layout: single-column | sidebar | split | full-bleed

meta:
  title: "<page title>"
  description: "<meta description>"
  og-image: "<url>"

components:
  - $ref: CMP-<component-name>

stores:
  - $ref: STR-<store-name>
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `route` | object | yes | Routing configuration |
| `route.path` | string | yes | URL path pattern |
| `route.params` | array | no | Path parameters |
| `route.query` | array | no | Query string parameters |
| `route.hash` | boolean | no | Whether hash fragment is used |
| `route.guard` | object | no | Access guard (auth, role, feature-flag) |
| `layout` | string | yes | Page layout type |
| `meta` | object | no | HTML meta tags |
| `components` | array of $ref | yes | References to component entities |
| `stores` | array of $ref | no | Stores this page subscribes to |

## 3. Component (`CMP-`)

Reusable UI block. Contains elements or child components, binds to stores.

```yaml
entity: component
id: CMP-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

props:
  - name: "<prop-name>"
    type: string | number | boolean | array | object
    required: true | false
    default: <value>
    description: "<prop description>"

children:
  - $ref: ELM-<element-name>
  - $ref: CMP-<child-component>

stores:
  - $ref: STR-<store-name>
    binds:
      - selector: "<selector-name>"
        to: "<prop-name>"

visibility:
  condition: "<boolean expression>"
  responsive:
    desktop: true | false
    tablet: true | false
    mobile: true | false
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `props` | array | no | Component input properties |
| `props[].name` | string | yes | Property name |
| `props[].type` | string | yes | Property type |
| `props[].required` | boolean | yes | Whether prop is required |
| `props[].default` | any | no | Default value (when not required) |
| `children` | array of $ref | yes | References to child elements/components |
| `stores` | array | no | Store subscriptions with selector bindings |
| `stores[].binds` | array | no | Selector-to-prop mappings |
| `visibility` | object | no | Conditional rendering rules |
| `visibility.condition` | string | no | Boolean expression for visibility |
| `visibility.responsive` | object | no | Per-viewport visibility flags |

## 4. Element (`ELM-`)

Atomic interactive unit. Defines visual states, actions, store bindings, validation, and accessibility.

```yaml
entity: element
id: ELM-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

type: button | input | text | image | icon | link | container | list | form

states:
  default: { <style-properties> }
  hover: { <style-properties> }
  active: { <style-properties> }
  disabled: { <style-properties> }
  error: { <style-properties> }
  loading: { <style-properties> }
  selected: { <style-properties> }
  focused: { <style-properties> }

actions:
  - trigger: click | submit | change | focus | blur | keydown | hover | scroll
    action: { $ref: ACT-<action-name> }

store-binding:
  store: { $ref: STR-<store-name> }
  field: "<state-field-name>"

validation:
  - rule: required | min-length | max-length | pattern | custom
    value: <rule-value>
    message: "<error message>"

accessibility:
  role: "<aria-role>"
  label: "<aria-label>"
  keyboard:
    - key: Enter | Space | Escape | Tab
      action: { $ref: ACT-<action-name> }
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | HTML element category |
| `states` | object | yes | Visual state definitions (at minimum `default`) |
| `states.default` | object | yes | Default visual state (always required) |
| `states.<state>` | object | no | Other states: hover, active, disabled, error, loading, selected, focused |
| `actions` | array | no | User interaction triggers |
| `actions[].trigger` | string | yes | DOM event type |
| `actions[].action` | $ref | yes | Reference to action entity |
| `store-binding` | object | no | Two-way binding to a store field |
| `validation` | array | no | Input validation rules |
| `validation[].rule` | string | yes | Validation rule type |
| `validation[].value` | any | no | Rule parameter (e.g., min length value) |
| `validation[].message` | string | yes | Error message on failure |
| `accessibility` | object | no | ARIA and keyboard accessibility |
| `accessibility.role` | string | no | ARIA role attribute |
| `accessibility.label` | string | no | ARIA label attribute |
| `accessibility.keyboard` | array | no | Keyboard shortcut bindings |

## 5. Action (`ACT-`)

User intent trigger. Links element interactions to event dispatching.

```yaml
entity: action
id: ACT-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

type: <action-type>

trigger:
  element: { $ref: ELM-<element-name> }
  interaction: click | submit | change | focus | blur | keydown | hover | scroll

preconditions:
  - condition: "<boolean expression>"
    fail-action: block | warn | redirect

events-dispatched:
  - $ref: EVT-<event-name>

debounce:
  wait: <milliseconds>
  leading: true | false
  trailing: true | false

throttle:
  wait: <milliseconds>

optimistic:
  rollback-event: EVT-<name>
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | Action type from actions catalog |
| `trigger` | object | yes | What triggers this action |
| `trigger.element` | $ref | yes | Source element reference |
| `trigger.interaction` | string | yes | DOM event type |
| `preconditions` | array | no | Conditions checked before dispatching |
| `preconditions[].condition` | string | yes | Boolean expression |
| `preconditions[].fail-action` | string | yes | What happens on failure |
| `events-dispatched` | array of $ref | yes | Events fired by this action |
| `debounce` | object | no | Debounce configuration |
| `throttle` | object | no | Throttle configuration |
| `optimistic` | object | no | Optimistic update with rollback |

## 6. Event (`EVT-`)

State mutation signal dispatched by actions. Targets stores and may trigger side effects.

```yaml
entity: event
id: EVT-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

payload:
  - name: "<field-name>"
    type: string | number | boolean | array | object
    required: true | false

target-stores:
  - $ref: STR-<store-name>

side-effects:
  - type: api-call | navigation | notification | analytics | local-storage
    config:
      # api-call:
      method: GET | POST | PUT | DELETE
      url: "<endpoint>"
      # navigation:
      path: "<target-path>"
      # notification:
      message: "<notification text>"
      level: info | success | warning | error
      # analytics:
      event-name: "<tracking event>"
      properties: { <key-value pairs> }
      # local-storage:
      action: set | get | remove
      key: "<storage key>"

on-success:
  dispatch: EVT-<name>
on-failure:
  dispatch: EVT-<name>
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `payload` | array | no | Event data fields |
| `payload[].name` | string | yes | Field name |
| `payload[].type` | string | yes | Field type |
| `payload[].required` | boolean | yes | Whether field is required |
| `target-stores` | array of $ref | yes | Stores that reduce this event |
| `side-effects` | array | no | Non-state effects triggered |
| `side-effects[].type` | string | yes | Side effect category |
| `side-effects[].config` | object | yes | Type-specific configuration |
| `on-success` | object | no | Event to dispatch on success |
| `on-failure` | object | no | Event to dispatch on failure |

## 7. Store (`STR-`)

State container. Holds initial state, reduces events, exposes selectors, notifies consumers.

```yaml
entity: store
id: STR-<name>
name: "<Display Name>"
docs: ./<name>.md
tests: ./<name>.feature

initial-state:
  <field>: <default-value>

reducers:
  <EVT-id>:
    - field: "<state-field>"
      operation: set | append | remove | increment | decrement | toggle | clear
      value: "<expression or payload reference>"

selectors:
  - name: "<selector-name>"
    compute: "<expression referencing state fields>"

consumers:
  - $ref: CMP-<component-name>
  - $ref: ELM-<element-name>

persistence:
  enabled: true | false
  storage: local-storage | session-storage | indexeddb
  key: "<storage-key>"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `initial-state` | object | yes | Default state values |
| `reducers` | object | yes | Event-to-mutation mappings (keyed by event ID) |
| `reducers.<EVT-id>` | array | yes | List of field mutations |
| `reducers.<EVT-id>[].field` | string | yes | State field to mutate |
| `reducers.<EVT-id>[].operation` | string | yes | Mutation operation |
| `reducers.<EVT-id>[].value` | string | no | Value expression (not needed for toggle/clear) |
| `selectors` | array | no | Derived state computations |
| `selectors[].name` | string | yes | Selector name |
| `selectors[].compute` | string | yes | Computation expression |
| `consumers` | array | no | Components/elements that subscribe |
| `persistence` | object | no | State persistence configuration |
| `persistence.enabled` | boolean | yes | Whether persistence is active |
| `persistence.storage` | string | yes | Storage backend |
| `persistence.key` | string | yes | Storage key |
