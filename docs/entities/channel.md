# channel

> **Declared by `@specforge/product`**, one of the builtin extensions. `channel` is one example of an entity kind, not part of
> SpecForge's core: the core grammar knows only `spec`, `ref`, `use`, `define` and `defaults`, and every other kind,
> with its fields, edges and rules, comes from an extension ([extension model](../extension-model.md)). Enable this
> one with `specforge add @specforge/product`. The fields, edges and rules below are read from its declaration.

## Purpose

A `channel` is the medium through which people reach what you offer: a web app, an API, a phone line, a shop
counter. It answers: **"Where does the interaction happen?"**

Journeys name the channels they use. Because channels are entities, the graph can answer "which features reach
this channel?" (`specforge product channel-features`) and flag a channel no journey uses (I047).

## Syntax

```spec
channel web "Web Application" {
  description       "Browser-based web application accessed over HTTPS."
  interaction_model request_response
  url               "https://app.example.com"
  tags              ["digital"]
}
```

## Fields

Every `channel` has an id (the word after `channel`) and a title (the quoted string after the id, required: E076). The fields below are the ones its declaration lists. A field no loaded extension declares is reported (W020).

### Required

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Human-readable summary of this channel. |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `interaction_model` | identifier | How users interact through this channel: one word, for example request_response, event_driven, batch, streaming, bidirectional, manual, in_person, self_service, broadcast. Any word is accepted. |
| `url` | string | URL or endpoint for this channel. |
| `status` | enum: active, deprecated | Lifecycle state; default `active`. |
| `reason` | string | Justification for the current status or a status change. |
| `tags` | string list | Freeform labels for filtering and categorization (a shared field of `@specforge/product`). |

## Relationships

A relation is written once, on the field that owns it; the other side is derived (ADR 0150). For an incoming edge the field is on the other entity.

| Direction | Entity | Edge | Written as | Meaning |
|-----------|--------|------|------------|---------|
| incoming | [journey](journey.md) | `JourneyUsesChannel` | `channels` on `journey` | Communication or distribution channels used in this journey |

## Validation rules

Core checks every entity whatever its kind: references must resolve (E003), ids are unique across the project (E002), required fields are present (E006), each value has its field's type, an enum value being one the field lists (E061), and every block has a title (E076). The rules below are the ones extensions declare for this kind; [diagnostics.md](../diagnostics.md) has every code.

| Code | Level | Declared by | Rule |
|------|-------|-------------|------|
| I047 | info | `@specforge/product` | Unreferenced channel |
| I068 | info | `@specforge/product` | Tag not lowercase-hyphenated |
| I070 | info | `@specforge/product` | Deprecated channel missing reason |


## Design guidance

### Interaction model

| Model | Use when | Example |
|-------|----------|---------|
| `request_response` | The user asks, the system answers | Web form, REST call |
| `event_driven` | The system pushes updates | Push notifications |
| `batch` | Work is submitted and processed later | File upload, scheduled report |
| `streaming` | Data flows continuously | Live dashboard |
| `bidirectional` | Real-time two-way exchange | Chat, collaborative editing |
| `manual` | No digital interaction | Shop counter, paper form |

### Channel vs. environment

A channel is how people interact (`web`, `api`), not where code runs ("production", "staging").

## Examples

### Deprecated

```spec
channel legacy_api "Legacy REST API v1" {
  description "Original REST API, superseded by the v2 endpoint."
  interaction_model request_response
  status deprecated
  reason "Replaced by the v2 API; sunset on 2026-12-31"
}
```

### Outside software

```spec
channel shop_counter "Shop Counter" {
  description "Customers deal with staff face to face; staff record the sale afterwards."
  interaction_model manual
  tags ["physical", "retail"]
}
```
