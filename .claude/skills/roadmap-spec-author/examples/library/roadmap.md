# @hex-di/validator — Roadmap

**Variant:** library
**Status:** Active
**Last Updated:** 2025-02-15

## Document Control

| Field | Value |
|-------|-------|
| Document ID | RM-validator |
| Revision | 3 |
| Last Updated | 2025-02-15 |
| Change Control | CCR-087 |

## Goal

Deliver a composable, type-safe validation library with schema inference, async validation support, and integration adapters for form libraries.

---

## 1. Schema Primitives

**Status:** Delivered

### Scope

Core primitive validators (string, number, boolean, date) with chainable refinements and type inference from schema definitions.

### Deliverables

| # | Deliverable | Spec Section | Package | Risk | Status |
|---|-------------|-------------|---------|------|--------|
| WI-FT-1-1 | Primitive validators | §1–§8 | `@hex-di/validator` | — | Delivered |
| WI-FT-1-2 | Refinement chains | §9–§14 | `@hex-di/validator` | — | Delivered |
| WI-FT-1-3 | Type inference | §15–§18 | `@hex-di/validator` | — | Delivered |

### Exit Criteria

- [x] EC-FT-1-1: All 6 primitive types validate correctly with edge cases
- [x] EC-FT-1-2: Refinements compose without type widening
- [x] EC-FT-1-3: `Infer<typeof schema>` produces exact TypeScript types

---

## 2. Composition & Async

**Status:** In Progress

### Scope

Object/array/tuple schema composition, union/intersection types, and async validation for server-side checks (uniqueness, external API validation).

### Deliverables

| # | Deliverable | Spec Section | Package | Risk | Status |
|---|-------------|-------------|---------|------|--------|
| WI-FT-2-1 | Object schemas | §19–§28 | `@hex-di/validator` | — | Delivered |
| WI-FT-2-2 | Array/tuple schemas | §29–§35 | `@hex-di/validator` | — | Delivered |
| WI-FT-2-3 | Union/intersection | §36–§42 | `@hex-di/validator` | Type inference complexity | In Progress |
| WI-FT-2-4 | Async validation | §43–§51 | `@hex-di/validator` | Cancellation semantics | Planned |

### Exit Criteria

- [x] EC-FT-2-1: Nested object schemas validate 5+ levels deep
- [x] EC-FT-2-2: Array schemas support min/max length and per-item validation
- [ ] EC-FT-2-3: Union discriminated by literal field infers correct branch type
- [ ] EC-FT-2-4: Async validators support AbortSignal cancellation

---

## 3. Integration Adapters

**Status:** Planned

### Scope

Adapter ports for React Hook Form, Formik, and plain DOM forms. Each adapter translates the validation schema into the target library's format.

### Deliverables

| # | Deliverable | Spec Section | Package | Risk | Status |
|---|-------------|-------------|---------|------|--------|
| WI-FT-3-1 | Adapter port definition | §52–§55 | `@hex-di/validator` | — | Planned |
| WI-FT-3-2 | React Hook Form adapter | §56–§62 | `@hex-di/validator-rhf` | RHF API changes in v8 | Planned |
| WI-FT-3-3 | Formik adapter | §63–§68 | `@hex-di/validator-formik` | Formik maintenance status | Planned |
| WI-FT-3-4 | Plain DOM adapter | §69–§74 | `@hex-di/validator` | — | Planned |

### Exit Criteria

- [ ] EC-FT-3-1: Adapter port compiles with strict mode, no runtime deps on form libs
- [ ] EC-FT-3-2: RHF adapter passes all validation scenarios from §56–§62
- [ ] EC-FT-3-3: Formik adapter handles async validation with loading states
- [ ] EC-FT-3-4: DOM adapter works without any framework dependency

---

## Status Summary

| # | Feature | Status | Items | Delivered | Remaining |
|---|---------|--------|-------|-----------|-----------|
| FT-1 | Schema Primitives | Delivered | 3 | 3 | 0 |
| FT-2 | Composition & Async | In Progress | 4 | 2 | 2 |
| FT-3 | Integration Adapters | Planned | 4 | 0 | 4 |

---

## Version History

| Revision | Date | Change | CCR |
|----------|------|--------|-----|
| 1 | 2025-01-10 | Initial roadmap with 3 features | CCR-071 |
| 2 | 2025-02-01 | FT-1 marked Delivered, FT-2 items updated | CCR-079 |
| 3 | 2025-02-15 | Added FT-3 adapter details, updated status summary | CCR-087 |
